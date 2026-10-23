use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::clock::Hlc;

/// One write to one field of one record, as it travels between devices.
///
/// `(device, seq)` names it uniquely: every device numbers its own writes from 1 and never reuses a
/// number. `value: None` is a tombstone. An app may use one field per record (`field` empty, the
/// value being the whole record) or one per column; the merge does not care.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub device: String,
    pub seq: u64,
    pub ms: i64,
    #[serde(default)]
    pub counter: u32,
    pub kind: String,
    pub id: String,
    #[serde(default)]
    pub field: String,
    #[serde(default)]
    pub value: Option<String>,
}

pub type Slot = (String, String, String);

impl Record {
    pub fn slot(&self) -> Slot {
        (self.kind.clone(), self.id.clone(), self.field.clone())
    }

    pub fn clock(&self) -> Hlc {
        Hlc::new(self.ms, self.counter)
    }

    /// The total order last writer wins is defined by. The device breaks ties between devices, so
    /// the winner depends on the writes and not on the order they arrived in; the sequence breaks
    /// ties inside one device.
    pub fn order(&self) -> (i64, u32, &str, u64) {
        (self.ms, self.counter, self.device.as_str(), self.seq)
    }

    pub fn beats(&self, other: &Record) -> bool {
        self.order() > other.order()
    }
}

/// The winning write per `(kind, id, field)`. Offering the same writes in any order, any number of
/// times, leaves the same winners.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Winners(BTreeMap<Slot, Record>);

impl Winners {
    pub fn new() -> Winners {
        Winners::default()
    }

    /// Whether the record became the winner of its slot.
    pub fn offer(&mut self, record: Record) -> bool {
        let slot = record.slot();
        match self.0.get(&slot) {
            Some(held) if !record.beats(held) => false,
            _ => {
                self.0.insert(slot, record);
                true
            }
        }
    }

    pub fn get(&self, kind: &str, id: &str, field: &str) -> Option<&Record> {
        self.0
            .get(&(kind.to_string(), id.to_string(), field.to_string()))
    }

    pub fn records(&self) -> impl Iterator<Item = &Record> {
        self.0.values()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(device: &str, seq: u64, ms: i64, value: Option<&str>) -> Record {
        Record {
            device: device.into(),
            seq,
            ms,
            counter: 0,
            kind: "note".into(),
            id: "n1".into(),
            field: "body".into(),
            value: value.map(Into::into),
        }
    }

    #[test]
    fn the_later_write_wins_whatever_order_they_arrive_in() {
        let early = write("a", 1, 100, Some("first"));
        let late = write("b", 1, 200, Some("second"));
        for order in [[early.clone(), late.clone()], [late.clone(), early.clone()]] {
            let mut winners = Winners::new();
            for record in order {
                winners.offer(record);
            }
            assert_eq!(winners.get("note", "n1", "body"), Some(&late));
        }
    }

    #[test]
    fn a_tie_in_time_is_broken_by_device_then_sequence() {
        let a = write("a", 9, 100, Some("a"));
        let b = write("b", 1, 100, Some("b"));
        assert!(b.beats(&a));
        assert!(write("a", 2, 100, None).beats(&write("a", 1, 100, None)));
    }

    #[test]
    fn a_tombstone_wins_over_an_older_write_that_arrives_after_it() {
        let mut winners = Winners::new();
        winners.offer(write("b", 2, 300, None));
        assert!(!winners.offer(write("a", 1, 200, Some("resurrected"))));
        assert_eq!(winners.get("note", "n1", "body").unwrap().value, None);
    }

    #[test]
    fn fields_of_one_record_merge_independently() {
        let mut winners = Winners::new();
        let mut title = write("a", 1, 500, Some("title from a"));
        title.field = "title".into();
        winners.offer(title.clone());
        winners.offer(write("b", 1, 100, Some("body from b")));
        assert_eq!(winners.len(), 2);
        assert_eq!(winners.get("note", "n1", "title"), Some(&title));
    }

    #[test]
    fn older_json_without_optional_fields_still_reads() {
        let record: Record =
            serde_json::from_str(r#"{"device":"a","seq":1,"ms":5,"kind":"k","id":"i"}"#).unwrap();
        assert_eq!(record.counter, 0);
        assert_eq!(record.field, "");
        assert_eq!(record.value, None);
    }
}
