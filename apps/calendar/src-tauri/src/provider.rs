// The seam between Calendar and wherever a calendar lives. The sync engine speaks only these types,
// and each provider (Google today) turns them into its own wire format. Failures are margin-http's
// neutral `HttpError`: `Gone` means a cursor has died and the calendar must be read in full again,
// and `Conflict` means someone else changed the event first, or that a create already landed.

use std::future::Future;

use serde::{Deserialize, Serialize};

use crate::dto::{Attendee, Conference};

/// Where an incremental read left off. Opaque: only the provider that issued it can read it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cursor(pub String);

/// An event's version as its provider names it, sent back on a write so a stale one is refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version(pub String);

/// One page of a read. `cursor` arrives only with the last page, and nothing may be committed
/// before it does.
#[derive(Debug, Clone)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_page: Option<String>,
    pub cursor: Option<Cursor>,
    /// The whole list rather than the changes since `cursor`, so anything missing from it is gone.
    pub full: bool,
}

#[derive(Debug, Clone, Default)]
pub struct RemoteCalendar {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub color_hex: Option<String>,
    /// owner | writer | reader | freeBusyReader
    pub access_role: String,
    pub time_zone: Option<String>,
    pub primary: bool,
    pub deleted: bool,
    /// Whether the provider shows it, which is where a new calendar's local selection starts.
    pub selected: bool,
}

/// A point in time as stored: RFC3339 with an offset, or a bare `YYYY-MM-DD` for an all-day date,
/// with the IANA zone a timed event is filed under when there is one.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct When {
    pub at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zone: Option<String>,
}

impl When {
    pub fn all_day(&self) -> bool {
        self.at.len() == 10
    }
}

#[derive(Debug, Clone, Default)]
pub struct RemoteEvent {
    pub id: String,
    pub version: Option<Version>,
    /// confirmed | tentative | cancelled
    pub status: String,
    pub summary: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub start: Option<When>,
    pub end: Option<When>,
    /// Raw RFC5545 lines: RRULE, RDATE, EXDATE.
    pub recurrence: Vec<String>,
    /// For an exception, the series it belongs to and the unmoved start it replaces.
    pub series_id: Option<String>,
    pub original_start: Option<String>,
    pub attendees: Vec<Attendee>,
    pub conference: Option<Conference>,
    pub updated: Option<String>,
    /// Calendar's event colour, 1 to 11. None follows the calendar.
    pub color: Option<String>,
    /// Where the provider keeps it when the id alone does not say, such as a CalDAV href. An event
    /// with no id and a ref is a deletion of everything under that ref.
    pub remote_ref: Option<String>,
}

/// What Calendar asks a provider to write. An absent field is left alone. An empty string on
/// `description`, `location` or `color` clears it. This is also what the outbox stores.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventWrite {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<When>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<When>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recurrence: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

impl EventWrite {
    /// `other`'s fields over this one's, for an edit layered on a copy of an occurrence.
    pub fn overlay(&mut self, other: &EventWrite) {
        macro_rules! take {
            ($($field:ident),*) => {
                $(if other.$field.is_some() {
                    self.$field = other.$field.clone();
                })*
            };
        }
        take!(id, summary, description, location, start, end, recurrence, color);
    }
}

pub type Result<T> = std::result::Result<T, margin_http::HttpError>;

/// Every call names the account it acts for; the provider turns that into credentials.
pub trait CalendarProvider: Sync {
    fn calendars(
        &self,
        account_id: &str,
        cursor: Option<&Cursor>,
        page: Option<&str>,
    ) -> impl Future<Output = Result<Page<RemoteCalendar>>> + Send;

    fn events(
        &self,
        account_id: &str,
        calendar_id: &str,
        cursor: Option<&Cursor>,
        page: Option<&str>,
    ) -> impl Future<Output = Result<Page<RemoteEvent>>> + Send;

    fn create(
        &self,
        account_id: &str,
        calendar_id: &str,
        write: &EventWrite,
    ) -> impl Future<Output = Result<RemoteEvent>> + Send;

    fn update(
        &self,
        account_id: &str,
        calendar_id: &str,
        event_id: &str,
        write: &EventWrite,
        version: Option<&Version>,
    ) -> impl Future<Output = Result<RemoteEvent>> + Send;

    /// Deleting what is already gone succeeds.
    fn delete(
        &self,
        account_id: &str,
        calendar_id: &str,
        event_id: &str,
        version: Option<&Version>,
    ) -> impl Future<Output = Result<()>> + Send;

    /// The concrete occurrence of a series at `original_start`, when the provider has one to give.
    fn occurrence(
        &self,
        account_id: &str,
        calendar_id: &str,
        event_id: &str,
        original_start: &str,
    ) -> impl Future<Output = Result<Option<RemoteEvent>>> + Send;
}
