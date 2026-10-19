// Tables and migrations. `migrate` is idempotent and forward-only: it runs the steps between the
// version recorded in `meta` and VERSION, and refuses to open a database written by a newer build
// rather than silently misreading it.
//
// No table declares a FOREIGN KEY. The auth agent writes `accounts` with INSERT OR REPLACE, and a
// REPLACE deletes the conflicting row first, which under `foreign_keys=ON` would either cascade
// away every calendar and event for that account or be rejected outright. Referential integrity is
// enforced by `write::wipe_account` and `write::clear_calendar` instead, which is the only place it
// actually matters.

use margin_sqlite::{META, Schema, Stamp};
use rusqlite::Connection;

pub const VERSION: i32 = 5;

// `start_ms`, `end_ms` and `series_until_ms` are not in Google's shape. `start_at` is RFC3339 with
// an offset, or a bare YYYY-MM-DD when all-day, and neither form sorts or compares against an
// epoch-millisecond window in SQL. They are derived on every write from the columns beside them,
// so they are a cache, never a source of truth, and `read::masters_overlapping` is the only reader.
const V1: &str = "
CREATE TABLE IF NOT EXISTS accounts (
    id           TEXT PRIMARY KEY,
    email        TEXT NOT NULL,
    keychain_ref TEXT,
    created_at   INTEGER
);

CREATE TABLE IF NOT EXISTS calendars (
    id          TEXT PRIMARY KEY,
    account_id  TEXT NOT NULL,
    summary     TEXT NOT NULL DEFAULT '',
    description TEXT,
    color_hex   TEXT NOT NULL DEFAULT '',
    selected    INTEGER NOT NULL DEFAULT 1,
    access_role TEXT NOT NULL DEFAULT 'reader',
    time_zone   TEXT NOT NULL DEFAULT '',
    primary_cal INTEGER NOT NULL DEFAULT 0,
    sync_token  TEXT,
    deleted     INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS events (
    id                 TEXT NOT NULL,
    calendar_id        TEXT NOT NULL,
    account_id         TEXT NOT NULL,
    etag               TEXT,
    status             TEXT NOT NULL DEFAULT 'confirmed',
    summary            TEXT NOT NULL DEFAULT '',
    description        TEXT,
    location           TEXT,
    start_at           TEXT,
    start_tz           TEXT,
    end_at             TEXT,
    end_tz             TEXT,
    all_day            INTEGER NOT NULL DEFAULT 0,
    recurrence         TEXT NOT NULL DEFAULT '[]',
    recurring_event_id TEXT,
    original_start     TEXT,
    attendees          TEXT,
    conference         TEXT,
    updated_at         TEXT,
    dirty              INTEGER NOT NULL DEFAULT 0,
    start_ms           INTEGER,
    end_ms             INTEGER,
    series_until_ms    INTEGER,
    color_id           TEXT,
    PRIMARY KEY (id, calendar_id)
);

CREATE TABLE IF NOT EXISTS outbox (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    op             TEXT NOT NULL,
    calendar_id    TEXT NOT NULL,
    event_id       TEXT,
    original_start TEXT,
    scope          TEXT,
    payload        TEXT,
    etag           TEXT,
    attempts       INTEGER NOT NULL DEFAULT 0,
    created_at     INTEGER NOT NULL DEFAULT 0,
    last_error     TEXT
);

CREATE INDEX IF NOT EXISTS idx_events_calendar_start ON events (calendar_id, start_ms);
CREATE INDEX IF NOT EXISTS idx_events_series ON events (recurring_event_id, original_start);
CREATE INDEX IF NOT EXISTS idx_events_account ON events (account_id);
CREATE INDEX IF NOT EXISTS idx_calendars_account ON calendars (account_id);
CREATE INDEX IF NOT EXISTS idx_outbox_created ON outbox (created_at, id);
";

// Google's per-event colour, which overrides the calendar's. Stored as the id rather than a hex so
// it round-trips to `colorId` on the wire unchanged.
const V2: &str = "ALTER TABLE events ADD COLUMN color_id TEXT;";

// The provider an account belongs to, so the sync engine can route it. Every account before this
// was Google's. The same step rewrites the two blobs and the queued writes that were stored in
// Google's request and response shapes into Calendar's own; see `neutral_blobs`.
const V3: &str = "ALTER TABLE accounts ADD COLUMN provider TEXT NOT NULL DEFAULT 'google';";

// Where a subscribed calendar is fetched from and when it last was. Both are NULL for every other
// calendar, so an existing database needs nothing more than the two columns.
const V4: &str = "
ALTER TABLE calendars ADD COLUMN source_url TEXT;
ALTER TABLE calendars ADD COLUMN refreshed_at INTEGER;
";

// A CalDAV account's server and username, and the resource href a CalDAV event lives at. NULL for
// everything that came before, which is all Google's and needs neither.
const V5: &str = "
ALTER TABLE accounts ADD COLUMN server_url TEXT;
ALTER TABLE accounts ADD COLUMN username TEXT;
ALTER TABLE events ADD COLUMN remote_ref TEXT;
CREATE INDEX IF NOT EXISTS idx_events_remote_ref ON events (calendar_id, remote_ref);
";

const SCHEMA: Schema = Schema {
    stamp: Stamp::Meta(META),
    version: VERSION,
    what: "this calendar database",
};

pub fn migrate(conn: &Connection) -> Result<(), String> {
    SCHEMA.migrate(conn, true, |found| {
        if found < 1 {
            conn.execute_batch(V1).map_err(|e| e.to_string())?;
        } else if found < 2 {
            // Only for a database that predates the column; V1 already declares it.
            conn.execute_batch(V2).map_err(|e| e.to_string())?;
        }
        if found < 3 {
            conn.execute_batch(V3).map_err(|e| e.to_string())?;
            neutral_blobs(conn)?;
        }
        if found < 4 {
            conn.execute_batch(V4).map_err(|e| e.to_string())?;
        }
        if found < 5 {
            conn.execute_batch(V5).map_err(|e| e.to_string())?;
        }
        Ok(())
    })
}

/// Attendees and conferencing were Google's JSON verbatim, and a queued write was the Google request
/// body. All three become Calendar's own shapes, which are what every provider reads and writes.
fn neutral_blobs(conn: &Connection) -> Result<(), String> {
    use crate::google::calendar::{attendees, conference, write_from_body};
    use serde_json::Value;

    let parse = |text: &str| serde_json::from_str::<Value>(text).map_err(|e| e.to_string());
    let rows: Vec<(i64, Option<String>, Option<String>)> = {
        let mut stmt = conn
            .prepare(
                "SELECT rowid, attendees, conference FROM events
                 WHERE attendees IS NOT NULL OR conference IS NOT NULL",
            )
            .map_err(|e| e.to_string())?;
        stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .map_err(|e| e.to_string())?
            .collect::<Result<_, _>>()
            .map_err(|e| e.to_string())?
    };
    for (rowid, people, meeting) in rows {
        let people = match people {
            Some(text) => {
                let list = attendees(&parse(&text)?);
                (!list.is_empty())
                    .then(|| serde_json::to_string(&list))
                    .transpose()
                    .map_err(|e| e.to_string())?
            }
            None => None,
        };
        let meeting = match meeting {
            Some(text) => conference(&parse(&text)?)
                .map(|found| serde_json::to_string(&found))
                .transpose()
                .map_err(|e| e.to_string())?,
            None => None,
        };
        conn.execute(
            "UPDATE events SET attendees = ?1, conference = ?2 WHERE rowid = ?3",
            rusqlite::params![people, meeting, rowid],
        )
        .map_err(|e| e.to_string())?;
    }

    let queued: Vec<(i64, String)> = {
        let mut stmt = conn
            .prepare("SELECT id, payload FROM outbox WHERE payload IS NOT NULL")
            .map_err(|e| e.to_string())?;
        stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .map_err(|e| e.to_string())?
            .collect::<Result<_, _>>()
            .map_err(|e| e.to_string())?
    };
    for (id, payload) in queued {
        let write = write_from_body(&parse(&payload)?);
        let text = serde_json::to_string(&write).map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE outbox SET payload = ?1 WHERE id = ?2",
            rusqlite::params![text, id],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}
