// Conversions between the providers' neutral shapes, the store's rows and the IPC DTOs. Nothing
// here reaches the network or the database, so every one of these is a pure function the tests can
// lean on.

use chrono::{DateTime, Local, NaiveDate, TimeZone};

use crate::dto::{EventDraft, EventPatch, Instance};
use crate::provider::{EventWrite, RemoteCalendar, RemoteEvent, When};
use crate::store::write::{self, CalendarRow, EventRow};

fn split(value: Option<&When>) -> (String, Option<String>) {
    match value {
        Some(value) => (value.at.clone(), value.zone.clone()),
        None => (String::new(), None),
    }
}

pub fn is_cancelled(event: &RemoteEvent) -> bool {
    event.status == "cancelled"
}

pub fn event_row(event: &RemoteEvent, calendar_id: &str, account_id: &str) -> EventRow {
    let all_day = event.start.as_ref().is_some_and(When::all_day);
    let (start_at, start_tz) = split(event.start.as_ref());
    let (end_at, end_tz) = split(event.end.as_ref());
    EventRow {
        id: event.id.clone(),
        calendar_id: calendar_id.to_string(),
        account_id: account_id.to_string(),
        etag: event.version.as_ref().map(|v| v.0.clone()),
        status: event.status.clone(),
        summary: event.summary.clone(),
        description: event.description.clone(),
        location: event.location.clone(),
        start_at,
        start_tz,
        end_at,
        end_tz,
        all_day,
        recurrence: event.recurrence.clone(),
        recurring_event_id: event.series_id.clone(),
        original_start: event.original_start.clone(),
        attendees: (!event.attendees.is_empty())
            .then(|| serde_json::to_string(&event.attendees).ok())
            .flatten(),
        conference: event
            .conference
            .as_ref()
            .and_then(|found| serde_json::to_string(found).ok()),
        updated_at: event.updated.clone(),
        color_id: event.color.clone(),
        dirty: false,
        remote_ref: event.remote_ref.clone(),
    }
}

pub fn calendar_row(remote: &RemoteCalendar, account_id: &str) -> CalendarRow {
    CalendarRow {
        id: remote.id.clone(),
        account_id: account_id.to_string(),
        summary: remote.name.clone(),
        description: remote.description.clone(),
        color_hex: remote.color_hex.clone().unwrap_or_default(),
        access_role: remote.access_role.clone(),
        time_zone: remote.time_zone.clone().unwrap_or_default(),
        primary: remote.primary,
        deleted: remote.deleted,
        // The provider's own tick, so a calendar the user has hidden there starts hidden here
        // rather than dumping twenty calendars onto the grid on first sync. Insert only: once the
        // row exists, the local selection is the user's and a refresh must not overwrite it.
        selected: remote.selected,
    }
}

pub fn writable(access_role: &str) -> bool {
    matches!(access_role, "owner" | "writer")
}

/// A date-only string is all-day whatever the flag says. The zone rides along on a timed value.
fn when(value: &str, all_day: bool, zone: Option<&str>) -> When {
    let all_day = all_day || value.len() == 10;
    When {
        at: value.to_string(),
        zone: zone
            .filter(|zone| !all_day && !zone.is_empty())
            .map(str::to_string),
    }
}

pub fn draft_write(
    draft: &EventDraft,
    id: &str,
    start_tz: Option<&str>,
    end_tz: Option<&str>,
) -> EventWrite {
    EventWrite {
        id: Some(id.to_string()),
        summary: Some(draft.summary.clone()),
        description: draft.description.clone(),
        location: draft.location.clone(),
        start: Some(when(&draft.start, draft.all_day, start_tz)),
        end: Some(when(&draft.end, draft.all_day, end_tz)),
        recurrence: (!draft.recurrence.is_empty()).then(|| draft.recurrence.clone()),
        color: draft.color_id.clone().filter(|c| !c.is_empty()),
    }
}

/// An absent field stays absent, which is what makes this a patch. `all_day` and the zones come
/// from the row rather than the patch, because a patch that moves an event without restating its
/// all-day flag still has to say which kind of time it is, and one that moves a series without
/// restating its zone must not let the provider refile it under the calendar's.
pub fn patch_write(
    patch: &EventPatch,
    all_day: bool,
    start_tz: Option<&str>,
    end_tz: Option<&str>,
) -> EventWrite {
    let all_day = patch.all_day.unwrap_or(all_day);
    EventWrite {
        id: None,
        summary: patch.summary.clone(),
        description: patch.description.clone(),
        location: patch.location.clone(),
        start: patch.start.as_deref().map(|value| when(value, all_day, start_tz)),
        end: patch.end.as_deref().map(|value| when(value, all_day, end_tz)),
        recurrence: patch.recurrence.clone(),
        color: patch.color_id.clone(),
    }
}

/// A one-off copy of a series' occurrence at `original_start`, keeping the master's length.
pub fn occurrence_write(master: &EventRow, original_start: &str) -> Result<EventWrite, String> {
    let (start, end) = if original_start.len() == 10 {
        let day = |value: &str| NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|e| e.to_string());
        let span = day(&master.end_at)? - day(&master.start_at)?;
        let start = day(original_start)?;
        (start.to_string(), (start + span).to_string())
    } else {
        let at = |value: &str| DateTime::parse_from_rfc3339(value).map_err(|e| e.to_string());
        let span = at(&master.end_at)? - at(&master.start_at)?;
        let start = at(original_start)?;
        (start.to_rfc3339(), (start + span).to_rfc3339())
    };
    let draft = EventDraft {
        calendar_id: master.calendar_id.clone(),
        summary: master.summary.clone(),
        description: master.description.clone(),
        location: master.location.clone(),
        start,
        end,
        all_day: master.all_day,
        recurrence: Vec::new(),
        color_id: master.color_id.clone(),
    };
    let mut write = draft_write(&draft, "", master.start_tz.as_deref(), master.end_tz.as_deref());
    write.id = None;
    Ok(write)
}

/// The occurrence of a local series at `original_start` as a row of its own, for an edit to land on.
pub fn local_exception(master: &EventRow, original_start: &str) -> Result<EventRow, String> {
    let copy = occurrence_write(master, original_start)?;
    Ok(EventRow {
        id: new_event_id(),
        recurring_event_id: Some(master.id.clone()),
        original_start: Some(original_start.to_string()),
        recurrence: Vec::new(),
        start_at: copy.start.map(|w| w.at).unwrap_or_default(),
        end_at: copy.end.map(|w| w.at).unwrap_or_default(),
        etag: None,
        dirty: false,
        ..master.clone()
    })
}

pub fn recurrence_write(recurrence: &[String]) -> EventWrite {
    EventWrite {
        recurrence: Some(recurrence.to_vec()),
        ..Default::default()
    }
}

pub fn draft_row(draft: &EventDraft, id: &str, account_id: &str) -> EventRow {
    EventRow {
        id: id.to_string(),
        calendar_id: draft.calendar_id.clone(),
        account_id: account_id.to_string(),
        status: "confirmed".to_string(),
        summary: draft.summary.clone(),
        description: draft.description.clone(),
        location: draft.location.clone(),
        start_at: draft.start.clone(),
        end_at: draft.end.clone(),
        all_day: draft.all_day,
        recurrence: draft.recurrence.clone(),
        color_id: draft.color_id.clone().filter(|c| !c.is_empty()),
        dirty: true,
        ..Default::default()
    }
}

pub fn apply_patch(row: &mut EventRow, patch: &EventPatch) {
    if let Some(value) = &patch.summary {
        row.summary = value.clone();
    }
    if let Some(value) = &patch.description {
        row.description = (!value.is_empty()).then(|| value.clone());
    }
    if let Some(value) = &patch.location {
        row.location = (!value.is_empty()).then(|| value.clone());
    }
    if let Some(value) = &patch.color_id {
        row.color_id = (!value.is_empty()).then(|| value.clone());
    }
    if let Some(value) = patch.all_day {
        row.all_day = value;
    }
    if let Some(value) = &patch.start {
        row.start_at = value.clone();
    }
    if let Some(value) = &patch.end {
        row.end_at = value.clone();
    }
    if let Some(value) = &patch.recurrence {
        row.recurrence = value.clone();
    }
}

/// Epoch milliseconds for the optimistic echo only. `recur` owns the real arithmetic; this exists
/// because an all-day event has to sit at local midnight rather than the UTC midnight the store's
/// window columns use, and the echo renders before the next read replaces it.
fn local_ms(value: &str, all_day: bool) -> i64 {
    if all_day || value.len() == 10 {
        return NaiveDate::parse_from_str(value, "%Y-%m-%d")
            .ok()
            .and_then(|date| date.and_hms_opt(0, 0, 0))
            .and_then(|naive| Local.from_local_datetime(&naive).earliest())
            .map(|dt| dt.timestamp_millis())
            .unwrap_or(0);
    }
    write::epoch_ms(value, false).unwrap_or(0)
}

/// The row a local write just made, as the frontend will render it until the next read. Attendees
/// and conferencing are the provider's to fill in, so they are empty rather than guessed at.
pub fn pending_instance(row: &EventRow, color_hex: &str, read_only: bool) -> Instance {
    let color_hex = crate::recur::event_color(row.color_id.as_deref()).unwrap_or(color_hex);
    Instance {
        event_id: row.id.clone(),
        calendar_id: row.calendar_id.clone(),
        account_id: row.account_id.clone(),
        original_start: row.original_start.clone(),
        start: row.start_at.clone(),
        end: row.end_at.clone(),
        start_ms: local_ms(&row.start_at, row.all_day),
        end_ms: local_ms(&row.end_at, row.all_day),
        all_day: row.all_day,
        summary: row.summary.clone(),
        description: row.description.clone(),
        location: row.location.clone(),
        status: row.status.clone(),
        recurring: !row.recurrence.is_empty(),
        color_hex: color_hex.to_string(),
        color_id: row.color_id.clone(),
        etag: row.etag.clone(),
        organizer: None,
        attendees: Vec::new(),
        conference: None,
        read_only,
        pending: true,
    }
}

/// A client-chosen event id in base32hex, which Google accepts and every other provider can use as
/// is. It is what makes an optimistic create idempotent: the row keeps the id it rendered with, and a retry of a request whose response was
/// lost collides with itself rather than creating a second event.
pub fn new_event_id() -> String {
    use rand::RngCore;
    const ALPHABET: &[u8] = b"0123456789abcdefghijklmnopqrstuv";
    let mut bytes = [0u8; 26];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes
        .iter()
        .map(|byte| ALPHABET[(byte & 0x1f) as usize] as char)
        .collect()
}
