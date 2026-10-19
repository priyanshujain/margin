// Google Calendar as a `CalendarProvider`: Calendar's neutral types in, Google's REST shapes out,
// and back again. Everything that knows what `conferenceData` or `colorId` looks like lives here.

use serde_json::{Map, Value, json};
use tauri::Manager;

use crate::dto::{Attendee, Conference};
use crate::google::api::{self, CalendarListPage, EventDateTime, EventsPage, RawCalendar, RawEvent};
use crate::provider::{
    CalendarProvider, Cursor, EventWrite, Page, RemoteCalendar, RemoteEvent, Result, Version, When,
};
use margin_http::HttpError;

pub struct Google {
    pub app: tauri::AppHandle,
}

impl Google {
    async fn token(&self, account_id: &str) -> Result<String> {
        let state = self.app.state::<crate::google::AuthState>();
        crate::google::auth::valid_access_token(&self.app, &state, account_id)
            .await
            .map_err(HttpError::Other)
    }
}

impl CalendarProvider for Google {
    async fn calendars(
        &self,
        account_id: &str,
        cursor: Option<&Cursor>,
        page: Option<&str>,
    ) -> Result<Page<RemoteCalendar>> {
        let access = self.token(account_id).await?;
        let list = api::calendar_list(&access, cursor.map(|c| c.0.as_str()), page).await?;
        Ok(calendar_page(list))
    }

    async fn events(
        &self,
        account_id: &str,
        calendar_id: &str,
        cursor: Option<&Cursor>,
        page: Option<&str>,
    ) -> Result<Page<RemoteEvent>> {
        let access = self.token(account_id).await?;
        let list =
            api::events_list(&access, calendar_id, cursor.map(|c| c.0.as_str()), page).await?;
        Ok(event_page(list))
    }

    async fn create(&self, account_id: &str, calendar_id: &str, write: &EventWrite) -> Result<RemoteEvent> {
        let access = self.token(account_id).await?;
        let raw = api::events_insert(&access, calendar_id, &body(write)).await?;
        Ok(event(&raw))
    }

    async fn update(
        &self,
        account_id: &str,
        calendar_id: &str,
        event_id: &str,
        write: &EventWrite,
        version: Option<&Version>,
    ) -> Result<RemoteEvent> {
        let access = self.token(account_id).await?;
        let raw = api::events_patch(
            &access,
            calendar_id,
            event_id,
            &body(write),
            version.map(|v| v.0.as_str()),
        )
        .await?;
        Ok(event(&raw))
    }

    async fn delete(
        &self,
        account_id: &str,
        calendar_id: &str,
        event_id: &str,
        version: Option<&Version>,
    ) -> Result<()> {
        let access = self.token(account_id).await?;
        api::events_delete(&access, calendar_id, event_id, version.map(|v| v.0.as_str())).await
    }

    async fn occurrence(
        &self,
        account_id: &str,
        calendar_id: &str,
        event_id: &str,
        original_start: &str,
    ) -> Result<Option<RemoteEvent>> {
        let access = self.token(account_id).await?;
        let found = api::events_instances(&access, calendar_id, event_id, original_start).await?;
        Ok(found.first().map(event))
    }
}

pub fn calendar_page(list: CalendarListPage) -> Page<RemoteCalendar> {
    Page {
        items: list.items.iter().map(calendar).collect(),
        next_page: list.next_page_token,
        cursor: list.next_sync_token.map(Cursor),
        full: false,
    }
}

pub fn event_page(list: EventsPage) -> Page<RemoteEvent> {
    Page {
        items: list.items.iter().map(event).collect(),
        next_page: list.next_page_token,
        cursor: list.next_sync_token.map(Cursor),
        full: false,
    }
}

pub fn calendar(raw: &RawCalendar) -> RemoteCalendar {
    RemoteCalendar {
        id: raw.id.clone(),
        name: raw.summary.clone().unwrap_or_else(|| raw.id.clone()),
        description: raw.description.clone(),
        color_hex: raw.background_color.clone(),
        access_role: raw.access_role.clone().unwrap_or_else(|| "reader".to_string()),
        time_zone: raw.time_zone.clone(),
        primary: raw.primary.unwrap_or(false),
        deleted: raw.deleted.unwrap_or(false),
        selected: raw.selected.unwrap_or(true),
    }
}

/// The stored form keeps the RFC3339 string when there is one and the bare date when the event is
/// all-day. Never converted, because an all-day date shifted into a zone is the bug this whole
/// codebase is arranged to avoid.
fn when_of(value: &EventDateTime) -> Option<When> {
    let at = value.date_time.clone().or_else(|| value.date.clone())?;
    Some(When {
        at,
        zone: value.time_zone.clone(),
    })
}

pub fn event(raw: &RawEvent) -> RemoteEvent {
    RemoteEvent {
        id: raw.id.clone(),
        version: raw.etag.clone().map(Version),
        status: raw.status.clone().unwrap_or_else(|| "confirmed".to_string()),
        summary: raw.summary.clone().unwrap_or_default(),
        description: raw.description.clone(),
        location: raw.location.clone(),
        start: raw.start.as_ref().and_then(when_of),
        end: raw.end.as_ref().and_then(when_of),
        recurrence: raw.recurrence.clone().unwrap_or_default(),
        series_id: raw.recurring_event_id.clone(),
        original_start: raw.original_start_time.as_ref().and_then(when_of).map(|w| w.at),
        attendees: raw.attendees.as_ref().map(attendees).unwrap_or_default(),
        conference: raw.conference_data.as_ref().and_then(conference),
        updated: raw.updated.clone(),
        color: raw.color_id.clone(),
        remote_ref: None,
    }
}

/// `{"date": ...}` for an all-day event, `{"dateTime": ...}` otherwise. The zone rides along on a
/// timed event because the offset alone only pins the instant: a series filed under the wrong zone
/// drifts an hour away from itself at the next DST transition.
fn wire_when(value: &When) -> Value {
    if value.all_day() {
        return json!({ "date": value.at });
    }
    match value.zone.as_deref().filter(|zone| !zone.is_empty()) {
        Some(zone) => json!({ "dateTime": value.at, "timeZone": zone }),
        None => json!({ "dateTime": value.at }),
    }
}

/// An absent field stays absent, which is what makes a patch a patch.
pub fn body(write: &EventWrite) -> Value {
    let mut body = Map::new();
    if let Some(id) = &write.id {
        body.insert("id".to_string(), json!(id));
    }
    if let Some(value) = &write.summary {
        body.insert("summary".to_string(), json!(value));
    }
    if let Some(value) = &write.description {
        body.insert("description".to_string(), json!(value));
    }
    if let Some(value) = &write.location {
        body.insert("location".to_string(), json!(value));
    }
    if let Some(value) = &write.start {
        body.insert("start".to_string(), wire_when(value));
    }
    if let Some(value) = &write.end {
        body.insert("end".to_string(), wire_when(value));
    }
    if let Some(value) = &write.recurrence {
        body.insert("recurrence".to_string(), json!(value));
    }
    // Back to the calendar's colour is a null colorId.
    if let Some(value) = &write.color {
        body.insert(
            "colorId".to_string(),
            if value.is_empty() { Value::Null } else { json!(value) },
        );
    }
    Value::Object(body)
}

/// Google's attendee array, read field by field: it sends whichever keys apply.
pub fn attendees(value: &Value) -> Vec<Attendee> {
    let Some(items) = value.as_array() else {
        return Vec::new();
    };
    let text = |item: &Value, key: &str| item.get(key).and_then(Value::as_str).map(str::to_string);
    let flag = |item: &Value, key: &str| item.get(key).and_then(Value::as_bool).unwrap_or(false);
    items
        .iter()
        .filter_map(|item| {
            Some(Attendee {
                email: text(item, "email")?,
                display_name: text(item, "displayName"),
                response_status: text(item, "responseStatus")
                    .unwrap_or_else(|| "needsAction".to_string()),
                organizer: flag(item, "organizer"),
                is_self: flag(item, "self"),
                optional: flag(item, "optional"),
            })
        })
        .collect()
}

/// `conferenceData` read down to the video entry point, or the first one there is.
pub fn conference(value: &Value) -> Option<Conference> {
    let kind = value
        .pointer("/conferenceSolution/key/type")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let entry = value.get("entryPoints").and_then(Value::as_array).and_then(|items| {
        items
            .iter()
            .find(|item| item.get("entryPointType").and_then(Value::as_str) == Some("video"))
            .or_else(|| items.first())
    });
    let uri = entry
        .and_then(|entry| entry.get("uri"))
        .and_then(Value::as_str)
        .map(str::to_string);
    if kind.is_empty() && uri.is_none() {
        return None;
    }
    let label = entry
        .and_then(|entry| entry.get("label"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            value
                .pointer("/conferenceSolution/name")
                .and_then(Value::as_str)
                .map(str::to_string)
        });
    Some(Conference { kind, uri, label })
}

/// A request body queued by a build before schema 3, read back into the write it described.
pub fn write_from_body(body: &Value) -> EventWrite {
    let text = |key: &str| body.get(key).and_then(Value::as_str).map(str::to_string);
    let when = |key: &str| {
        let value = body.get(key)?;
        let at = value
            .get("dateTime")
            .or_else(|| value.get("date"))
            .and_then(Value::as_str)?
            .to_string();
        let zone = value.get("timeZone").and_then(Value::as_str).map(str::to_string);
        Some(When { at, zone })
    };
    EventWrite {
        id: text("id"),
        summary: text("summary"),
        description: text("description"),
        location: text("location"),
        start: when("start"),
        end: when("end"),
        recurrence: body.get("recurrence").and_then(Value::as_array).map(|lines| {
            lines
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        }),
        color: match body.get("colorId") {
            Some(Value::Null) => Some(String::new()),
            Some(value) => value.as_str().map(str::to_string),
            None => None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attendees_survive_google_leaving_most_of_the_keys_out() {
        let json: Value = serde_json::from_str(
            r#"[
            {"email":"a@example.com","displayName":"A","responseStatus":"accepted","organizer":true},
            {"email":"b@example.com","responseStatus":"needsAction","self":true,"optional":true},
            {"displayName":"no address"}
        ]"#,
        )
        .expect("json");
        let parsed = attendees(&json);
        assert_eq!(parsed.len(), 2, "an attendee with no address is not one");
        assert!(parsed[0].organizer && !parsed[0].optional);
        assert!(parsed[1].is_self && parsed[1].optional);
        assert!(attendees(&json!("not a list")).is_empty());
    }

    #[test]
    fn a_conference_is_read_down_to_its_video_entry_point() {
        let json: Value = serde_json::from_str(
            r#"{
            "conferenceSolution":{"key":{"type":"hangoutsMeet"},"name":"Google Meet"},
            "entryPoints":[
                {"entryPointType":"more","uri":"https://tel.meet/x"},
                {"entryPointType":"video","uri":"https://meet.google.com/abc","label":"meet.google.com/abc"}
            ]
        }"#,
        )
        .expect("json");
        let parsed = conference(&json).expect("a conference");
        assert_eq!(parsed.kind, "hangoutsMeet");
        assert_eq!(parsed.uri.as_deref(), Some("https://meet.google.com/abc"));
        assert_eq!(parsed.label.as_deref(), Some("meet.google.com/abc"));
        assert!(conference(&json!({})).is_none());
    }
}
