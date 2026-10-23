use std::collections::BTreeMap;
use std::sync::Mutex;

use crate::clock::{Hlc, now_ms};
use crate::record::{Record, Winners};

/// The local side of sync: an app's own database, seen as a log of records.
///
/// The engine never holds a borrow across a network call, so every method is a short synchronous
/// step an app can run against its database connection.
pub trait Replica: Sync {
    /// Stable for the life of the installation and unique among the person's devices.
    fn device_id(&self) -> Result<String, String>;

    /// The highest sequence held for each device, this one included.
    fn high_water(&self) -> Result<BTreeMap<String, u64>, String>;

    /// This device's own records above a sequence, in sequence order.
    fn export(&self, device: &str, after: u64) -> Result<Vec<Record>, String>;

    /// Takes in records from elsewhere, idempotently. Returns how many changed what the app shows.
    fn absorb(&self, records: &[Record]) -> Result<usize, String>;

    /// The winning record of every slot, tombstones included: what a snapshot holds.
    fn compacted(&self) -> Result<Vec<Record>, String>;

    fn meta(&self, key: &str) -> Result<Option<String>, String>;

    fn set_meta(&self, key: &str, value: &str) -> Result<(), String>;
}

/// A complete replica in memory: the log, its winners and a hybrid logical clock. The reference
/// implementation the engine is tested against, and usable as-is by an app whose records fit in
/// memory and which persists `log()` itself.
pub struct MemoryReplica {
    device: String,
    inner: Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    log: BTreeMap<(String, u64), Record>,
    winners: Winners,
    clock: Hlc,
    meta: BTreeMap<String, String>,
}

impl MemoryReplica {
    pub fn new(device: impl Into<String>) -> MemoryReplica {
        MemoryReplica {
            device: device.into(),
            inner: Mutex::new(Inner::default()),
        }
    }

    pub fn set(&self, kind: &str, id: &str, field: &str, value: Option<&str>) -> Record {
        self.set_at(now_ms(), kind, id, field, value)
    }

    /// A local write at a given wall time, for tests that need two devices in one millisecond or a
    /// device whose clock is wrong.
    pub fn set_at(
        &self,
        wall_ms: i64,
        kind: &str,
        id: &str,
        field: &str,
        value: Option<&str>,
    ) -> Record {
        let mut inner = self.inner.lock().expect("replica lock");
        let stamp = inner.clock.tick(wall_ms);
        let seq = inner
            .log
            .range((self.device.clone(), 0)..=(self.device.clone(), u64::MAX))
            .next_back()
            .map(|((_, seq), _)| seq + 1)
            .unwrap_or(1);
        let record = Record {
            device: self.device.clone(),
            seq,
            ms: stamp.ms,
            counter: stamp.counter,
            kind: kind.into(),
            id: id.into(),
            field: field.into(),
            value: value.map(Into::into),
        };
        inner
            .log
            .insert((record.device.clone(), seq), record.clone());
        inner.winners.offer(record.clone());
        record
    }

    pub fn get(&self, kind: &str, id: &str, field: &str) -> Option<String> {
        let inner = self.inner.lock().expect("replica lock");
        inner
            .winners
            .get(kind, id, field)
            .and_then(|record| record.value.clone())
    }

    /// Every live value, as `(kind, id, field) = value`, for comparing replicas.
    pub fn view(&self) -> BTreeMap<(String, String, String), String> {
        let inner = self.inner.lock().expect("replica lock");
        inner
            .winners
            .records()
            .filter_map(|record| record.value.clone().map(|value| (record.slot(), value)))
            .collect()
    }

    pub fn log(&self) -> Vec<Record> {
        self.inner
            .lock()
            .expect("replica lock")
            .log
            .values()
            .cloned()
            .collect()
    }
}

impl Replica for MemoryReplica {
    fn device_id(&self) -> Result<String, String> {
        Ok(self.device.clone())
    }

    fn high_water(&self) -> Result<BTreeMap<String, u64>, String> {
        let inner = self.inner.lock().map_err(|e| e.to_string())?;
        let mut out = BTreeMap::new();
        for (device, seq) in inner.log.keys() {
            out.insert(device.clone(), *seq);
        }
        Ok(out)
    }

    fn export(&self, device: &str, after: u64) -> Result<Vec<Record>, String> {
        let inner = self.inner.lock().map_err(|e| e.to_string())?;
        Ok(inner
            .log
            .range((device.to_string(), after + 1)..=(device.to_string(), u64::MAX))
            .map(|(_, record)| record.clone())
            .collect())
    }

    fn absorb(&self, records: &[Record]) -> Result<usize, String> {
        let mut inner = self.inner.lock().map_err(|e| e.to_string())?;
        let mut changed = 0;
        for record in records {
            let at = (record.device.clone(), record.seq);
            if inner.log.contains_key(&at) {
                continue;
            }
            inner.clock.observe(record.clock());
            inner.log.insert(at, record.clone());
            if inner.winners.offer(record.clone()) {
                changed += 1;
            }
        }
        Ok(changed)
    }

    fn compacted(&self) -> Result<Vec<Record>, String> {
        let inner = self.inner.lock().map_err(|e| e.to_string())?;
        Ok(inner.winners.records().cloned().collect())
    }

    fn meta(&self, key: &str) -> Result<Option<String>, String> {
        let inner = self.inner.lock().map_err(|e| e.to_string())?;
        Ok(inner.meta.get(key).cloned())
    }

    fn set_meta(&self, key: &str, value: &str) -> Result<(), String> {
        let mut inner = self.inner.lock().map_err(|e| e.to_string())?;
        inner.meta.insert(key.to_string(), value.to_string());
        Ok(())
    }
}
