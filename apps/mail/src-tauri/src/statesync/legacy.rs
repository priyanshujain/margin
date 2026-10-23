// Mail's backups from before margin-sync: `<account-hash>/<device>/<first>-<last>.seg`, sealed
// MMB1 under a key derived from Mail's own phrase. R2 wrote them at the bucket root, Drive under
// `margin/mail/`. They are read, never written or deleted, so older builds on other devices keep
// working and the source stays until the owner removes it.

use std::collections::BTreeMap;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use margin_sync::{Key, Replica};
use objstore::ObjectStore;
use sha2::{Digest, Sha256};

use super::replica::to_sync;
use crate::secrets;
use crate::state::merge;

const SUFFIX: &str = ".seg";
const KEY_ID: &str = "margin-mail backup key";
const R2_ID: &str = "margin-mail r2 credentials";

/// The same on every device, so a second one finds the first one's folder from the address.
pub fn account_hash(email: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"margin-mail backup account v1");
    hasher.update(email.trim().to_lowercase().as_bytes());
    hasher.finalize()[..16]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub fn stored_key() -> Result<Option<Key>, String> {
    let Some(held) = secrets::load(KEY_ID)? else {
        return Ok(None);
    };
    let raw = BASE64
        .decode(held)
        .map_err(|e| format!("the stored backup key is malformed: {e}"))?;
    let bytes: [u8; 32] = raw
        .try_into()
        .map_err(|_| "the stored backup key is not 32 bytes".to_string())?;
    Ok(Some(Key::from_bytes(bytes)))
}

pub fn remember_key(key: &Key) -> Result<(), String> {
    secrets::store(KEY_ID, &BASE64.encode(key.as_bytes()))
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct R2 {
    pub endpoint: String,
    pub bucket: String,
    pub access_key: String,
    pub secret: String,
}

pub fn stored_r2() -> Result<Option<R2>, String> {
    match secrets::load(R2_ID)? {
        Some(json) => serde_json::from_str(&json)
            .map(Some)
            .map_err(|e| format!("the stored R2 credentials are malformed: {e}")),
        None => Ok(None),
    }
}

fn parse(rest: &str) -> Option<(&str, &str, i64)> {
    let (device, file) = rest.split_once('/')?;
    if file.contains('/') {
        return None;
    }
    let (_, last) = file.strip_suffix(SUFFIX)?.split_once('-')?;
    Some((device, file, last.parse().ok()?))
}

/// Takes in every legacy segment under `prefix` (`<hash>/` or `mail/<hash>/`) that holds records
/// this account does not have yet. Returns how many records changed a table.
pub async fn import<R: Replica>(
    store: &dyn ObjectStore,
    prefix: &str,
    hash: &str,
    key: &Key,
    replica: &R,
) -> Result<usize, String> {
    let me = replica.device_id()?;
    let high: BTreeMap<String, u64> = replica.high_water()?;
    let mut wanted: Vec<(String, String)> = Vec::new();
    for meta in store.list(prefix).await.map_err(|e| e.to_string())? {
        let Some(rest) = meta.key.strip_prefix(prefix) else {
            continue;
        };
        let Some((device, file, last)) = parse(rest) else {
            continue;
        };
        if device == me || last as u64 <= high.get(device).copied().unwrap_or(0) {
            continue;
        }
        wanted.push((meta.key.clone(), format!("{hash}/{device}/{file}")));
    }
    wanted.sort();
    let mut applied = 0;
    for (key_in_store, name) in wanted {
        let object = store.get(&key_in_store).await.map_err(|e| e.to_string())?;
        let plain = margin_crypt::open(key, &name, &object.bytes).map_err(|_| {
            format!("{name} could not be decrypted. The recovery phrase is not the one this backup was made with, or the file was changed.")
        })?;
        let text = String::from_utf8(plain).map_err(|_| format!("{name} is not a segment"))?;
        let records: Vec<_> = merge::decode(&text)?.into_iter().map(to_sync).collect();
        applied += replica.absorb(&records)?;
    }
    Ok(applied)
}
