// CalDAV (RFC 4791) as a `CalendarProvider`, signed in with a username and an app password, which
// is what Fastmail, iCloud and Nextcloud all offer. A calendar's id is its collection URL, an
// event's resource href is kept as its `remote_ref`, and the cursor is the collection's WebDAV
// sync token (RFC 6578). A write fetches the resource, changes only what Calendar changed and puts
// it back with If-Match, so whatever else the resource carries survives the round trip.

use std::sync::LazyLock;
use std::time::Duration;

use margin_http::HttpError;
use reqwest::header::{ETAG, HeaderMap, IF_MATCH, IF_NONE_MATCH, LOCATION};
use reqwest::{Method, StatusCode};
use tauri::Manager;

use crate::dto::Account;
use crate::ical;
use crate::provider::{
    CalendarProvider, Cursor, EventWrite, Page, RemoteCalendar, RemoteEvent, Result, Version,
};
use crate::store::{Store, read, write};

pub const PROVIDER: &str = "caldav";

const MAX_REDIRECTS: usize = 5;

/// Redirects are followed by hand: reqwest turns a redirected PROPFIND into a GET, and the
/// well-known discovery URL is always a redirect.
static HTTP: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(60))
        .build()
        .expect("could not build the CalDAV client")
});

#[derive(Clone)]
pub struct Login {
    pub username: String,
    pub password: String,
}

struct Reply {
    url: url::Url,
    status: StatusCode,
    headers: HeaderMap,
    body: String,
}

fn method(name: &str) -> Method {
    Method::from_bytes(name.as_bytes()).expect("a valid method")
}

async fn send(
    login: &Login,
    verb: &str,
    url: &str,
    headers: &[(&str, String)],
    body: Option<String>,
) -> Result<Reply> {
    let mut url = url::Url::parse(url).map_err(|e| HttpError::Other(format!("{url}: {e}")))?;
    for _ in 0..=MAX_REDIRECTS {
        let mut request = HTTP
            .request(method(verb), url.clone())
            .basic_auth(&login.username, Some(&login.password));
        for (name, value) in headers {
            request = request.header(*name, value);
        }
        if let Some(body) = &body {
            request = request.body(body.clone());
        }
        let resp = request.send().await?;
        let status = resp.status();
        if status.is_redirection()
            && let Some(next) = resp.headers().get(LOCATION).and_then(|v| v.to_str().ok())
        {
            url = url.join(next).map_err(|e| HttpError::Other(e.to_string()))?;
            continue;
        }
        let headers = resp.headers().clone();
        let body = resp.text().await?;
        return Ok(Reply { url, status, headers, body });
    }
    Err(HttpError::Other("the server redirected too many times".to_string()))
}

fn refuse(reply: &Reply, context: &str) -> HttpError {
    if reply.status == StatusCode::UNAUTHORIZED {
        return HttpError::Unauthorized(
            "the server did not accept that username and password".to_string(),
        );
    }
    let reason = margin_http::strip_urls(
        reply.body.trim().chars().take(200).collect::<String>().as_str(),
        "the server gave no reason",
    );
    margin_http::status_error(
        reply.status.as_u16(),
        context,
        margin_http::retry_after(&reply.headers),
        reason,
    )
}

fn etag_of(reply: &Reply) -> Option<String> {
    reply.headers.get(ETAG).and_then(|v| v.to_str().ok()).map(str::to_string)
}

// -- multistatus -------------------------------------------------------------------------------

struct Member {
    href: String,
    /// The response-level status, which is how a sync report names a deleted member.
    gone: bool,
    props: Vec<Prop>,
}

struct Prop {
    name: String,
    text: String,
    children: Vec<String>,
    href: Option<String>,
    comps: Vec<String>,
}

impl Member {
    fn prop(&self, name: &str) -> Option<&Prop> {
        self.props.iter().find(|p| p.name == name)
    }
}

fn multistatus(body: &str, base: &url::Url) -> Result<(Vec<Member>, Option<String>)> {
    let doc = roxmltree::Document::parse(body)
        .map_err(|e| HttpError::Other(format!("the server's answer is not XML: {e}")))?;
    let mut members = Vec::new();
    let mut token = None;
    for node in doc.root_element().children().filter(|n| n.is_element()) {
        match node.tag_name().name() {
            "sync-token" => token = node.text().map(|t| t.trim().to_string()),
            "response" => {
                let mut href = String::new();
                let mut gone = false;
                let mut props = Vec::new();
                for part in node.children().filter(|n| n.is_element()) {
                    match part.tag_name().name() {
                        "href" => {
                            let raw = part.text().unwrap_or_default().trim();
                            href = base.join(raw).map(|u| u.to_string()).unwrap_or_else(|_| raw.to_string());
                        }
                        "status" => gone = part.text().unwrap_or_default().contains(" 404"),
                        "propstat" => {
                            let ok = part
                                .children()
                                .find(|n| n.tag_name().name() == "status")
                                .and_then(|n| n.text())
                                .is_some_and(|t| t.contains(" 200"));
                            if !ok {
                                continue;
                            }
                            let Some(prop) = part.children().find(|n| n.tag_name().name() == "prop") else {
                                continue;
                            };
                            for value in prop.children().filter(|n| n.is_element()) {
                                let descendants: Vec<_> = value.descendants().filter(|n| n.is_element()).collect();
                                props.push(Prop {
                                    name: value.tag_name().name().to_string(),
                                    text: value.text().unwrap_or_default().trim().to_string(),
                                    children: descendants.iter().skip(1).map(|n| n.tag_name().name().to_string()).collect(),
                                    href: descendants
                                        .iter()
                                        .find(|n| n.tag_name().name() == "href")
                                        .and_then(|n| n.text())
                                        .and_then(|t| base.join(t.trim()).ok())
                                        .map(|u| u.to_string()),
                                    comps: descendants
                                        .iter()
                                        .filter(|n| n.tag_name().name() == "comp")
                                        .filter_map(|n| n.attribute("name"))
                                        .map(str::to_string)
                                        .collect(),
                                });
                            }
                        }
                        _ => {}
                    }
                }
                members.push(Member { href, gone, props });
            }
            _ => {}
        }
    }
    Ok((members, token))
}

async fn propfind(login: &Login, url: &str, depth: &str, props: &str) -> Result<(url::Url, Vec<Member>)> {
    let body = format!(
        r#"<?xml version="1.0" encoding="utf-8"?><d:propfind xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav" xmlns:cs="http://calendarserver.org/ns/" xmlns:x="http://apple.com/ns/ical/"><d:prop>{props}</d:prop></d:propfind>"#
    );
    let reply = send(
        login,
        "PROPFIND",
        url,
        &[("Depth", depth.to_string()), ("Content-Type", "application/xml; charset=utf-8".to_string())],
        Some(body),
    )
    .await?;
    if reply.status.as_u16() != 207 {
        return Err(refuse(&reply, "Reading the calendar server"));
    }
    let (members, _) = multistatus(&reply.body, &reply.url)?;
    Ok((reply.url, members))
}

// -- discovery ---------------------------------------------------------------------------------

/// The URL that holds the user's calendars, found from whatever address they typed: the server's
/// root, its well-known CalDAV address, a principal, or the home itself.
pub async fn discover(login: &Login, server: &str) -> Result<String> {
    let server = if server.contains("://") { server.to_string() } else { format!("https://{server}") };
    let mut principal = None;
    for candidate in [server.clone(), join(&server, "/.well-known/caldav")] {
        match propfind(login, &candidate, "0", "<d:current-user-principal/>").await {
            Ok((_, members)) => {
                principal = members
                    .iter()
                    .find_map(|m| m.prop("current-user-principal").and_then(|p| p.href.clone()));
                if principal.is_some() {
                    break;
                }
            }
            Err(HttpError::Unauthorized(reason)) => return Err(HttpError::Unauthorized(reason)),
            Err(_) => continue,
        }
    }
    let principal = principal.ok_or_else(|| {
        HttpError::Other("that address does not look like a CalDAV server".to_string())
    })?;
    let (_, members) = propfind(login, &principal, "0", "<c:calendar-home-set/>").await?;
    members
        .iter()
        .find_map(|m| m.prop("calendar-home-set").and_then(|p| p.href.clone()))
        .ok_or_else(|| HttpError::Other("the server named no calendar home".to_string()))
}

fn join(base: &str, path: &str) -> String {
    url::Url::parse(base)
        .and_then(|u| u.join(path))
        .map(|u| u.to_string())
        .unwrap_or_else(|_| format!("{base}{path}"))
}

const CALENDAR_PROPS: &str = "<d:resourcetype/><d:displayname/><x:calendar-color/><c:calendar-description/><c:supported-calendar-component-set/><d:current-user-privilege-set/>";

pub async fn list_calendars(login: &Login, home: &str) -> Result<Vec<RemoteCalendar>> {
    let (_, members) = propfind(login, home, "1", CALENDAR_PROPS).await?;
    Ok(members
        .iter()
        .filter(|m| m.prop("resourcetype").is_some_and(|p| p.children.iter().any(|c| c == "calendar")))
        .filter(|m| {
            m.prop("supported-calendar-component-set")
                .is_none_or(|p| p.comps.is_empty() || p.comps.iter().any(|c| c.eq_ignore_ascii_case("VEVENT")))
        })
        .map(|m| {
            let writable = m.prop("current-user-privilege-set").is_none_or(|p| {
                p.children.iter().any(|c| matches!(c.as_str(), "write" | "write-content" | "all"))
            });
            RemoteCalendar {
                id: m.href.clone(),
                name: m
                    .prop("displayname")
                    .map(|p| p.text.clone())
                    .filter(|n| !n.is_empty())
                    .unwrap_or_else(|| last_segment(&m.href)),
                description: m.prop("calendar-description").map(|p| p.text.clone()).filter(|d| !d.is_empty()),
                color_hex: m
                    .prop("calendar-color")
                    .map(|p| p.text.chars().take(7).collect::<String>())
                    .filter(|c| c.starts_with('#') && c.len() == 7),
                access_role: if writable { "owner" } else { "reader" }.to_string(),
                time_zone: None,
                primary: false,
                deleted: false,
                selected: true,
            }
        })
        .collect())
}

fn last_segment(href: &str) -> String {
    href.trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or(href)
        .to_string()
}

// -- events ------------------------------------------------------------------------------------

fn resource_events(text: &str, href: &str, etag: Option<&str>) -> Result<Vec<RemoteEvent>> {
    let feed = ical::parse(text).map_err(HttpError::Other)?;
    Ok(feed
        .events
        .into_iter()
        .map(|mut event| {
            event.remote_ref = Some(href.to_string());
            event.version = etag.map(|e| Version(e.to_string()));
            event
        })
        .collect())
}

fn sync_body(token: Option<&str>) -> String {
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?><d:sync-collection xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav"><d:sync-token>{}</d:sync-token><d:sync-level>1</d:sync-level><d:prop><d:getetag/><c:calendar-data/></d:prop></d:sync-collection>"#,
        token.unwrap_or_default()
    )
}

async fn multiget(login: &Login, calendar: &str, hrefs: &[String]) -> Result<Vec<Member>> {
    if hrefs.is_empty() {
        return Ok(Vec::new());
    }
    let wanted: String = hrefs
        .iter()
        .map(|h| format!("<d:href>{}</d:href>", url::Url::parse(h).map(|u| u.path().to_string()).unwrap_or_else(|_| h.clone())))
        .collect();
    let body = format!(
        r#"<?xml version="1.0" encoding="utf-8"?><c:calendar-multiget xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav"><d:prop><d:getetag/><c:calendar-data/></d:prop>{wanted}</c:calendar-multiget>"#
    );
    let reply = send(
        login,
        "REPORT",
        calendar,
        &[("Depth", "1".to_string()), ("Content-Type", "application/xml; charset=utf-8".to_string())],
        Some(body),
    )
    .await?;
    if reply.status.as_u16() != 207 {
        return Err(refuse(&reply, "Reading events"));
    }
    Ok(multistatus(&reply.body, &reply.url)?.0)
}

pub async fn changes(login: &Login, calendar: &str, token: Option<&str>) -> Result<Page<RemoteEvent>> {
    let reply = send(
        login,
        "REPORT",
        calendar,
        &[("Depth", "1".to_string()), ("Content-Type", "application/xml; charset=utf-8".to_string())],
        Some(sync_body(token)),
    )
    .await?;
    // A token the server has forgotten is refused with `valid-sync-token`, as a 403 or a 409.
    if token.is_some()
        && matches!(reply.status.as_u16(), 403 | 409)
        && reply.body.contains("valid-sync-token")
    {
        return Err(HttpError::Gone);
    }
    if reply.status.as_u16() != 207 {
        return Err(refuse(&reply, "Reading events"));
    }
    let (members, next) = multistatus(&reply.body, &reply.url)?;
    let mut items = Vec::new();
    let mut missing = Vec::new();
    for member in members.iter().filter(|m| m.href.trim_end_matches('/') != calendar.trim_end_matches('/')) {
        if member.gone {
            items.push(RemoteEvent {
                status: "cancelled".to_string(),
                remote_ref: Some(member.href.clone()),
                ..Default::default()
            });
            continue;
        }
        let etag = member.prop("getetag").map(|p| p.text.clone());
        match member.prop("calendar-data").map(|p| p.text.as_str()).filter(|t| !t.is_empty()) {
            Some(text) => items.extend(resource_events(text, &member.href, etag.as_deref())?),
            None => missing.push(member.href.clone()),
        }
    }
    for member in multiget(login, calendar, &missing).await? {
        let etag = member.prop("getetag").map(|p| p.text.clone());
        if let Some(text) = member.prop("calendar-data").map(|p| p.text.as_str()) {
            items.extend(resource_events(text, &member.href, etag.as_deref())?);
        }
    }
    Ok(Page {
        items,
        next_page: None,
        cursor: Some(Cursor(next.ok_or_else(|| {
            HttpError::Other("the server does not keep sync tokens, which Calendar needs".to_string())
        })?)),
        full: false,
    })
}

fn stamp() -> String {
    chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string()
}

async fn fetch(login: &Login, href: &str) -> Result<(String, Option<String>)> {
    let reply = send(login, "GET", href, &[], None).await?;
    if !reply.status.is_success() {
        return Err(refuse(&reply, "Reading the event"));
    }
    let etag = etag_of(&reply);
    Ok((reply.body, etag))
}

async fn put(login: &Login, href: &str, text: String, guard: (&str, String)) -> Result<Option<String>> {
    let reply = send(
        login,
        "PUT",
        href,
        &[("Content-Type", "text/calendar; charset=utf-8".to_string()), guard],
        Some(text),
    )
    .await?;
    match reply.status.as_u16() {
        200..=299 => Ok(etag_of(&reply)),
        412 => Err(HttpError::Conflict),
        _ => Err(refuse(&reply, "Saving the event")),
    }
}

fn split(event_id: &str) -> (&str, Option<&str>) {
    match event_id.split_once("::") {
        Some((uid, original)) => (uid, Some(original)),
        None => (event_id, None),
    }
}

fn landed(text: &str, href: &str, etag: Option<String>, event_id: &str) -> Result<RemoteEvent> {
    resource_events(text, href, etag.as_deref())?
        .into_iter()
        .find(|e| e.id == event_id || split(&e.id).0 == event_id)
        .ok_or_else(|| HttpError::Other("the saved event is not in its resource".to_string()))
}

// -- the provider ------------------------------------------------------------------------------

pub struct CalDav {
    pub app: tauri::AppHandle,
}

impl CalDav {
    fn store(&self) -> Result<tauri::State<'_, Store>> {
        self.app
            .try_state::<Store>()
            .ok_or_else(|| HttpError::Other("the local store is not open".to_string()))
    }

    fn login(&self, account_id: &str) -> Result<(Login, String)> {
        let store = self.store()?;
        let (home, username) = {
            let conn = store.conn.lock().map_err(|e| HttpError::Other(e.to_string()))?;
            read::account_server(&conn, account_id).map_err(HttpError::Other)?
        }
        .ok_or_else(|| HttpError::Other(format!("account {account_id} has no server")))?;
        let password = crate::secrets::load(account_id)
            .map_err(HttpError::Other)?
            .ok_or_else(|| HttpError::Unauthorized("no password is stored for this account".to_string()))?;
        Ok((Login { username, password }, home))
    }

    fn href(&self, calendar_id: &str, uid: &str) -> Result<String> {
        let store = self.store()?;
        let known = {
            let conn = store.conn.lock().map_err(|e| HttpError::Other(e.to_string()))?;
            read::remote_ref(&conn, calendar_id, uid).map_err(HttpError::Other)?
        };
        Ok(known.unwrap_or_else(|| join(calendar_id, &format!("{}.ics", url_safe(uid)))))
    }
}

fn url_safe(uid: &str) -> String {
    uid.chars()
        .map(|c| if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') { c } else { '_' })
        .collect()
}

impl CalendarProvider for CalDav {
    async fn calendars(
        &self,
        account_id: &str,
        _cursor: Option<&Cursor>,
        _page: Option<&str>,
    ) -> Result<Page<RemoteCalendar>> {
        let (login, home) = self.login(account_id)?;
        Ok(Page {
            items: list_calendars(&login, &home).await?,
            next_page: None,
            cursor: Some(Cursor("caldav".to_string())),
            full: true,
        })
    }

    async fn events(
        &self,
        account_id: &str,
        calendar_id: &str,
        cursor: Option<&Cursor>,
        _page: Option<&str>,
    ) -> Result<Page<RemoteEvent>> {
        let (login, _) = self.login(account_id)?;
        changes(&login, calendar_id, cursor.map(|c| c.0.as_str())).await
    }

    async fn create(&self, account_id: &str, calendar_id: &str, write: &EventWrite) -> Result<RemoteEvent> {
        let (login, _) = self.login(account_id)?;
        let uid = write.id.clone().unwrap_or_else(crate::sync::new_event_id);
        let href = join(calendar_id, &format!("{}.ics", url_safe(&uid)));
        let text = ical::new_resource(&uid, write, &stamp()).map_err(HttpError::Other)?;
        let etag = put(&login, &href, text.clone(), (IF_NONE_MATCH.as_str(), "*".to_string())).await?;
        landed(&text, &href, etag, &uid)
    }

    async fn update(
        &self,
        account_id: &str,
        calendar_id: &str,
        event_id: &str,
        write: &EventWrite,
        _version: Option<&Version>,
    ) -> Result<RemoteEvent> {
        let (login, _) = self.login(account_id)?;
        let (uid, original) = split(event_id);
        let href = self.href(calendar_id, uid)?;
        let (current, etag) = fetch(&login, &href).await?;
        let text = ical::apply(&current, original, write, &stamp()).map_err(HttpError::Other)?;
        let guard = match etag {
            Some(etag) => (IF_MATCH.as_str(), etag),
            None => (IF_MATCH.as_str(), "*".to_string()),
        };
        let etag = put(&login, &href, text.clone(), guard).await?;
        landed(&text, &href, etag, event_id)
    }

    async fn delete(
        &self,
        account_id: &str,
        calendar_id: &str,
        event_id: &str,
        _version: Option<&Version>,
    ) -> Result<()> {
        let (login, _) = self.login(account_id)?;
        let (uid, original) = split(event_id);
        let href = self.href(calendar_id, uid)?;
        if let Some(original) = original {
            let (current, etag) = fetch(&login, &href).await?;
            let text = ical::exclude(&current, original, &stamp()).map_err(HttpError::Other)?;
            put(&login, &href, text, (IF_MATCH.as_str(), etag.unwrap_or_else(|| "*".to_string()))).await?;
            return Ok(());
        }
        let reply = send(&login, "DELETE", &href, &[], None).await?;
        match reply.status.as_u16() {
            200..=299 | 404 | 410 => Ok(()),
            _ => Err(refuse(&reply, "Deleting the event")),
        }
    }

    /// Every occurrence is addressable as the series' UID and its start, whether or not the
    /// resource has an override for it yet.
    async fn occurrence(
        &self,
        _account_id: &str,
        _calendar_id: &str,
        event_id: &str,
        original_start: &str,
    ) -> Result<Option<RemoteEvent>> {
        Ok(Some(RemoteEvent {
            id: ical::exception_id(event_id, original_start),
            ..Default::default()
        }))
    }
}

// -- accounts ----------------------------------------------------------------------------------

#[tauri::command]
pub async fn caldav_connect(
    app: tauri::AppHandle,
    server: String,
    username: String,
    password: String,
) -> std::result::Result<Account, String> {
    let username = username.trim().to_string();
    if username.is_empty() || password.is_empty() {
        return Err("A username and an app password are both needed.".to_string());
    }
    let login = Login { username: username.clone(), password: password.clone() };
    let home = discover(&login, server.trim()).await.map_err(|e| e.to_string())?;
    list_calendars(&login, &home).await.map_err(|e| e.to_string())?;

    let id = format!("caldav-{}", crate::sync::new_event_id());
    let label = username.clone();
    crate::secrets::store(&id, &password)?;
    {
        let store = app.try_state::<Store>().ok_or("the local store is not open")?;
        let conn = store.conn.lock().map_err(|e| e.to_string())?;
        write::upsert_account(&conn, &id, PROVIDER, &label, Some(&crate::secrets::reference(&id)))?;
        write::set_account_server(&conn, &id, &home, &username)?;
    }
    crate::emit_store_changed(&app, "account-connected");
    Ok(Account {
        id,
        email: label,
        connected: true,
        provider: PROVIDER.to_string(),
        server: Some(home),
    })
}

/// The password is forgotten and the account's rows go. There is nothing to revoke: an app password
/// is the user's to delete at their provider.
pub fn disconnect(app: &tauri::AppHandle, account_id: &str) -> std::result::Result<(), String> {
    let removed = crate::secrets::delete(account_id);
    {
        let store = app.try_state::<Store>().ok_or("the local store is not open")?;
        let conn = store.conn.lock().map_err(|e| e.to_string())?;
        write::wipe_account(&conn, account_id)?;
    }
    crate::emit_store_changed(app, "disconnect");
    removed
}
