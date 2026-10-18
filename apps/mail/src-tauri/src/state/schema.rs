// Tables and migrations for the state database, which is attached as `state`, so every
// statement here names it.
//
// Tables and migrations for the state database. Forward only and idempotent: it runs the steps between the
// version recorded in `state.meta` and VERSION, and refuses to open a database written by a newer build
// rather than silently misreading it. A downgrade that half-understands a schema is how mail goes
// missing, and a clear refusal is recoverable.

use margin_sqlite::{Schema, Stamp};
use rusqlite::Connection;

pub const VERSION: i32 = 2;

// Every statement in it is `IF NOT EXISTS`, so each version runs the whole file and gains its new tables.
const SCHEMA_SQL: &str = include_str!("state.sql");

const SCHEMA: Schema = Schema {
    stamp: Stamp::Meta("state.meta"),
    version: VERSION,
    what: "these decisions",
};

pub fn migrate(conn: &Connection) -> Result<(), String> {
    SCHEMA.migrate(conn, false, |_| {
        conn.execute_batch(SCHEMA_SQL).map_err(|e| e.to_string())
    })
}

pub fn version(conn: &Connection) -> Result<i32, String> {
    SCHEMA.version(conn)
}
