use std::path::Path;
use std::time::Duration;

use rusqlite::{Connection, OptionalExtension, params};

/// WAL and `synchronous = NORMAL` always; the rest varies by app.
#[derive(Debug, Clone, Copy, Default)]
pub struct Options {
    pub foreign_keys: bool,
    pub busy_timeout: Option<Duration>,
    /// Makes the WAL give its space back after a checkpoint.
    pub journal_size_limit: Option<i64>,
}

pub fn open(path: &Path, options: Options) -> Result<Connection, String> {
    let conn = Connection::open(path).map_err(|e| e.to_string())?;
    configure(&conn, options)?;
    Ok(conn)
}

pub fn configure(conn: &Connection, options: Options) -> Result<(), String> {
    let mut pragmas = format!(
        "PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL; PRAGMA foreign_keys = {};",
        if options.foreign_keys { "ON" } else { "OFF" }
    );
    if let Some(limit) = options.journal_size_limit {
        pragmas.push_str(&format!(" PRAGMA journal_size_limit = {limit};"));
    }
    conn.execute_batch(&pragmas).map_err(|e| e.to_string())?;
    if let Some(timeout) = options.busy_timeout {
        conn.busy_timeout(timeout).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// ATTACH takes a literal, so the path is escaped rather than bound.
pub fn attach(conn: &Connection, path: &Path, alias: &str) -> Result<(), String> {
    conn.execute_batch(&format!(
        "ATTACH DATABASE '{}' AS {alias};",
        path.to_string_lossy().replace('\'', "''")
    ))
    .map_err(|e| e.to_string())
}

/// A transaction that does not need `&mut Connection`. Rolls back unless committed, so an early
/// `?` cannot leave half a change behind.
pub struct Tx<'a> {
    conn: &'a Connection,
    done: bool,
}

impl<'a> Tx<'a> {
    /// `BEGIN IMMEDIATE`, taking the write lock up front.
    pub fn begin(conn: &'a Connection) -> Result<Tx<'a>, String> {
        Tx::start(conn, "BEGIN IMMEDIATE;")
    }

    /// `BEGIN`, taking the write lock at the first write.
    pub fn deferred(conn: &'a Connection) -> Result<Tx<'a>, String> {
        Tx::start(conn, "BEGIN;")
    }

    fn start(conn: &'a Connection, sql: &str) -> Result<Tx<'a>, String> {
        conn.execute_batch(sql).map_err(|e| e.to_string())?;
        Ok(Tx { conn, done: false })
    }

    pub fn commit(mut self) -> Result<(), String> {
        self.done = true;
        self.conn
            .execute_batch("COMMIT;")
            .map_err(|e| e.to_string())
    }
}

impl Drop for Tx<'_> {
    fn drop(&mut self) {
        if !self.done {
            let _ = self.conn.execute_batch("ROLLBACK;");
        }
    }
}

/// The key-value table every app keeps beside its schema, `meta` or `state.meta` when attached.
pub const META: &str = "meta";

const VERSION_KEY: &str = "schema_version";

pub fn meta_get(conn: &Connection, table: &str, key: &str) -> Result<Option<String>, String> {
    conn.query_row(
        &format!("SELECT value FROM {table} WHERE key = ?1"),
        [key],
        |row| row.get(0),
    )
    .optional()
    .map_err(|e| e.to_string())
}

pub fn meta_set(conn: &Connection, table: &str, key: &str, value: &str) -> Result<(), String> {
    conn.execute(
        &format!(
            "INSERT INTO {table} (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value"
        ),
        params![key, value],
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

pub fn meta_i64(conn: &Connection, table: &str, key: &str) -> Result<Option<i64>, String> {
    Ok(meta_get(conn, table, key)?.and_then(|v| v.parse().ok()))
}

pub fn meta_clear(conn: &Connection, table: &str, key: &str) -> Result<(), String> {
    conn.execute(&format!("DELETE FROM {table} WHERE key = ?1"), [key])
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Where a schema records its version.
#[derive(Debug, Clone, Copy)]
pub enum Stamp<'a> {
    /// `schema_version` in the named meta table, created if missing.
    Meta(&'a str),
    /// `PRAGMA user_version`.
    UserVersion,
}

/// A forward-only schema at `version`. `what` names the database in the refusal, as in
/// "this mailbox is at schema 3, which is newer than this build understands (2)".
#[derive(Debug, Clone, Copy)]
pub struct Schema<'a> {
    pub stamp: Stamp<'a>,
    pub version: i32,
    pub what: &'a str,
}

impl Schema<'_> {
    pub fn version(&self, conn: &Connection) -> Result<i32, String> {
        match self.stamp {
            Stamp::Meta(table) => Ok(meta_get(conn, table, VERSION_KEY)?
                .and_then(|v| v.parse::<i32>().ok())
                .unwrap_or(0)),
            Stamp::UserVersion => conn
                .query_row("PRAGMA user_version", [], |row| row.get(0))
                .map_err(|e| e.to_string()),
        }
    }

    /// Runs `step(found)` inside one transaction and records the version, unless the database is
    /// already current. A database from a newer build is refused rather than misread.
    /// `immediate` chooses `BEGIN IMMEDIATE` over `BEGIN`.
    pub fn migrate(
        &self,
        conn: &Connection,
        immediate: bool,
        step: impl FnOnce(i32) -> Result<(), String>,
    ) -> Result<(), String> {
        if let Stamp::Meta(table) = self.stamp {
            conn.execute_batch(&format!(
                "CREATE TABLE IF NOT EXISTS {table} (key TEXT PRIMARY KEY, value TEXT NOT NULL);"
            ))
            .map_err(|e| e.to_string())?;
        }
        let found = self.version(conn)?;
        if found > self.version {
            return Err(format!(
                "{} is at schema {found}, which is newer than this build understands ({})",
                self.what, self.version
            ));
        }
        if found == self.version {
            return Ok(());
        }
        let tx = if immediate {
            Tx::begin(conn)?
        } else {
            Tx::deferred(conn)?
        };
        step(found)?;
        match self.stamp {
            Stamp::Meta(table) => meta_set(conn, table, VERSION_KEY, &self.version.to_string())?,
            Stamp::UserVersion => conn
                .execute_batch(&format!("PRAGMA user_version = {};", self.version))
                .map_err(|e| e.to_string())?,
        }
        tx.commit()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const V1: &str = "CREATE TABLE IF NOT EXISTS events (id TEXT PRIMARY KEY);";
    const V2: &str = "ALTER TABLE events ADD COLUMN color_id TEXT;";

    const SCHEMA: Schema = Schema {
        stamp: Stamp::Meta(META),
        version: 2,
        what: "this calendar database",
    };

    fn migrate(conn: &Connection) -> Result<(), String> {
        SCHEMA.migrate(conn, true, |found| {
            if found < 1 {
                conn.execute_batch(V1).map_err(|e| e.to_string())?;
            }
            if found < 2 {
                conn.execute_batch(V2).map_err(|e| e.to_string())?;
            }
            Ok(())
        })
    }

    #[test]
    fn migrate_runs_twice_without_complaint() {
        let conn = Connection::open_in_memory().expect("memory");
        migrate(&conn).expect("first migrate");
        migrate(&conn).expect("second migrate");
        let version: String = conn
            .query_row(
                "SELECT value FROM meta WHERE key = 'schema_version'",
                [],
                |r| r.get(0),
            )
            .expect("a recorded version");
        assert_eq!(version, "2");
    }

    #[test]
    fn migrate_refuses_a_database_from_a_newer_build() {
        let conn = Connection::open_in_memory().expect("memory");
        migrate(&conn).expect("migrate");
        meta_set(&conn, META, "schema_version", "99").expect("bump");
        assert_eq!(
            migrate(&conn),
            Err(
                "this calendar database is at schema 99, which is newer than this build understands (2)"
                    .to_string()
            )
        );
    }
}
