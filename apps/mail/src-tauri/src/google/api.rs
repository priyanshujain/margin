// Typed wrapper over the Gmail REST API, in the calendar's shape: one function per call the app
// makes, an access token in and a typed response out, and `read_json` taking the body to a String
// before anything tries to deserialise it so a Google error payload survives into the message
// rather than being swallowed by a parse failure on a shape that was never the response type.
//
// Two things live here that are not calls, because every call has to respect them:
//
//   `Call`   the unit cost and the required scope of each method, in one table
//   `Quota`  the rolling minute, which is the only reason a first sync finishes at all
//
// The arithmetic, so nobody has to redo it: the budget is 6,000 units per minute per user,
// `messages.get` is 20 units, and hydration is batched 50 at a time. A full batch is 50 * 20 =
// 1,000 units, six batches fill the minute exactly, and the seventh waits. That is 300 messages a
// minute, which is 67 minutes for a twenty thousand message mailbox and most of a working day for
// a hundred thousand. Everything else the app does is noise against it: polling `history.list`
// every 12 seconds is 10 units a minute.

use std::sync::LazyLock;

use base64::alphabet;
use base64::engine::{DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig};
use base64::Engine;
use serde::{Deserialize, Serialize};

pub use margin_google::{error_for, read_bytes, read_empty, read_json};
pub use margin_http::{
    backoff, backoff_ms, jitter_ms, now_ms, path_segment, query_string, retry_after, url_with,
    with_retry, with_retry_no_replay, HttpError as ApiError, Quota, DEFAULT_RETRY_MS,
    DROPPED_WAITS_MS, MAX_ATTEMPTS, MAX_BACKOFF_MS,
};

pub const BASE: &str = "https://gmail.googleapis.com/gmail/v1";

/// The batch endpoint is not on `gmail.googleapis.com`. The discovery document puts it at
/// `batch/gmail/v1` on `www.googleapis.com`, and the other host answers 404.
pub const BATCH_URL: &str = "https://www.googleapis.com/batch/gmail/v1";

/// Gmail caps a list page at 500 and the ids are cheap, so there is no reason to ask for fewer.
pub const MAX_LIST_RESULTS: &str = "500";

/// One `batchModify` takes up to 1,000 ids for its flat 50 units.
pub const MAX_MODIFY_IDS: usize = 1000;

pub const SCOPE_MODIFY: &str = "https://www.googleapis.com/auth/gmail.modify";
pub const SCOPE_SETTINGS: &str = "https://www.googleapis.com/auth/gmail.settings.basic";

/// The headers hydration asks for, and no others. `format=metadata` without `metadataHeaders`
/// returns every header a message carries, which on a mailing list message is a page of `Received`
/// and DKIM signatures nobody reads: same 20 units, ten times the bytes.
///
/// These are the ones the reader, the threader and the screener actually parse. `List-Id`,
/// `List-Unsubscribe`, `List-Unsubscribe-Post`, `Precedence` and `Auto-Submitted` together are the
/// "not a human" signal the Feed is sorted by.
pub const METADATA_HEADERS: [&str; 17] = [
    "From",
    "To",
    "Cc",
    "Bcc",
    "Reply-To",
    "Subject",
    "Date",
    "Message-ID",
    "In-Reply-To",
    "References",
    "List-Id",
    "List-Unsubscribe",
    "List-Unsubscribe-Post",
    "Precedence",
    "Auto-Submitted",
    "Content-Type",
    // Not read by anything yet. It is the only place SPF, DKIM and DMARC results appear, asking
    // for one more header costs nothing at all, and a Screener card that cannot say whether a
    // first message was actually from who it claims is a Screener card missing the point.
    "Authentication-Results",
];

/// Gmail's `raw` is base64url, not standard base64: getting `+/` and `-_` the wrong way round
/// produces MIME that Gmail rejects with a 400 saying nothing useful. Decoding is padding
/// indifferent because both padded and unpadded bodies arrive.
pub const B64: GeneralPurpose = GeneralPurpose::new(
    &alphabet::URL_SAFE,
    GeneralPurposeConfig::new().with_decode_padding_mode(DecodePaddingMode::Indifferent),
);

/// A client of this layer's own rather than the OAuth stack's. They share no host: auth talks to
/// `oauth2.googleapis.com`, this talks to `gmail.googleapis.com`, `people.googleapis.com` and
/// `www.googleapis.com`, so one pool would not be reused anyway.
pub static HTTP: LazyLock<reqwest::Client> =
    LazyLock::new(|| margin_http::client(margin_http::Profile::Api));

// -- the call table ---------------------------------------------------------------------------

/// Every Gmail method the app calls, with the two facts that belong to the call rather than to the
/// response: what it costs against the minute, and which scope a 403 is complaining about. Google
/// never names the scope in the error body, only the service and the method, so the caller is the
/// only one who knows what to put on the Grant button.
///
/// Costs are Google's table as read on 2026-09-03, after the 1 May 2026 change. Anything quoting
/// `messages.get` at 5 units predates it and is wrong by a factor of four.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Call {
    MessagesList,
    HistoryList,
    MessagesGet,
    AttachmentsGet,
    LabelsList,
    BatchModify,
    MessagesModify,
    MessagesSend,
    DraftsList,
    DraftsCreate,
    DraftsUpdate,
    DraftsDelete,
    SendAsList,
    GetProfile,
}

impl Call {
    pub const fn units(self) -> u32 {
        match self {
            Call::MessagesSend => 100,
            Call::BatchModify => 50,
            // `metadata`, `full`, `raw` and `minimal` all cost the same: there is no cheaper way to
            // read a message, which is why hydration is the whole budget.
            Call::MessagesGet | Call::AttachmentsGet => 20,
            Call::DraftsUpdate => 15,
            Call::DraftsCreate | Call::DraftsDelete => 10,
            Call::MessagesList | Call::MessagesModify | Call::DraftsList => 5,
            Call::HistoryList => 2,
            Call::LabelsList | Call::SendAsList | Call::GetProfile => 1,
        }
    }

    pub const fn scope(self) -> &'static str {
        match self {
            Call::SendAsList => SCOPE_SETTINGS,
            _ => SCOPE_MODIFY,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Call::MessagesList => "Gmail message list",
            Call::HistoryList => "Gmail history",
            Call::MessagesGet => "Gmail message fetch",
            Call::AttachmentsGet => "Gmail attachment fetch",
            Call::LabelsList => "Gmail label list",
            Call::BatchModify => "Gmail label change",
            Call::MessagesModify => "Gmail label change",
            Call::MessagesSend => "Gmail send",
            Call::DraftsList => "Gmail draft list",
            Call::DraftsCreate => "Gmail draft create",
            Call::DraftsUpdate => "Gmail draft update",
            Call::DraftsDelete => "Gmail draft delete",
            Call::SendAsList => "Gmail send-as list",
            Call::GetProfile => "Gmail profile",
        }
    }
}

// -- errors -----------------------------------------------------------------------------------

/// The text with every sentence that carried a link taken out, and Google's wording when nothing
/// is left.
pub fn strip_urls(text: &str) -> String {
    margin_http::strip_urls(text, margin_google::NO_REASON)
}

impl From<ApiError> for crate::provider::ProviderError {
    fn from(e: ApiError) -> crate::provider::ProviderError {
        use crate::provider::ProviderError as P;
        match e {
            ApiError::Unauthorized(m) => P::Auth(m),
            ApiError::InsufficientScope(s) => P::Scope(s),
            ApiError::RateLimited { retry_after_ms } => P::RateLimited { retry_after_ms },
            ApiError::NotFound(_) => P::NotFound,
            ApiError::Offline(m) | ApiError::Dropped(m) => P::Network(m),
            ApiError::Other(m) => P::Other(m),
            other @ (ApiError::Gone | ApiError::Conflict) => P::Other(other.to_string()),
        }
    }
}

// -- quota ------------------------------------------------------------------------------------

/// Per user, per project, per minute. Google says it cannot be raised for any reason.
pub const BUDGET_PER_MINUTE: u32 = 6_000;

/// The window the budget is measured over.
pub const WINDOW_MS: u64 = 60_000;

/// How many messages one batch hydrates. Google allows 100 per batch and recommends no more than
/// 50, and 50 * 20 units is exactly a sixth of the minute, which makes the pacing legible.
pub const BATCH_SIZE: usize = 50;

/// One rolling minute per account, because the limit is per user and every part of the app that
/// touches one mailbox shares it: the poll loop, hydration and a triage keystroke all spend from
/// the same 6,000.
static LEDGER: LazyLock<margin_http::Ledger> =
    LazyLock::new(|| margin_http::Ledger::new(BUDGET_PER_MINUTE, WINDOW_MS));

/// Reserves `units` against the account's minute, sleeping until they fit.
pub async fn spend(account_id: &str, units: u32) {
    LEDGER.spend(account_id, units).await
}

/// What is left of this account's minute, for the account chip.
pub fn remaining(account_id: &str) -> u32 {
    LEDGER.remaining(account_id)
}

// -- queries ----------------------------------------------------------------------------------

/// The storage window as Gmail sees it. `after:` takes unix seconds, and a bare number is the only
/// form that is unambiguous: the date forms are interpreted in the user's timezone.
pub fn window_query(after_ms: i64) -> String {
    format!("after:{}", after_ms.max(0) / 1000)
}

/// The app's operators (`from:`, `to:`, `subject:`, `has:attachment`, `filename:`, `in:`,
/// `before:`, `after:`, `label:`) are Gmail's own, so a user's query crosses unchanged and the only
/// work is joining it to a window term when there is one. Nothing is quoted or escaped here on
/// purpose: rewriting a search query is how a search silently stops matching what the user typed.
pub fn search_query(user_query: &str, after_ms: Option<i64>) -> String {
    let user_query = user_query.trim();
    match (user_query.is_empty(), after_ms) {
        (true, None) => String::new(),
        (true, Some(after)) => window_query(after),
        (false, None) => user_query.to_string(),
        (false, Some(after)) => format!("{user_query} {}", window_query(after)),
    }
}

/// The path a metadata fetch uses, as a batch part needs it: absolute, with the query string
/// attached. The one definition of the hydration request, shared by the batched and the single
/// form so they cannot drift apart.
pub fn metadata_path(message_id: &str) -> String {
    let mut path = format!(
        "/gmail/v1/users/me/messages/{}?format=metadata",
        path_segment(message_id)
    );
    for header in METADATA_HEADERS {
        path.push_str("&metadataHeaders=");
        path.push_str(header);
    }
    path
}

// -- response shapes ----------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageId {
    pub id: String,
    #[serde(default)]
    pub thread_id: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageIdsPage {
    #[serde(default)]
    pub messages: Vec<MessageId>,
    #[serde(default)]
    pub next_page_token: Option<String>,
    /// Gmail's own word for it is an estimate, and it can be wildly wrong. A hint for the progress
    /// bar, never a denominator.
    #[serde(default)]
    pub result_size_estimate: Option<u32>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Header {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub value: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Payload {
    #[serde(default)]
    pub headers: Vec<Header>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub thread_id: String,
    #[serde(default)]
    pub label_ids: Vec<String>,
    #[serde(default)]
    pub snippet: String,
    /// Epoch milliseconds, as a string, because JSON has no int64. The time Gmail received the
    /// message, not the `Date` header, which is the sender's claim and can be hours out.
    #[serde(default)]
    pub internal_date: String,
    #[serde(default)]
    pub size_estimate: u32,
    #[serde(default)]
    pub history_id: String,
    #[serde(default)]
    pub payload: Option<Payload>,
    /// Only with `format=raw`.
    #[serde(default)]
    pub raw: Option<String>,
}

impl Message {
    pub fn internal_date_ms(&self) -> i64 {
        self.internal_date.parse().unwrap_or(0)
    }

    pub fn header_pairs(&self) -> Vec<(String, String)> {
        self.payload
            .as_ref()
            .map(|p| {
                p.headers
                    .iter()
                    .map(|h| (h.name.clone(), h.value.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryMessage {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub thread_id: String,
    /// The message's whole label set as it stands after the change, not the delta.
    #[serde(default)]
    pub label_ids: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryMessageRef {
    #[serde(default)]
    pub message: HistoryMessage,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryLabelChange {
    #[serde(default)]
    pub message: HistoryMessage,
    #[serde(default)]
    pub label_ids: Vec<String>,
}

/// Google recommends the specific change-type fields over the `messages` bag, so `messages` is not
/// deserialised at all.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryRecord {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub messages_added: Vec<HistoryMessageRef>,
    /// Permanently deleted, not trashed. Trashing arrives as a `TRASH` label add.
    #[serde(default)]
    pub messages_deleted: Vec<HistoryMessageRef>,
    #[serde(default)]
    pub labels_added: Vec<HistoryLabelChange>,
    #[serde(default)]
    pub labels_removed: Vec<HistoryLabelChange>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryPage {
    #[serde(default)]
    pub history: Vec<HistoryRecord>,
    #[serde(default)]
    pub next_page_token: Option<String>,
    /// The cursor to store, once the last page has been applied.
    #[serde(default)]
    pub history_id: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Label {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    /// `system` or `user`. Names change and system labels are their own ids, so the id is the key.
    #[serde(default, rename = "type")]
    pub kind: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LabelsPage {
    #[serde(default)]
    pub labels: Vec<Label>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Draft {
    #[serde(default)]
    pub id: String,
    /// The draft id is stable across updates; the inner message id is not, so nothing stores it.
    #[serde(default)]
    pub message: Option<MessageId>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SendAs {
    #[serde(default)]
    pub send_as_email: String,
    #[serde(default)]
    pub display_name: String,
    /// HTML, and Gmail sanitises it before storing it.
    #[serde(default)]
    pub signature: String,
    #[serde(default)]
    pub is_primary: bool,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default)]
    pub verification_status: String,
    #[serde(default)]
    pub reply_to_address: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SendAsPage {
    #[serde(default)]
    pub send_as: Vec<SendAs>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    #[serde(default)]
    pub email_address: String,
    #[serde(default)]
    pub messages_total: u32,
    #[serde(default)]
    pub threads_total: u32,
    #[serde(default)]
    pub history_id: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentBody {
    #[serde(default)]
    pub attachment_id: String,
    #[serde(default)]
    pub size: u32,
    #[serde(default)]
    pub data: String,
}

// -- calls ------------------------------------------------------------------------------------

/// Ids and thread ids, newest first, 500 a page. `includeSpamTrash` is on because the mirror holds
/// what the mailbox holds; the places decide what to show.
pub async fn messages_list(
    access_token: &str,
    query: Option<&str>,
    page_token: Option<&str>,
) -> Result<MessageIdsPage, ApiError> {
    let call = Call::MessagesList;
    let mut params = vec![
        ("maxResults", MAX_LIST_RESULTS),
        ("includeSpamTrash", "true"),
    ];
    if let Some(q) = query.filter(|q| !q.is_empty()) {
        params.push(("q", q));
    }
    if let Some(token) = page_token {
        params.push(("pageToken", token));
    }
    let resp = HTTP
        .get(url_with(&format!("{BASE}/users/me/messages"), &params))
        .bearer_auth(access_token)
        .send()
        .await?;
    read_json(resp, call.name(), call.scope()).await
}

/// The change log. Never combined with `q`: history is mailbox-wide and has no query parameter,
/// which is why every screening decision is made locally.
pub async fn history_list(
    access_token: &str,
    start_history_id: &str,
    page_token: Option<&str>,
) -> Result<HistoryPage, ApiError> {
    let call = Call::HistoryList;
    let mut params = vec![
        ("startHistoryId", start_history_id),
        ("maxResults", MAX_LIST_RESULTS),
    ];
    if let Some(token) = page_token {
        params.push(("pageToken", token));
    }
    let resp = HTTP
        .get(url_with(&format!("{BASE}/users/me/history"), &params))
        .bearer_auth(access_token)
        .send()
        .await?;
    read_json(resp, call.name(), call.scope()).await
}

/// One message's metadata. The batched form in `batch.rs` is what a sync uses; this is for the one
/// that got dropped from a batch and for anything that wants a single message.
pub async fn messages_get_metadata(
    access_token: &str,
    message_id: &str,
) -> Result<Message, ApiError> {
    let call = Call::MessagesGet;
    let mut params: Vec<(&str, &str)> = vec![("format", "metadata")];
    for header in METADATA_HEADERS {
        params.push(("metadataHeaders", header));
    }
    let resp = HTTP
        .get(url_with(
            &format!("{BASE}/users/me/messages/{}", path_segment(message_id)),
            &params,
        ))
        .bearer_auth(access_token)
        .send()
        .await?;
    read_json(resp, call.name(), call.scope()).await
}

/// The whole RFC 2822 message. Bodies are always parsed from these rather than from Gmail's
/// pre-parsed `payload`, because encoded words and legacy charsets arrive daily and a real MIME
/// parser is the only thing that reads them.
pub async fn messages_get_raw(access_token: &str, message_id: &str) -> Result<Vec<u8>, ApiError> {
    let call = Call::MessagesGet;
    let resp = HTTP
        .get(url_with(
            &format!("{BASE}/users/me/messages/{}", path_segment(message_id)),
            &[("format", "raw")],
        ))
        .bearer_auth(access_token)
        .send()
        .await?;
    let message: Message = read_json(resp, call.name(), call.scope()).await?;
    let raw = message
        .raw
        .ok_or_else(|| ApiError::Other("Gmail returned a message with no raw body".into()))?;
    B64.decode(raw.as_bytes())
        .map_err(|e| ApiError::Other(format!("Gmail returned an undecodable raw body: {e}")))
}

/// Attachment ids are reported to change between `messages.get` calls, so callers store the part
/// path and re-read the id rather than keeping it.
pub async fn attachment_get(
    access_token: &str,
    message_id: &str,
    attachment_id: &str,
) -> Result<Vec<u8>, ApiError> {
    let call = Call::AttachmentsGet;
    let resp = HTTP
        .get(format!(
            "{BASE}/users/me/messages/{}/attachments/{}",
            path_segment(message_id),
            path_segment(attachment_id)
        ))
        .bearer_auth(access_token)
        .send()
        .await?;
    let body: AttachmentBody = read_json(resp, call.name(), call.scope()).await?;
    B64.decode(body.data.as_bytes())
        .map_err(|e| ApiError::Other(format!("Gmail returned an undecodable attachment: {e}")))
}

pub async fn labels_list(access_token: &str) -> Result<Vec<Label>, ApiError> {
    let call = Call::LabelsList;
    let resp = HTTP
        .get(format!("{BASE}/users/me/labels"))
        .bearer_auth(access_token)
        .send()
        .await?;
    let page: LabelsPage = read_json(resp, call.name(), call.scope()).await?;
    Ok(page.labels)
}

/// Up to 1,000 ids for a flat 50 units, and an empty body on success. There is no per-id error
/// report, so an id that has since been deleted takes the whole call down with a 400; the caller
/// keeps its batches to what it has just seen in history.
pub async fn messages_batch_modify(
    access_token: &str,
    ids: &[String],
    add: &[String],
    remove: &[String],
) -> Result<(), ApiError> {
    let call = Call::BatchModify;
    let body = serde_json::json!({
        "ids": ids,
        "addLabelIds": add,
        "removeLabelIds": remove,
    });
    let resp = HTTP
        .post(format!("{BASE}/users/me/messages/batchModify"))
        .bearer_auth(access_token)
        .json(&body)
        .send()
        .await?;
    read_empty(resp, call.name(), call.scope()).await
}

/// One message, 5 units against `batchModify`'s 50. Worth it for a single keystroke.
pub async fn messages_modify(
    access_token: &str,
    message_id: &str,
    add: &[String],
    remove: &[String],
) -> Result<Message, ApiError> {
    let call = Call::MessagesModify;
    let body = serde_json::json!({
        "addLabelIds": add,
        "removeLabelIds": remove,
    });
    let resp = HTTP
        .post(format!(
            "{BASE}/users/me/messages/{}/modify",
            path_segment(message_id)
        ))
        .bearer_auth(access_token)
        .json(&body)
        .send()
        .await?;
    read_json(resp, call.name(), call.scope()).await
}

/// `thread_id` is one of the three things a reply needs to thread; the other two are the
/// `In-Reply-To` and `References` headers, which the composer put in `raw`.
pub async fn messages_send(
    access_token: &str,
    raw: &[u8],
    thread_id: Option<&str>,
) -> Result<MessageId, ApiError> {
    let call = Call::MessagesSend;
    let mut body = serde_json::json!({ "raw": B64.encode(raw) });
    if let Some(thread) = thread_id {
        body["threadId"] = serde_json::Value::String(thread.to_string());
    }
    let resp = HTTP
        .post(format!("{BASE}/users/me/messages/send"))
        .bearer_auth(access_token)
        .json(&body)
        .send()
        .await?;
    read_json(resp, call.name(), call.scope()).await
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftsPage {
    #[serde(default)]
    pub drafts: Vec<Draft>,
    pub next_page_token: Option<String>,
}

pub async fn drafts_list(
    access_token: &str,
    page_token: Option<&str>,
) -> Result<DraftsPage, ApiError> {
    let mut params = vec![("maxResults", MAX_LIST_RESULTS)];
    if let Some(token) = page_token {
        params.push(("pageToken", token));
    }
    let resp = HTTP
        .get(url_with(&format!("{BASE}/users/me/drafts"), &params))
        .bearer_auth(access_token)
        .send()
        .await?;
    read_json(resp, Call::DraftsList.name(), Call::DraftsList.scope()).await
}

pub async fn drafts_create(
    access_token: &str,
    raw: &[u8],
    thread_id: Option<&str>,
) -> Result<Draft, ApiError> {
    let call = Call::DraftsCreate;
    let resp = HTTP
        .post(format!("{BASE}/users/me/drafts"))
        .bearer_auth(access_token)
        .json(&draft_body(raw, thread_id))
        .send()
        .await?;
    read_json(resp, call.name(), call.scope()).await
}

pub async fn drafts_update(
    access_token: &str,
    draft_id: &str,
    raw: &[u8],
    thread_id: Option<&str>,
) -> Result<Draft, ApiError> {
    let call = Call::DraftsUpdate;
    let resp = HTTP
        .put(format!("{BASE}/users/me/drafts/{}", path_segment(draft_id)))
        .bearer_auth(access_token)
        .json(&draft_body(raw, thread_id))
        .send()
        .await?;
    read_json(resp, call.name(), call.scope()).await
}

pub async fn drafts_delete(access_token: &str, draft_id: &str) -> Result<(), ApiError> {
    let call = Call::DraftsDelete;
    let resp = HTTP
        .delete(format!("{BASE}/users/me/drafts/{}", path_segment(draft_id)))
        .bearer_auth(access_token)
        .send()
        .await?;
    read_empty(resp, call.name(), call.scope()).await
}

/// A reply draft's `threadId` goes inside `message`, not beside it, which is the one place the
/// draft and send bodies differ.
fn draft_body(raw: &[u8], thread_id: Option<&str>) -> serde_json::Value {
    let mut message = serde_json::json!({ "raw": B64.encode(raw) });
    if let Some(thread) = thread_id {
        message["threadId"] = serde_json::Value::String(thread.to_string());
    }
    serde_json::json!({ "message": message })
}

/// The verified send-as addresses and the primary signature. Everything else on this endpoint is
/// service-account territory, so this is read only.
pub async fn send_as_list(access_token: &str) -> Result<Vec<SendAs>, ApiError> {
    let call = Call::SendAsList;
    let resp = HTTP
        .get(format!("{BASE}/users/me/settings/sendAs"))
        .bearer_auth(access_token)
        .send()
        .await?;
    let page: SendAsPage = read_json(resp, call.name(), call.scope()).await?;
    Ok(page.send_as)
}

/// One unit, and it carries the `historyId` that a first sync records before it lists anything, so
/// the first partial sync covers everything that changed during the crawl.
pub async fn get_profile(access_token: &str) -> Result<Profile, ApiError> {
    let call = Call::GetProfile;
    let resp = HTTP
        .get(format!("{BASE}/users/me/profile"))
        .bearer_auth(access_token)
        .send()
        .await?;
    read_json(resp, call.name(), call.scope()).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::ProviderError;

    /// Google's 404 from `history.list` once the cursor has aged out. Routine, not a disaster.
    const HISTORY_GONE: &str = r#"{"error":{"code":404,
        "message":"Requested entity was not found.",
        "errors":[{"message":"Requested entity was not found.","domain":"global","reason":"notFound"}],
        "status":"NOT_FOUND"}}"#;

    /// A scope the user cleared on the consent screen. Note that the body names the service and the
    /// method and never the scope, which is why `error_for` is told what the call needed.
    const INSUFFICIENT_SCOPE: &str = r#"{"error":{"code":403,
        "message":"Request had insufficient authentication scopes.",
        "errors":[{"message":"Insufficient Permission","domain":"global","reason":"insufficientPermissions"}],
        "status":"PERMISSION_DENIED",
        "details":[{"@type":"type.googleapis.com/google.rpc.ErrorInfo","reason":"ACCESS_TOKEN_SCOPE_INSUFFICIENT","domain":"googleapis.com","metadata":{"method":"google.gmail.v1.GmailService.ListMessages","service":"gmail.googleapis.com"}}]}}"#;

    const RATE_LIMIT_429: &str = r#"{"error":{"code":429,
        "message":"User-rate limit exceeded.  Retry after 2026-09-03T12:00:04.000Z",
        "errors":[{"message":"User-rate limit exceeded.  Retry after 2026-09-03T12:00:04.000Z","domain":"usageLimits","reason":"rateLimitExceeded"}],
        "status":"RESOURCE_EXHAUSTED"}}"#;

    /// The 403 flavour of the same thing, which older Gmail projects still get.
    const RATE_LIMIT_403: &str = r#"{"error":{"code":403,
        "message":"User Rate Limit Exceeded",
        "errors":[{"message":"User Rate Limit Exceeded","domain":"usageLimits","reason":"userRateLimitExceeded"}],
        "status":"PERMISSION_DENIED"}}"#;

    const INVALID_CREDENTIALS: &str = r#"{"error":{"code":401,
        "message":"Request had invalid authentication credentials. Expected OAuth 2 access token, login cookie or other valid authentication credential.",
        "errors":[{"message":"Invalid Credentials","domain":"global","reason":"authError","location":"Authorization","locationType":"header"}],
        "status":"UNAUTHENTICATED"}}"#;

    /// A project that has run out of day. Retrying this in thirty seconds only spends tomorrow's.
    const DAILY_LIMIT: &str = r#"{"error":{"code":403,
        "message":"Daily Limit Exceeded",
        "errors":[{"message":"Daily Limit Exceeded","domain":"usageLimits","reason":"dailyLimitExceeded"}],
        "status":"PERMISSION_DENIED"}}"#;

    fn mapped(status: u16, body: &str) -> ProviderError {
        error_for(status, "Gmail history", SCOPE_MODIFY, None, body).into()
    }

    #[test]
    fn a_404_is_a_not_found_that_history_reads_as_an_expired_cursor() {
        assert!(matches!(
            error_for(404, "Gmail history", SCOPE_MODIFY, None, HISTORY_GONE),
            ApiError::NotFound(_)
        ));
        assert_eq!(mapped(404, HISTORY_GONE), ProviderError::NotFound);
    }

    #[test]
    fn a_403_naming_an_insufficient_scope_carries_the_scope_the_call_needed() {
        let error = error_for(
            403,
            "Gmail message list",
            SCOPE_MODIFY,
            None,
            INSUFFICIENT_SCOPE,
        );
        assert_eq!(error, ApiError::InsufficientScope(SCOPE_MODIFY.to_string()));
        assert_eq!(
            ProviderError::from(error),
            ProviderError::Scope(SCOPE_MODIFY.to_string())
        );
    }

    #[test]
    fn a_settings_call_names_the_settings_scope_rather_than_modify() {
        let error = error_for(
            403,
            Call::SendAsList.name(),
            Call::SendAsList.scope(),
            None,
            INSUFFICIENT_SCOPE,
        );
        assert_eq!(
            error,
            ApiError::InsufficientScope(SCOPE_SETTINGS.to_string())
        );
    }

    #[test]
    fn both_rate_limit_answers_are_rate_limited_and_carry_a_delay() {
        assert_eq!(
            mapped(429, RATE_LIMIT_429),
            ProviderError::RateLimited {
                retry_after_ms: DEFAULT_RETRY_MS
            }
        );
        assert_eq!(
            mapped(403, RATE_LIMIT_403),
            ProviderError::RateLimited {
                retry_after_ms: DEFAULT_RETRY_MS
            }
        );
        assert_eq!(
            error_for(429, "Gmail history", SCOPE_MODIFY, Some(8_000), RATE_LIMIT_429),
            ApiError::RateLimited {
                retry_after_ms: 8_000
            }
        );
    }

    #[test]
    fn a_401_is_a_signed_out_account_and_keeps_googles_sentence() {
        let ProviderError::Auth(message) = mapped(401, INVALID_CREDENTIALS) else {
            panic!("a 401 is an Auth");
        };
        assert!(message.starts_with("Request had invalid authentication credentials."));
        assert!(!message.contains("UNAUTHENTICATED"));
    }

    #[test]
    fn a_daily_limit_is_not_retried_as_a_rate_limit() {
        assert!(matches!(
            error_for(403, "Gmail send", SCOPE_MODIFY, None, DAILY_LIMIT),
            ApiError::Other(_)
        ));
    }

    /// The Gmail API switched off in the project behind the credentials file. Google's sentence
    /// is a console link, and a console link on a toast is a URL and not a reason.
    const API_DISABLED: &str = r#"{"error":{"code":403,
        "message":"Gmail API has not been used in project 123456 before or it is disabled. Enable it by visiting https://console.developers.google.com/apis/api/gmail.googleapis.com/overview?project=123456 then retry. If you enabled this API recently, wait a few minutes for the action to propagate to our systems and retry.",
        "errors":[{"message":"Gmail API has not been used in project 123456 before or it is disabled.","domain":"usageLimits","reason":"accessNotConfigured","extendedHelp":"https://console.developers.google.com"}],
        "status":"PERMISSION_DENIED"}}"#;

    /// The 401 as Google actually sends it, with the "See https://..." tail.
    const INVALID_CREDENTIALS_WITH_LINK: &str = r#"{"error":{"code":401,
        "message":"Request had invalid authentication credentials. Expected OAuth 2 access token, login cookie or other valid authentication credential. See https://developers.google.com/identity/sign-in/web/devconsole-project.",
        "status":"UNAUTHENTICATED"}}"#;

    #[test]
    fn no_error_ever_carries_a_link() {
        let disabled = error_for(403, "Gmail history", SCOPE_MODIFY, None, API_DISABLED);
        let ApiError::Other(said) = &disabled else {
            panic!("a switched off API is an Other, got {disabled:?}");
        };
        assert!(!said.contains("http"), "{said}");
        assert!(said.contains("switched off"), "{said}");

        let ProviderError::Auth(said) = mapped(401, INVALID_CREDENTIALS_WITH_LINK) else {
            panic!("a 401 is an Auth");
        };
        assert!(!said.contains("http"), "{said}");
        assert!(said.starts_with("Request had invalid authentication credentials."), "{said}");
        assert!(said.ends_with("valid authentication credential."), "{said}");

        let ApiError::Other(said) = error_for(
            400,
            "Gmail label change",
            SCOPE_MODIFY,
            None,
            r#"{"error":{"message":"Invalid id value","status":"INVALID_ARGUMENT"}}"#,
        ) else {
            panic!("a 400 is an Other");
        };
        assert_eq!(said, "Gmail label change failed (400): Invalid id value");
    }

    #[test]
    fn a_link_goes_with_its_sentence_and_a_message_that_was_only_a_link_says_so() {
        assert_eq!(
            strip_urls("Enable it by visiting https://console.developers.google.com/x then retry. Then wait."),
            "Then wait."
        );
        assert_eq!(strip_urls("Invalid id value"), "Invalid id value");
        assert_eq!(
            strip_urls("See https://developers.google.com/identity"),
            "Google answered with a link and no reason"
        );
        assert_eq!(
            strip_urls("error sending request: connection reset by peer"),
            "error sending request: connection reset by peer"
        );
    }

    #[test]
    fn a_gateway_timeout_is_googles_bad_day_and_not_a_refusal() {
        assert!(matches!(
            error_for(504, "Gmail history", SCOPE_MODIFY, None, ""),
            ApiError::RateLimited { .. }
        ));
        assert!(matches!(
            error_for(408, "Gmail history", SCOPE_MODIFY, None, ""),
            ApiError::RateLimited { .. }
        ));
    }

    #[test]
    fn an_unparseable_body_is_truncated_rather_than_pasted_whole() {
        let body = "x".repeat(5000);
        let ApiError::Other(message) = error_for(400, "Gmail send", SCOPE_MODIFY, None, &body)
        else {
            panic!("a 400 is an Other");
        };
        assert!(message.chars().count() < 260, "was {}", message.chars().count());
        assert!(message.ends_with('…'));
    }

    // -- quota ---------------------------------------------------------------------------------

    /// The number the whole sync design rests on: fifty metadata fetches, twenty units each.
    #[test]
    fn a_full_batch_of_fifty_metadata_fetches_is_one_thousand_units() {
        let units = BATCH_SIZE as u32 * Call::MessagesGet.units();
        assert_eq!(units, 1_000);
        assert_eq!(BUDGET_PER_MINUTE / units, 6);
    }

    #[test]
    fn six_batches_fill_the_minute_and_the_seventh_waits_for_the_window_to_roll() {
        let batch = BATCH_SIZE as u32 * Call::MessagesGet.units();
        let mut quota = Quota::new(BUDGET_PER_MINUTE, WINDOW_MS);
        let start = 1_000_000u64;

        for n in 0..6 {
            let at = start + n * 2_000;
            assert_eq!(quota.wait_for(at, batch), 0, "batch {n} should have fitted");
            quota.charge(at, batch);
        }
        assert_eq!(quota.spent(start + 12_000), BUDGET_PER_MINUTE);

        // The seventh would take the minute to 7,000, so the accountant refuses to start it and
        // says how long until the first batch leaves the window.
        let wait = quota.wait_for(start + 12_000, batch);
        assert_eq!(wait, WINDOW_MS - 12_000);

        // Once it has, exactly one batch's worth of room is back.
        let after = start + WINDOW_MS;
        assert_eq!(quota.wait_for(after, batch), 0);
        assert_eq!(quota.spent(after), BUDGET_PER_MINUTE - batch);
    }

    #[test]
    fn an_ordinary_minute_of_work_never_touches_the_ceiling() {
        let mut quota = Quota::new(BUDGET_PER_MINUTE, WINDOW_MS);
        let mut at = 0u64;
        // A poll every twelve seconds, a page of ids, a triage keystroke and a send.
        for call in [
            Call::HistoryList,
            Call::HistoryList,
            Call::HistoryList,
            Call::HistoryList,
            Call::HistoryList,
            Call::MessagesList,
            Call::BatchModify,
            Call::MessagesSend,
            Call::LabelsList,
            Call::GetProfile,
        ] {
            assert_eq!(quota.wait_for(at, call.units()), 0, "{call:?} should fit");
            quota.charge(at, call.units());
            at += 1_000;
        }
        assert_eq!(quota.spent(at), 10 + 5 + 50 + 100 + 1 + 1);
        assert!(quota.spent(at) < BUDGET_PER_MINUTE);
    }

    #[test]
    fn spend_leaves_the_window_on_its_own() {
        let mut quota = Quota::new(BUDGET_PER_MINUTE, WINDOW_MS);
        quota.charge(0, 5_000);
        assert_eq!(quota.spent(59_999), 5_000);
        assert_eq!(quota.spent(60_000), 0);
    }

    // -- backoff -------------------------------------------------------------------------------

    #[test]
    fn the_backoff_schedule_increases_and_is_bounded() {
        let delays: Vec<u64> = (0..10).map(|n| backoff_ms(n, 0)).collect();
        assert_eq!(delays[0], 1_000);
        for pair in delays.windows(2) {
            assert!(pair[1] >= pair[0], "{pair:?} went backwards");
        }
        assert!(delays.iter().all(|d| *d <= MAX_BACKOFF_MS));
        assert_eq!(*delays.last().expect("ten delays"), MAX_BACKOFF_MS);
    }

    #[test]
    fn the_backoff_is_jittered_rather_than_identical_every_time() {
        let mut seen = std::collections::HashSet::new();
        for _ in 0..50 {
            seen.insert(backoff(0));
        }
        assert!(seen.len() > 1, "every delay was the same: {seen:?}");
        assert!(seen.iter().all(|d| (1_000..2_000).contains(d)), "{seen:?}");
    }

    // -- queries -------------------------------------------------------------------------------

    #[test]
    fn a_storage_window_is_an_after_term_in_unix_seconds() {
        // 2026-08-04T00:00:00Z
        assert_eq!(window_query(1_785_801_600_000), "after:1785801600");
        assert_eq!(window_query(0), "after:0");
        assert_eq!(window_query(-1), "after:0");
    }

    #[test]
    fn a_user_query_crosses_unchanged_and_gains_the_window_when_there_is_one() {
        assert_eq!(
            search_query("from:ana has:attachment", None),
            "from:ana has:attachment"
        );
        assert_eq!(
            search_query("  subject:\"the lease\" -in:spam  ", None),
            "subject:\"the lease\" -in:spam"
        );
        assert_eq!(
            search_query("label:Margin/Snoozed", Some(1_785_801_600_000)),
            "label:Margin/Snoozed after:1785801600"
        );
        assert_eq!(search_query("   ", None), "");
        assert_eq!(search_query("", Some(1_785_801_600_000)), "after:1785801600");
    }

    #[test]
    fn hydration_asks_for_a_fixed_header_list_and_nothing_else() {
        let path = metadata_path("18f9a2b3c4d5e6f7");
        assert!(path.starts_with("/gmail/v1/users/me/messages/18f9a2b3c4d5e6f7?format=metadata"));
        assert_eq!(path.matches("&metadataHeaders=").count(), METADATA_HEADERS.len());
        for header in METADATA_HEADERS {
            assert!(path.contains(&format!("&metadataHeaders={header}")), "{header}");
        }
        assert!(!path.contains("Received"));
    }

    #[test]
    fn ids_are_percent_encoded_into_the_path() {
        assert_eq!(path_segment("18f9a2b3c4d5e6f7"), "18f9a2b3c4d5e6f7");
        assert_eq!(path_segment("r-123/456"), "r-123%2F456");
    }

    #[test]
    fn raw_is_base64url_and_decoding_tolerates_either_padding() {
        let bytes = b"Subject: hi\r\n\r\nbody with >>? bytes \xff\xfe";
        let encoded = B64.encode(bytes);
        assert!(!encoded.contains('+') && !encoded.contains('/'));
        assert_eq!(B64.decode(encoded.as_bytes()).expect("padded"), bytes);
        assert_eq!(
            B64.decode(encoded.trim_end_matches('=').as_bytes())
                .expect("unpadded"),
            bytes
        );
    }
}
