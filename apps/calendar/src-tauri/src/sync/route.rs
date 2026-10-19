// The provider for each account, picked by the `provider` it was stored with. The engine is handed
// this as its one `CalendarProvider` and never asks which one it is talking to.

use std::collections::HashMap;

use margin_http::HttpError;

use crate::caldav::CalDav;
use crate::google::calendar::Google;
use crate::provider::{
    CalendarProvider, Cursor, EventWrite, Page, RemoteCalendar, RemoteEvent, Result, Version,
};
use crate::store::{read, Store};

pub struct Route {
    google: Google,
    caldav: CalDav,
    kinds: HashMap<String, String>,
}

enum Kind {
    Google,
    CalDav,
}

impl Route {
    pub fn new(app: &tauri::AppHandle, store: &Store) -> Route {
        let kinds = super::with_conn(store, read::accounts)
            .unwrap_or_default()
            .into_iter()
            .map(|account| (account.id, account.provider))
            .collect();
        Route {
            google: Google { app: app.clone() },
            caldav: CalDav { app: app.clone() },
            kinds,
        }
    }

    fn kind(&self, account_id: &str) -> Result<Kind> {
        match self.kinds.get(account_id).map(String::as_str) {
            Some("google") => Ok(Kind::Google),
            Some(crate::caldav::PROVIDER) => Ok(Kind::CalDav),
            Some(other) => Err(HttpError::Other(format!("no provider is built in for {other}"))),
            None => Err(HttpError::Other(format!("account {account_id} is not connected"))),
        }
    }
}

impl CalendarProvider for Route {
    async fn calendars(
        &self,
        account_id: &str,
        cursor: Option<&Cursor>,
        page: Option<&str>,
    ) -> Result<Page<RemoteCalendar>> {
        match self.kind(account_id)? {
            Kind::Google => self.google.calendars(account_id, cursor, page).await,
            Kind::CalDav => self.caldav.calendars(account_id, cursor, page).await,
        }
    }

    async fn events(
        &self,
        account_id: &str,
        calendar_id: &str,
        cursor: Option<&Cursor>,
        page: Option<&str>,
    ) -> Result<Page<RemoteEvent>> {
        match self.kind(account_id)? {
            Kind::Google => self.google.events(account_id, calendar_id, cursor, page).await,
            Kind::CalDav => self.caldav.events(account_id, calendar_id, cursor, page).await,
        }
    }

    async fn create(&self, account_id: &str, calendar_id: &str, write: &EventWrite) -> Result<RemoteEvent> {
        match self.kind(account_id)? {
            Kind::Google => self.google.create(account_id, calendar_id, write).await,
            Kind::CalDav => self.caldav.create(account_id, calendar_id, write).await,
        }
    }

    async fn update(
        &self,
        account_id: &str,
        calendar_id: &str,
        event_id: &str,
        write: &EventWrite,
        version: Option<&Version>,
    ) -> Result<RemoteEvent> {
        match self.kind(account_id)? {
            Kind::Google => self.google.update(account_id, calendar_id, event_id, write, version).await,
            Kind::CalDav => self.caldav.update(account_id, calendar_id, event_id, write, version).await,
        }
    }

    async fn delete(
        &self,
        account_id: &str,
        calendar_id: &str,
        event_id: &str,
        version: Option<&Version>,
    ) -> Result<()> {
        match self.kind(account_id)? {
            Kind::Google => self.google.delete(account_id, calendar_id, event_id, version).await,
            Kind::CalDav => self.caldav.delete(account_id, calendar_id, event_id, version).await,
        }
    }

    async fn occurrence(
        &self,
        account_id: &str,
        calendar_id: &str,
        event_id: &str,
        original_start: &str,
    ) -> Result<Option<RemoteEvent>> {
        match self.kind(account_id)? {
            Kind::Google => self.google.occurrence(account_id, calendar_id, event_id, original_start).await,
            Kind::CalDav => self.caldav.occurrence(account_id, calendar_id, event_id, original_start).await,
        }
    }
}
