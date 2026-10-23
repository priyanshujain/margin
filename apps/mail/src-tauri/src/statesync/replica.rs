use std::collections::BTreeMap;

use margin_sync::{Record, Replica};
use rusqlite::Connection;

use crate::db::Db;
use crate::state::{device, journal, merge};

/// One account's state database as a sync replica. The journal is already a per-device log with
/// last-writer-wins by `(at_ms, device, seq)`, so the mapping is one to one: `kind` and `key` are
/// the record's kind and id, the field is empty and the value is the whole payload.
pub trait Journal: Sync {
    fn with<T, F: FnOnce(&Connection) -> Result<T, String>>(&self, f: F) -> Result<T, String>;
}

pub struct Account<'a> {
    pub db: &'a Db,
    pub account_id: &'a str,
}

impl Journal for Account<'_> {
    fn with<T, F: FnOnce(&Connection) -> Result<T, String>>(&self, f: F) -> Result<T, String> {
        self.db.with(self.account_id, f)
    }
}

pub struct StateReplica<J>(pub J);

pub fn to_sync(record: journal::Record) -> Record {
    Record {
        device: record.device_id,
        seq: record.seq.max(0) as u64,
        ms: record.at_ms,
        counter: 0,
        kind: record.kind,
        id: record.key,
        field: String::new(),
        value: Some(record.payload),
    }
}

pub fn to_journal(record: &Record) -> journal::Record {
    journal::Record {
        device_id: record.device.clone(),
        seq: record.seq as i64,
        at_ms: record.ms,
        kind: record.kind.clone(),
        key: record.id.clone(),
        payload: record.value.clone().unwrap_or_else(|| "{}".to_string()),
    }
}

fn winners(conn: &Connection) -> Result<Vec<journal::Record>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT device_id, seq, at_ms, kind, key, payload FROM (
                 SELECT *, ROW_NUMBER() OVER (
                     PARTITION BY kind, key ORDER BY at_ms DESC, device_id DESC, seq DESC
                 ) AS rank FROM state.journal
             ) WHERE rank = 1 ORDER BY at_ms ASC, device_id ASC, seq ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(journal::Record {
                device_id: row.get(0)?,
                seq: row.get(1)?,
                at_ms: row.get(2)?,
                kind: row.get(3)?,
                key: row.get(4)?,
                payload: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

impl<J: Journal> Replica for StateReplica<J> {
    fn device_id(&self) -> Result<String, String> {
        self.0.with(device::device_id)
    }

    fn high_water(&self) -> Result<BTreeMap<String, u64>, String> {
        Ok(self
            .0
            .with(merge::high_water)?
            .into_iter()
            .map(|(device, seq)| (device, seq.max(0) as u64))
            .collect())
    }

    fn export(&self, device: &str, after: u64) -> Result<Vec<Record>, String> {
        Ok(self
            .0
            .with(|conn| merge::export(conn, device, after as i64))?
            .into_iter()
            .map(to_sync)
            .collect())
    }

    fn absorb(&self, records: &[Record]) -> Result<usize, String> {
        let records: Vec<journal::Record> = records.iter().map(to_journal).collect();
        Ok(self.0.with(|conn| merge::absorb(conn, &records))?.applied)
    }

    fn compacted(&self) -> Result<Vec<Record>, String> {
        Ok(self.0.with(winners)?.into_iter().map(to_sync).collect())
    }

    fn meta(&self, key: &str) -> Result<Option<String>, String> {
        self.0.with(|conn| device::meta_get(conn, key))
    }

    fn set_meta(&self, key: &str, value: &str) -> Result<(), String> {
        self.0.with(|conn| device::meta_set(conn, key, value))
    }
}
