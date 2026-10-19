// Calendars that live on this device and nowhere else. They are ordinary rows in `calendars` under
// the account id `local`, which has no row in `accounts`: nothing pulls them, a write to one is
// final the moment it lands, and the outbox never sees it.

use rusqlite::Connection;

use crate::dto::Calendar;
use crate::store::write::{self, CalendarRow};
use crate::store::{Store, read};

pub const ACCOUNT: &str = "local";

/// The light palette's eight calendar hues, so a new calendar lands exactly on one of them.
const HUES: [&str; 8] = [
    "#6f6194", "#47705f", "#a06044", "#4e6484", "#856434", "#955767", "#3a6b78", "#667141",
];

pub fn hue(index: usize) -> &'static str {
    HUES[index % HUES.len()]
}

pub fn is_local(conn: &Connection, calendar_id: &str) -> Result<bool, String> {
    Ok(read::calendars(conn)?
        .iter()
        .any(|calendar| calendar.id == calendar_id && calendar.account_id == ACCOUNT))
}

/// A first launch with nothing connected opens on a calendar it can write to. A store that already
/// has an account or a calendar is left exactly as it is.
pub fn ensure_first_calendar(conn: &Connection) -> Result<(), String> {
    if read::accounts(conn)?.is_empty() && read::calendars(conn)?.is_empty() {
        create(conn, "Calendar", true)?;
    }
    Ok(())
}

pub fn create(conn: &Connection, name: &str, primary: bool) -> Result<Calendar, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("A calendar needs a name.".to_string());
    }
    let taken = read::calendars(conn)?
        .iter()
        .filter(|calendar| calendar.account_id == ACCOUNT)
        .count();
    let row = CalendarRow {
        id: format!("local-{}", crate::sync::new_event_id()),
        account_id: ACCOUNT.to_string(),
        summary: name.to_string(),
        color_hex: hue(taken).to_string(),
        access_role: "owner".to_string(),
        primary,
        selected: true,
        ..Default::default()
    };
    write::upsert_calendar(conn, &row)?;
    read::calendars(conn)?
        .into_iter()
        .find(|calendar| calendar.id == row.id)
        .ok_or_else(|| "the new calendar did not save".to_string())
}

#[tauri::command]
pub fn calendar_create(
    app: tauri::AppHandle,
    store: tauri::State<'_, Store>,
    name: String,
) -> Result<Calendar, String> {
    let calendar = {
        let conn = store.conn.lock().map_err(|e| e.to_string())?;
        create(&conn, &name, false)?
    };
    crate::emit_store_changed(&app, "calendar-created");
    Ok(calendar)
}

/// A local calendar, or a subscription. One that belongs to an account goes when the account does,
/// or when its provider stops listing it.
#[tauri::command]
pub fn calendar_delete(
    app: tauri::AppHandle,
    store: tauri::State<'_, Store>,
    calendar_id: String,
) -> Result<(), String> {
    {
        let conn = store.conn.lock().map_err(|e| e.to_string())?;
        let deletable = read::calendars(&conn)?.iter().any(|calendar| {
            calendar.id == calendar_id
                && (calendar.account_id == ACCOUNT || calendar.account_id == crate::subscribe::ACCOUNT)
        });
        if !deletable {
            return Err("Only a calendar on this device or a subscription can be deleted here.".to_string());
        }
        write::delete_calendar(&conn, &calendar_id)?;
    }
    crate::emit_store_changed(&app, "calendar-deleted");
    Ok(())
}
