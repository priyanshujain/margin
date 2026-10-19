// iCalendar files and feeds. An imported file becomes a calendar on this device that is yours to
// edit. A subscription is a read-only calendar under the account id `ics`, refetched from its URL
// on every manual sync and otherwise once it is older than `REFRESH_MINUTES`, with the feed's ETag
// kept as its cursor so an unchanged feed costs a 304.

use std::sync::LazyLock;

use rusqlite::{Connection, params};

use crate::dto::Calendar;
use crate::ical::{self, Feed};
use crate::provider::RemoteEvent;
use crate::store::write::{self, CalendarRow, Tx};
use crate::store::{Store, read};
use crate::sync::model;

pub const ACCOUNT: &str = "ics";

const REFRESH_MINUTES: i64 = 30;

static HTTP: LazyLock<reqwest::Client> =
    LazyLock::new(|| margin_http::client(margin_http::Profile::Api));

/// `webcal://` is `https://` by another name, and a feed URL is often pasted with one.
fn normalise(url: &str) -> Result<String, String> {
    let url = url.trim();
    let url = match url.split_once("://") {
        Some((scheme, rest)) if scheme.eq_ignore_ascii_case("webcal") => format!("https://{rest}"),
        Some(_) => url.to_string(),
        None => format!("https://{url}"),
    };
    let parsed = url::Url::parse(&url).map_err(|_| "That is not a calendar address.".to_string())?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("A calendar address starts with https://, http:// or webcal://.".to_string());
    }
    Ok(url)
}

enum Fetched {
    Unchanged,
    Feed { text: String, etag: Option<String> },
}

async fn fetch(url: &str, etag: Option<&str>) -> Result<Fetched, margin_http::HttpError> {
    let mut request = HTTP.get(url).header(reqwest::header::ACCEPT, "text/calendar, */*");
    if let Some(etag) = etag {
        request = request.header(reqwest::header::IF_NONE_MATCH, etag);
    }
    let resp = request.send().await?;
    let status = resp.status().as_u16();
    if status == 304 {
        return Ok(Fetched::Unchanged);
    }
    let retry = margin_http::retry_after(resp.headers());
    let etag = resp
        .headers()
        .get(reqwest::header::ETAG)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let text = resp.text().await?;
    if !(200..300).contains(&status) {
        return Err(margin_http::status_error(
            status,
            "Fetching the calendar",
            retry,
            margin_http::strip_urls(text.trim(), "the server gave no reason"),
        ));
    }
    Ok(Fetched::Feed { text, etag })
}

/// Everything in the calendar is replaced by what the feed says now, in one transaction.
fn replace_events(
    conn: &Connection,
    calendar_id: &str,
    account_id: &str,
    events: &[RemoteEvent],
) -> Result<(), String> {
    let tx = Tx::begin(conn)?;
    conn.execute("DELETE FROM events WHERE calendar_id = ?1", [calendar_id])
        .map_err(|e| e.to_string())?;
    for event in events {
        if model::is_cancelled(event) && event.series_id.is_none() {
            continue;
        }
        write::upsert_event(conn, &model::event_row(event, calendar_id, account_id))?;
    }
    tx.commit()
}

fn find(conn: &Connection, calendar_id: &str) -> Result<Calendar, String> {
    read::calendars(conn)?
        .into_iter()
        .find(|calendar| calendar.id == calendar_id)
        .ok_or_else(|| "the calendar did not save".to_string())
}

/// A file read into a new calendar on this device. Its events are ordinary local events from then
/// on, with nothing tying them to the file.
pub fn import(conn: &Connection, text: &str, fallback_name: &str) -> Result<(Calendar, usize), String> {
    let feed = ical::parse(text)?;
    if feed.events.is_empty() {
        return Err("There are no events in that file.".to_string());
    }
    let name = feed.name.clone().unwrap_or_else(|| fallback_name.to_string());
    let calendar = crate::local::create(conn, &name, false)?;
    replace_events(conn, &calendar.id, crate::local::ACCOUNT, &feed.events)?;
    Ok((calendar, feed.events.len()))
}

fn host_of(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(str::to_string))
        .unwrap_or_else(|| "Subscribed calendar".to_string())
}

pub fn add(conn: &Connection, url: &str, feed: &Feed, etag: Option<&str>) -> Result<Calendar, String> {
    let taken = read::calendars(conn)?
        .iter()
        .filter(|calendar| calendar.account_id == ACCOUNT)
        .count();
    let id = format!("ics-{}", crate::sync::new_event_id());
    write::upsert_calendar(
        conn,
        &CalendarRow {
            id: id.clone(),
            account_id: ACCOUNT.to_string(),
            summary: feed.name.clone().unwrap_or_else(|| host_of(url)),
            color_hex: crate::local::hue(taken + 3).to_string(),
            access_role: "reader".to_string(),
            selected: true,
            ..Default::default()
        },
    )?;
    conn.execute(
        "UPDATE calendars SET source_url = ?1, sync_token = ?2, refreshed_at = ?3 WHERE id = ?4",
        params![url, etag, write::now_ms(), id],
    )
    .map_err(|e| e.to_string())?;
    replace_events(conn, &id, ACCOUNT, &feed.events)?;
    find(conn, &id)
}

struct Due {
    id: String,
    name: String,
    url: String,
    etag: Option<String>,
}

fn due(conn: &Connection, force: bool) -> Result<Vec<Due>, String> {
    let stale = write::now_ms() - REFRESH_MINUTES * 60_000;
    let mut stmt = conn
        .prepare(
            "SELECT id, summary, source_url, sync_token FROM calendars
             WHERE account_id = ?1 AND deleted = 0 AND source_url IS NOT NULL
               AND (?2 OR refreshed_at IS NULL OR refreshed_at < ?3)",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![ACCOUNT, force, stale], |r| {
            Ok(Due {
                id: r.get(0)?,
                name: r.get(1)?,
                url: r.get(2)?,
                etag: r.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// What one round of refreshing did, in the sync pass's terms.
#[derive(Default)]
pub struct Refreshed {
    pub changed: bool,
    pub error: Option<String>,
}

pub async fn refresh(store: &Store, force: bool) -> Refreshed {
    let mut outcome = Refreshed::default();
    let due = match store.conn.lock().map_err(|e| e.to_string()).and_then(|conn| due(&conn, force)) {
        Ok(due) => due,
        Err(error) => {
            outcome.error = Some(error);
            return outcome;
        }
    };
    for feed in due {
        let fetched = fetch(&feed.url, feed.etag.as_deref()).await;
        let result = match fetched {
            Ok(Fetched::Unchanged) => store_refreshed(store, &feed.id, feed.etag.as_deref(), None),
            Ok(Fetched::Feed { text, etag }) => match ical::parse(&text) {
                Ok(parsed) => {
                    outcome.changed = true;
                    store_refreshed(store, &feed.id, etag.as_deref(), Some(&parsed.events))
                }
                Err(error) => Err(error),
            },
            // Offline is the expected state on a plane, and the feed is simply refreshed later.
            Err(margin_http::HttpError::Offline(_) | margin_http::HttpError::Dropped(_)) => Ok(()),
            Err(error) => Err(error.to_string()),
        };
        if let Err(error) = result {
            outcome.error.get_or_insert(format!("{}: {error}", feed.name));
        }
    }
    outcome
}

fn store_refreshed(
    store: &Store,
    calendar_id: &str,
    etag: Option<&str>,
    events: Option<&[RemoteEvent]>,
) -> Result<(), String> {
    let conn = store.conn.lock().map_err(|e| e.to_string())?;
    if let Some(events) = events {
        replace_events(&conn, calendar_id, ACCOUNT, events)?;
    }
    conn.execute(
        "UPDATE calendars SET sync_token = ?1, refreshed_at = ?2 WHERE id = ?3",
        params![etag, write::now_ms(), calendar_id],
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn ics_subscribe(
    app: tauri::AppHandle,
    store: tauri::State<'_, Store>,
    url: String,
) -> Result<Calendar, String> {
    let url = normalise(&url)?;
    let (text, etag) = match fetch(&url, None).await.map_err(|e| e.to_string())? {
        Fetched::Feed { text, etag } => (text, etag),
        Fetched::Unchanged => return Err("The server answered with nothing to read.".to_string()),
    };
    let feed = ical::parse(&text)?;
    let calendar = {
        let conn = store.conn.lock().map_err(|e| e.to_string())?;
        add(&conn, &url, &feed, etag.as_deref())?
    };
    crate::emit_store_changed(&app, "calendar-subscribed");
    Ok(calendar)
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Imported {
    pub calendar: Calendar,
    pub events: usize,
}

#[tauri::command]
pub fn ics_import(
    app: tauri::AppHandle,
    store: tauri::State<'_, Store>,
    text: String,
    name: String,
) -> Result<Imported, String> {
    let (calendar, events) = {
        let conn = store.conn.lock().map_err(|e| e.to_string())?;
        import(&conn, &text, &name)?
    };
    crate::emit_store_changed(&app, "calendar-imported");
    Ok(Imported { calendar, events })
}
