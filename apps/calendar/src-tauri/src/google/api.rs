// Typed wrapper over the Google Calendar REST API.
//
// Everything the sync engine and the write path need:
//   calendar_list(access, sync_token, page_token)          -> CalendarListPage
//   events_list(access, calendar_id, sync_token, page)     -> EventsPage  (one page; caller pages)
//   events_insert / events_patch / events_delete, all carrying If-Match where an etag is known
//   events_instances(access, calendar_id, event_id, original_start) -> Vec<RawEvent>
//
// The sync chain constraint: `singleEvents=false`, `showDeleted=true`, `maxResults=2500`, and the
// parameter set byte-identical on every call in a chain including the first. `timeMin`/`timeMax`
// are rejected alongside `syncToken`.

use std::sync::LazyLock;

use margin_http::{HttpError, path_segment, url_with, with_retry, with_retry_no_replay};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

const BASE: &str = "https://www.googleapis.com/calendar/v3";

/// One page of events. Google caps this at 2500 and the value is part of the sync chain, so it is
/// a constant rather than an argument.
pub const MAX_RESULTS: &str = "2500";

/// A Google event exactly as returned, before flattening into the store.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawEvent {
    pub id: String,
    #[serde(default)]
    pub etag: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub start: Option<EventDateTime>,
    #[serde(default)]
    pub end: Option<EventDateTime>,
    #[serde(default)]
    pub recurrence: Option<Vec<String>>,
    #[serde(default)]
    pub recurring_event_id: Option<String>,
    #[serde(default)]
    pub original_start_time: Option<EventDateTime>,
    #[serde(default)]
    pub organizer: Option<serde_json::Value>,
    #[serde(default)]
    pub attendees: Option<serde_json::Value>,
    #[serde(default)]
    pub conference_data: Option<serde_json::Value>,
    #[serde(default)]
    pub updated: Option<String>,
    #[serde(default)]
    pub transparency: Option<String>,
    #[serde(default)]
    pub color_id: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventDateTime {
    /// Date-only, for an all-day event. Never shift this into a zone.
    #[serde(default)]
    pub date: Option<String>,
    /// RFC3339 with an offset.
    #[serde(default)]
    pub date_time: Option<String>,
    #[serde(default)]
    pub time_zone: Option<String>,
}

/// One page of `events.list`. `next_sync_token` is present only on the final page.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventsPage {
    #[serde(default)]
    pub items: Vec<RawEvent>,
    #[serde(default)]
    pub next_page_token: Option<String>,
    #[serde(default)]
    pub next_sync_token: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawCalendar {
    pub id: String,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub background_color: Option<String>,
    #[serde(default)]
    pub access_role: Option<String>,
    #[serde(default)]
    pub time_zone: Option<String>,
    #[serde(default)]
    pub primary: Option<bool>,
    #[serde(default)]
    pub selected: Option<bool>,
    #[serde(default)]
    pub deleted: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarListPage {
    #[serde(default)]
    pub items: Vec<RawCalendar>,
    #[serde(default)]
    pub next_page_token: Option<String>,
    #[serde(default)]
    pub next_sync_token: Option<String>,
}

/// One page of `events.instances`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstancesPage {
    #[serde(default)]
    items: Vec<RawEvent>,
}

static HTTP: LazyLock<reqwest::Client> =
    LazyLock::new(|| margin_http::client(margin_http::Profile::Api));

/// What a call needs, for the insufficient-scope error, since Google's body never names it.
const SCOPE: &str = "https://www.googleapis.com/auth/calendar";

/// 410 is `HttpError::Gone`, a dead sync token on a list call, and 412 is `HttpError::Conflict`,
/// someone else changing the event first. Everything else is margin-google's reading of the body.
async fn read<T: DeserializeOwned>(resp: reqwest::Response, context: &str) -> Result<T, HttpError> {
    margin_google::read_json(resp, context, SCOPE).await
}

fn create_error(status: u16, retry_after_ms: Option<u64>, body: &str) -> HttpError {
    match status {
        409 => HttpError::Conflict,
        _ => margin_google::error_for(status, "Google event create", SCOPE, retry_after_ms, body),
    }
}

/// The one definition of the sync chain's parameter set. Every call in a chain, including the
/// first, sends exactly this, which is what makes the chain valid.
fn events_params() -> Vec<(&'static str, &'static str)> {
    vec![
        ("singleEvents", "false"),
        ("showDeleted", "true"),
        ("maxResults", MAX_RESULTS),
    ]
}

fn calendar_params() -> Vec<(&'static str, &'static str)> {
    vec![
        ("showDeleted", "true"),
        ("showHidden", "true"),
        ("maxResults", "250"),
    ]
}

pub async fn calendar_list(
    access_token: &str,
    sync_token: Option<&str>,
    page_token: Option<&str>,
) -> Result<CalendarListPage, HttpError> {
    let mut query = calendar_params();
    if let Some(token) = sync_token {
        query.push(("syncToken", token));
    }
    if let Some(token) = page_token {
        query.push(("pageToken", token));
    }
    with_retry(|| async {
        let resp = HTTP
            .get(url_with(&format!("{BASE}/users/me/calendarList"), &query))
            .bearer_auth(access_token)
            .send()
            .await?;
        read(resp, "Google calendar list").await
    })
    .await
}

/// One page. The caller walks `next_page_token` to exhaustion, because `next_sync_token` only
/// appears on the final page and nothing may be committed before it does. Both tokens are passed
/// through when both are given, which is what the official clients do with `list_next`.
pub async fn events_list(
    access_token: &str,
    calendar_id: &str,
    sync_token: Option<&str>,
    page_token: Option<&str>,
) -> Result<EventsPage, HttpError> {
    let mut query = events_params();
    if let Some(token) = sync_token {
        query.push(("syncToken", token));
    }
    if let Some(token) = page_token {
        query.push(("pageToken", token));
    }
    with_retry(|| async {
        let resp = HTTP
            .get(url_with(
                &format!("{BASE}/calendars/{}/events", path_segment(calendar_id)),
                &query,
            ))
            .bearer_auth(access_token)
            .send()
            .await?;
        read(resp, "Google events list").await
    })
    .await
}

/// Not replayed on a dropped connection: the id is chosen locally, so a create whose answer was
/// lost is retried by the outbox. Google answers that retry 409, which is `Conflict`: it landed.
pub async fn events_insert(
    access_token: &str,
    calendar_id: &str,
    body: &serde_json::Value,
) -> Result<RawEvent, HttpError> {
    with_retry_no_replay(|| async {
        let resp = HTTP
            .post(format!("{BASE}/calendars/{}/events", path_segment(calendar_id)))
            .bearer_auth(access_token)
            .json(body)
            .send()
            .await?;
        margin_http::read_json(resp, "Google event create", |status, retry, body| {
            create_error(status, retry, body)
        })
        .await
    })
    .await
}

/// `etag` becomes an If-Match, so a 412 tells the caller someone else changed the event first
/// rather than the write silently clobbering them.
pub async fn events_patch(
    access_token: &str,
    calendar_id: &str,
    event_id: &str,
    body: &serde_json::Value,
    etag: Option<&str>,
) -> Result<RawEvent, HttpError> {
    with_retry_no_replay(|| async {
        let mut request = HTTP
            .patch(format!(
                "{BASE}/calendars/{}/events/{}",
                path_segment(calendar_id),
                path_segment(event_id)
            ))
            .bearer_auth(access_token)
            .json(body);
        if let Some(etag) = etag {
            request = request.header(reqwest::header::IF_MATCH, etag);
        }
        let resp = request.send().await?;
        read(resp, "Google event update").await
    })
    .await
}

/// Idempotent by design: Google answers 410 for an event that is already gone, and a delete of
/// something already deleted is the outcome the caller wanted. That is the one place where a 410
/// is not a dead sync token.
pub async fn events_delete(
    access_token: &str,
    calendar_id: &str,
    event_id: &str,
    etag: Option<&str>,
) -> Result<(), HttpError> {
    let outcome = with_retry(|| async {
        let mut request = HTTP
            .delete(format!(
                "{BASE}/calendars/{}/events/{}",
                path_segment(calendar_id),
                path_segment(event_id)
            ))
            .bearer_auth(access_token);
        if let Some(etag) = etag {
            request = request.header(reqwest::header::IF_MATCH, etag);
        }
        let resp = request.send().await?;
        margin_google::read_empty(resp, "Google event delete", SCOPE).await
    })
    .await;
    match outcome {
        Err(HttpError::NotFound(_) | HttpError::Gone) => Ok(()),
        other => other,
    }
}

/// Resolves the concrete instance of a series at `original_start`, which is how a "this occurrence"
/// edit finds its event rather than constructing the instance id by hand. The filter narrows the
/// result to that one occurrence, so there is nothing to paginate.
pub async fn events_instances(
    access_token: &str,
    calendar_id: &str,
    event_id: &str,
    original_start: &str,
) -> Result<Vec<RawEvent>, HttpError> {
    let page: InstancesPage = with_retry(|| async {
        let resp = HTTP
            .get(url_with(
                &format!(
                    "{BASE}/calendars/{}/events/{}/instances",
                    path_segment(calendar_id),
                    path_segment(event_id)
                ),
                &[
                    ("originalStart", original_start),
                    ("showDeleted", "true"),
                    ("maxResults", "250"),
                ],
            ))
            .bearer_auth(access_token)
            .send()
            .await?;
        read(resp, "Google event instances").await
    })
    .await?;
    Ok(page.items)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn error_for(status: u16, context: &str, body: &str) -> HttpError {
        margin_google::error_for(status, context, SCOPE, None, body)
    }

    #[test]
    fn an_unparseable_body_is_truncated_rather_than_pasted_whole() {
        let body = "x".repeat(5000);
        let HttpError::Other(message) = error_for(400, "Google events list", &body) else {
            panic!("a 400 is an Other");
        };
        assert!(message.chars().count() < 260, "was {}", message.chars().count());
        assert!(message.ends_with('…'));
    }

    #[test]
    fn a_410_is_a_dead_sync_token() {
        assert_eq!(error_for(410, "Google events list", "{}"), HttpError::Gone);
    }

    #[test]
    fn a_412_is_a_lost_race() {
        assert_eq!(error_for(412, "Google event update", "{}"), HttpError::Conflict);
    }

    #[test]
    fn a_create_that_already_landed_is_a_conflict() {
        assert_eq!(create_error(409, None, "{}"), HttpError::Conflict);
        assert!(matches!(create_error(400, None, "{}"), HttpError::Other(_)));
    }

    #[test]
    fn the_sync_chain_parameters_are_fixed_and_carry_no_window() {
        let params = events_params();
        assert_eq!(
            params,
            vec![
                ("singleEvents", "false"),
                ("showDeleted", "true"),
                ("maxResults", "2500"),
            ]
        );
        assert!(params.iter().all(|(key, _)| *key != "timeMin" && *key != "timeMax"));
    }

    #[test]
    fn calendar_ids_are_percent_encoded_into_the_path() {
        assert_eq!(
            path_segment("en.uk#holiday@group.v.calendar.google.com"),
            "en.uk%23holiday%40group.v.calendar.google.com"
        );
        assert_eq!(path_segment("primary"), "primary");
    }
}
