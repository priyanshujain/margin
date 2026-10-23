use std::collections::BTreeMap;

use margin_crypt::{CryptError, Key};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::error::SyncError;
use crate::record::Record;

pub const FORMAT: u32 = 1;

/// One device's own description of what it has published. Only that device ever writes it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub format: u32,
    pub device: String,
    #[serde(default)]
    pub name: String,
    pub created_ms: i64,
    pub updated_ms: i64,
    /// The last sequence this device has published, in segments or folded into its snapshot.
    pub high: u64,
    #[serde(default)]
    pub segments: Vec<SegmentRef>,
    #[serde(default)]
    pub snapshot: Option<SnapshotRef>,
    /// What this device had taken in from every device when it last wrote this.
    #[serde(default)]
    pub seen: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SegmentRef {
    pub first: u64,
    pub last: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotRef {
    pub ms: i64,
    pub covers: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Segment {
    pub format: u32,
    pub records: Vec<Record>,
}

/// Every winning record as of `covers`: for each device, every write up to that sequence is in
/// here or was beaten by something that is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub format: u32,
    pub covers: BTreeMap<String, u64>,
    pub records: Vec<Record>,
}

pub const DEVICES: &str = "devices";

pub fn manifest_key(device: &str) -> String {
    format!("{DEVICES}/{device}")
}

pub fn segment_key(device: &str, first: u64, last: u64) -> String {
    format!("records/{device}/{first:012}-{last:012}")
}

pub fn snapshot_key(device: &str, ms: i64) -> String {
    format!("snapshots/{device}/{ms:016}")
}

pub fn seal<T: Serialize>(key: &Key, name: &str, value: &T) -> Result<Vec<u8>, SyncError> {
    let plain = serde_json::to_vec(value).map_err(|e| SyncError::Corrupt {
        key: name.to_string(),
        detail: e.to_string(),
    })?;
    margin_crypt::seal(key, name, &plain).map_err(|e| SyncError::Corrupt {
        key: name.to_string(),
        detail: e.to_string(),
    })
}

pub fn open<T: DeserializeOwned>(key: &Key, name: &str, sealed: &[u8]) -> Result<T, SyncError> {
    let plain = margin_crypt::open(key, name, sealed).map_err(|e| match e {
        CryptError::Decrypt => SyncError::WrongKey {
            key: name.to_string(),
        },
        other => SyncError::Corrupt {
            key: name.to_string(),
            detail: other.to_string(),
        },
    })?;
    serde_json::from_slice(&plain).map_err(|e| SyncError::Corrupt {
        key: name.to_string(),
        detail: e.to_string(),
    })
}
