use std::collections::{BTreeMap, BTreeSet};

use margin_crypt::Key;
use objstore::{ETag, ObjectStore, PutMode};
use serde::{Deserialize, Serialize};

use crate::clock::now_ms;
use crate::error::SyncError;
use crate::record::Record;
use crate::replica::Replica;
use crate::wire::{
    self, DEVICES, FORMAT, Manifest, Segment, SegmentRef, Snapshot, SnapshotRef, manifest_key,
    segment_key, snapshot_key,
};

const STATE_KEY: &str = "margin-sync";

#[derive(Debug, Clone)]
pub struct Policy {
    pub segment_records: usize,
    /// Compact once this device has this many segments outstanding.
    pub max_segments: usize,
    /// Compact at most this often when something changed, so the store always has a recent
    /// snapshot to restore from.
    pub snapshot_every_ms: i64,
    pub keep_snapshots: usize,
}

impl Default for Policy {
    fn default() -> Policy {
        Policy {
            segment_records: 500,
            max_segments: 32,
            snapshot_every_ms: 24 * 60 * 60 * 1000,
            keep_snapshots: 3,
        }
    }
}

/// One app's (or one account's) records in one store: everything under `root`, sealed with `key`.
pub struct Space<'a> {
    store: &'a dyn ObjectStore,
    root: String,
    key: Key,
    device_name: String,
    store_id: String,
    policy: Policy,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    pub uploaded_segments: usize,
    pub uploaded_records: usize,
    pub downloaded_segments: usize,
    pub snapshots_loaded: usize,
    pub applied: usize,
    pub snapshot_written: bool,
    pub deleted: usize,
    pub peers: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub updated_ms: i64,
    pub high: u64,
    pub this_device: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct State {
    #[serde(default)]
    cursor: BTreeMap<String, u64>,
    #[serde(default)]
    published: u64,
    #[serde(default)]
    created_ms: i64,
    #[serde(default)]
    segments: Vec<SegmentRef>,
    #[serde(default)]
    snapshot: Option<SnapshotRef>,
    #[serde(default)]
    own_etag: Option<String>,
    #[serde(default)]
    peers: BTreeMap<String, Peer>,
    #[serde(default)]
    last_synced_ms: Option<i64>,
    #[serde(default)]
    changed_since_snapshot: bool,
    #[serde(default)]
    store_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Peer {
    etag: String,
    manifest: Manifest,
}

fn load(replica: &dyn Replica) -> Result<State, SyncError> {
    match replica.meta(STATE_KEY).map_err(replica_err)? {
        Some(text) => serde_json::from_str(&text).map_err(|e| SyncError::Corrupt {
            key: STATE_KEY.to_string(),
            detail: e.to_string(),
        }),
        None => Ok(State::default()),
    }
}

/// When this replica last finished a pass, against whichever store.
pub fn last_synced_ms(replica: &dyn Replica) -> Result<Option<i64>, SyncError> {
    Ok(load(replica)?.last_synced_ms)
}

fn replica_err(e: String) -> SyncError {
    SyncError::Replica(e)
}

impl<'a> Space<'a> {
    pub fn new(store: &'a dyn ObjectStore, root: &str, key: Key) -> Result<Space<'a>, SyncError> {
        let root = root.trim_matches('/').to_string();
        objstore::validate_key(&root)?;
        Ok(Space {
            store,
            root,
            key,
            device_name: String::new(),
            store_id: String::new(),
            policy: Policy::default(),
        })
    }

    /// Names the store this space lives in. When it changes, everything this replica believed
    /// about the old store is dropped and the next pass publishes afresh.
    pub fn with_store_id(mut self, id: impl Into<String>) -> Space<'a> {
        self.store_id = id.into();
        self
    }

    pub fn with_device_name(mut self, name: impl Into<String>) -> Space<'a> {
        self.device_name = name.into();
        self
    }

    pub fn with_policy(mut self, policy: Policy) -> Space<'a> {
        self.policy = policy;
        self
    }

    pub fn root(&self) -> &str {
        &self.root
    }

    fn full(&self, rel: &str) -> String {
        format!("{}/{rel}", self.root)
    }

    fn load(&self, replica: &dyn Replica) -> Result<State, SyncError> {
        let mut state = load(replica)?;
        if state.store_id != self.store_id {
            state = State {
                cursor: state.cursor,
                created_ms: state.created_ms,
                store_id: self.store_id.clone(),
                ..State::default()
            };
        }
        Ok(state)
    }

    fn save(&self, replica: &dyn Replica, state: &State) -> Result<(), SyncError> {
        let text = serde_json::to_string(state).map_err(|e| SyncError::Replica(e.to_string()))?;
        replica.set_meta(STATE_KEY, &text).map_err(replica_err)
    }

    /// Opens every device manifest in the store, so a wrong key is caught before anything is
    /// written. Returns how many devices are there.
    pub async fn check_key(&self) -> Result<usize, SyncError> {
        let listing = self.store.list(&self.full(&format!("{DEVICES}/"))).await?;
        let mut count = 0;
        for meta in listing {
            let object = self.store.get(&meta.key).await?;
            let _: Manifest = wire::open(&self.key, &meta.key, &object.bytes)?;
            count += 1;
        }
        Ok(count)
    }

    /// One list request: whether another device has published since the last pass, or this one
    /// has something to publish. Cheap enough to run on every focus and every half minute.
    pub async fn poll(&self, replica: &dyn Replica) -> Result<bool, SyncError> {
        let mut state = self.load(replica)?;
        let me = replica.device_id().map_err(replica_err)?;
        let high = replica.high_water().map_err(replica_err)?;
        if high.get(&me).copied().unwrap_or(0) > state.published {
            return Ok(true);
        }
        let prefix = self.full(&format!("{DEVICES}/"));
        let listing = self.store.list(&prefix).await?;
        let mut own_listed = false;
        for meta in listing {
            let Some(device) = meta.key.strip_prefix(&prefix) else {
                continue;
            };
            if device.contains('/') {
                continue;
            }
            if device == me {
                own_listed = true;
                continue;
            }
            match state.peers.get(device) {
                Some(peer) if peer.etag == meta.etag.as_str() => {}
                _ => return Ok(true),
            }
        }
        if !own_listed {
            return Ok(true);
        }
        state.last_synced_ms = Some(now_ms());
        self.save(replica, &state)?;
        Ok(false)
    }

    /// The device registry as of the last pass.
    pub fn devices(&self, replica: &dyn Replica) -> Result<Vec<DeviceInfo>, SyncError> {
        let state = self.load(replica)?;
        let me = replica.device_id().map_err(replica_err)?;
        let mut out = vec![DeviceInfo {
            id: me,
            name: self.device_name.clone(),
            updated_ms: state.last_synced_ms.unwrap_or_default(),
            high: state.published,
            this_device: true,
        }];
        out.extend(state.peers.values().map(|peer| DeviceInfo {
            id: peer.manifest.device.clone(),
            name: peer.manifest.name.clone(),
            updated_ms: peer.manifest.updated_ms,
            high: peer.manifest.high,
            this_device: false,
        }));
        Ok(out)
    }

    /// A full pass: take in what every other device published, publish what this one has not,
    /// and compact when due. Safe to run at any time and to interrupt at any point.
    pub async fn sync(&self, replica: &dyn Replica) -> Result<Report, SyncError> {
        let mut attempt = 0;
        loop {
            match self.pass(replica).await {
                Err(SyncError::Store(objstore::Error::NotFound { .. })) if attempt < 2 => {
                    attempt += 1;
                }
                other => return other,
            }
        }
    }

    async fn pass(&self, replica: &dyn Replica) -> Result<Report, SyncError> {
        let me = replica.device_id().map_err(replica_err)?;
        let mut state = self.load(replica)?;
        let mut report = Report::default();
        if state.created_ms == 0 {
            state.created_ms = now_ms();
        }
        let held = replica.high_water().map_err(replica_err)?;
        for (device, seq) in &held {
            if device != &me {
                let at = state.cursor.entry(device.clone()).or_default();
                *at = (*at).max(*seq);
            }
        }

        let prefix = self.full(&format!("{DEVICES}/"));
        let listing = self.store.list(&prefix).await?;
        let mut own_remote: Option<ETag> = None;
        let mut peers: BTreeMap<String, Peer> = BTreeMap::new();
        for meta in listing {
            let Some(device) = meta.key.strip_prefix(&prefix) else {
                continue;
            };
            if device.contains('/') {
                continue;
            }
            if device == me {
                own_remote = Some(meta.etag.clone());
                continue;
            }
            let peer = match state.peers.get(device) {
                Some(peer) if peer.etag == meta.etag.as_str() => peer.clone(),
                _ => {
                    let manifest = self.read_manifest(&meta.key, device).await?;
                    Peer {
                        etag: meta.etag.as_str().to_string(),
                        manifest,
                    }
                }
            };
            peers.insert(device.to_string(), peer);
        }
        report.peers = peers.len();

        if let Some(etag) = &own_remote
            && state.own_etag.as_deref() != Some(etag.as_str())
        {
            let remote = self
                .read_manifest(&self.full(&manifest_key(&me)), &me)
                .await?;
            if remote.high > held.get(&me).copied().unwrap_or(0) {
                return Err(SyncError::DeviceClash { device: me });
            }
            let snapshot_held = match &remote.snapshot {
                Some(snap) => self
                    .store
                    .head(&self.full(&snapshot_key(&me, snap.ms)))
                    .await
                    .is_ok(),
                None => true,
            };
            if remote.high > state.published && snapshot_held {
                self.verify_own(replica, &me, state.published, &remote)
                    .await?;
                state.published = remote.high;
                state.segments = remote.segments.clone();
                state.snapshot = remote.snapshot.clone();
            }
            state.own_etag = Some(etag.as_str().to_string());
        }

        report.applied += self
            .catch_up(replica, &me, &mut state, &peers, &mut report)
            .await?;
        state.peers = peers;
        self.save(replica, &state)?;

        let mine = replica.export(&me, state.published).map_err(replica_err)?;
        let size = self.policy.segment_records.max(1);
        if !mine.is_empty() || report.applied > 0 {
            state.changed_since_snapshot = true;
        }
        let now = now_ms();
        let only_here = state.cursor.iter().any(|(device, seq)| {
            *seq > state.peers.get(device).map_or(0, |peer| peer.manifest.high)
        });
        let has_data = !mine.is_empty() || state.published > 0 || only_here;
        let due = state.segments.len() + mine.len().div_ceil(size) > self.policy.max_segments
            || (has_data
                && state.snapshot.as_ref().is_none_or(|snap| {
                    state.changed_since_snapshot && now - snap.ms >= self.policy.snapshot_every_ms
                }));

        if due {
            if let Some(last) = mine.last() {
                state.published = last.seq;
                report.uploaded_records += mine.len();
            }
        } else {
            for chunk in mine.chunks(size) {
                let (Some(first), Some(last)) = (chunk.first(), chunk.last()) else {
                    continue;
                };
                self.put_segment(&me, first.seq, last.seq, chunk).await?;
                state.segments.push(SegmentRef {
                    first: first.seq,
                    last: last.seq,
                });
                state.published = last.seq;
                report.uploaded_segments += 1;
                report.uploaded_records += chunk.len();
            }
        }

        let mut retired: Vec<SegmentRef> = Vec::new();
        if due {
            let ms = state
                .snapshot
                .as_ref()
                .map_or(now, |snap| now.max(snap.ms + 1));
            let mut covers = state.cursor.clone();
            covers.retain(|_, seq| *seq > 0);
            if state.published > 0 {
                covers.insert(me.clone(), state.published);
            }
            let snapshot = Snapshot {
                format: FORMAT,
                covers: covers.clone(),
                records: replica.compacted().map_err(replica_err)?,
            };
            let key = self.full(&snapshot_key(&me, ms));
            let sealed = wire::seal(&self.key, &key, &snapshot)?;
            self.store.put(&key, sealed, PutMode::Overwrite).await?;
            state.snapshot = Some(SnapshotRef { ms, covers });
            retired = std::mem::take(&mut state.segments);
            state.changed_since_snapshot = false;
            report.snapshot_written = true;
        }

        if report.uploaded_segments > 0 || due || own_remote.is_none() {
            self.write_manifest(&me, &mut state, own_remote.as_ref(), now)
                .await?;
        }
        state.last_synced_ms = Some(now);
        self.save(replica, &state)?;

        if due {
            report.deleted += self.retire(&me, &retired, &state).await;
        }
        Ok(report)
    }

    /// The store says this device published more than this replica remembers: either a pass
    /// ended between writing the manifest and saving its state, or another installation shares
    /// the device id. Only the first leaves segments that match the local log.
    async fn verify_own(
        &self,
        replica: &dyn Replica,
        me: &str,
        after: u64,
        remote: &Manifest,
    ) -> Result<(), SyncError> {
        let local = replica.export(me, after).map_err(replica_err)?;
        for segment in remote.segments.iter().filter(|s| s.last > after) {
            let key = self.full(&segment_key(me, segment.first, segment.last));
            let object = self.store.get(&key).await?;
            let held: Segment = wire::open(&self.key, &key, &object.bytes)?;
            for record in held.records.iter().filter(|r| r.seq > after) {
                if local.iter().find(|l| l.seq == record.seq) != Some(record) {
                    return Err(SyncError::DeviceClash {
                        device: me.to_string(),
                    });
                }
            }
        }
        Ok(())
    }

    async fn read_manifest(&self, key: &str, device: &str) -> Result<Manifest, SyncError> {
        let object = self.store.get(key).await?;
        let manifest: Manifest = wire::open(&self.key, key, &object.bytes)?;
        if manifest.device != device {
            return Err(SyncError::Corrupt {
                key: key.to_string(),
                detail: format!("it describes device {}", manifest.device),
            });
        }
        Ok(manifest)
    }

    async fn catch_up(
        &self,
        replica: &dyn Replica,
        me: &str,
        state: &mut State,
        peers: &BTreeMap<String, Peer>,
        report: &mut Report,
    ) -> Result<usize, SyncError> {
        let mut applied = 0;
        let behind = |state: &State, m: &Manifest| {
            m.high > state.cursor.get(&m.device).copied().unwrap_or(0)
        };
        let gap = |state: &State, m: &Manifest| {
            let at = state.cursor.get(&m.device).copied().unwrap_or(0);
            if m.high <= at {
                return false;
            }
            let mut next = at + 1;
            let mut segments: Vec<&SegmentRef> =
                m.segments.iter().filter(|s| s.last > at).collect();
            segments.sort_by_key(|s| s.first);
            for segment in segments {
                if segment.first > next {
                    return true;
                }
                next = next.max(segment.last + 1);
            }
            next <= m.high
        };

        let fresh = !peers.is_empty()
            && peers
                .keys()
                .all(|d| state.cursor.get(d).copied().unwrap_or(0) == 0);
        let any_gap = peers.values().any(|p| gap(state, &p.manifest));
        let mut loaded: BTreeSet<String> = BTreeSet::new();

        if fresh || any_gap {
            let best = peers
                .values()
                .filter_map(|p| {
                    p.manifest
                        .snapshot
                        .as_ref()
                        .map(|s| (&p.manifest.device, s))
                })
                .max_by_key(|(_, s)| (s.covers.values().sum::<u64>(), s.ms));
            if let Some((device, snap)) = best {
                applied += self.load_snapshot(replica, me, state, device, snap).await?;
                loaded.insert(device.clone());
                report.snapshots_loaded += 1;
            }
        }
        for peer in peers.values() {
            let m = &peer.manifest;
            if gap(state, m)
                && !loaded.contains(&m.device)
                && let Some(snap) = &m.snapshot
            {
                applied += self
                    .load_snapshot(replica, me, state, &m.device, snap)
                    .await?;
                loaded.insert(m.device.clone());
                report.snapshots_loaded += 1;
            }
            if gap(state, m) {
                return Err(SyncError::Corrupt {
                    key: self.full(&manifest_key(&m.device)),
                    detail: "records it lists are missing from the store".to_string(),
                });
            }
        }

        for peer in peers.values() {
            let m = &peer.manifest;
            if !behind(state, m) {
                continue;
            }
            let mut segments: Vec<&SegmentRef> = m.segments.iter().collect();
            segments.sort_by_key(|s| s.first);
            for segment in segments {
                let at = state.cursor.get(&m.device).copied().unwrap_or(0);
                if segment.last <= at {
                    continue;
                }
                let key = self.full(&segment_key(&m.device, segment.first, segment.last));
                let object = self.store.get(&key).await?;
                let body: Segment = wire::open(&self.key, &key, &object.bytes)?;
                let fresh: Vec<Record> = body
                    .records
                    .into_iter()
                    .filter(|r| r.device == m.device && r.seq > at)
                    .collect();
                applied += replica.absorb(&fresh).map_err(replica_err)?;
                state.cursor.insert(m.device.clone(), segment.last);
                report.downloaded_segments += 1;
            }
            self.save(replica, state)?;
        }
        Ok(applied)
    }

    async fn load_snapshot(
        &self,
        replica: &dyn Replica,
        me: &str,
        state: &mut State,
        device: &str,
        snap: &SnapshotRef,
    ) -> Result<usize, SyncError> {
        let key = self.full(&snapshot_key(device, snap.ms));
        let object = self.store.get(&key).await?;
        let body: Snapshot = wire::open(&self.key, &key, &object.bytes)?;
        let applied = replica.absorb(&body.records).map_err(replica_err)?;
        for (d, seq) in &body.covers {
            if d != me {
                let at = state.cursor.entry(d.clone()).or_default();
                *at = (*at).max(*seq);
            }
        }
        self.save(replica, state)?;
        Ok(applied)
    }

    async fn put_segment(
        &self,
        me: &str,
        first: u64,
        last: u64,
        records: &[Record],
    ) -> Result<(), SyncError> {
        let key = self.full(&segment_key(me, first, last));
        let body = Segment {
            format: FORMAT,
            records: records.to_vec(),
        };
        let sealed = wire::seal(&self.key, &key, &body)?;
        if !self.store.capabilities().conditional_put {
            self.store.put(&key, sealed, PutMode::Overwrite).await?;
            return Ok(());
        }
        match self.store.put(&key, sealed, PutMode::IfNoneMatch).await {
            Ok(_) => Ok(()),
            Err(objstore::Error::PreconditionFailed { .. }) => {
                let object = self.store.get(&key).await?;
                let held: Segment = wire::open(&self.key, &key, &object.bytes)?;
                if held.records == records {
                    Ok(())
                } else {
                    Err(SyncError::DeviceClash {
                        device: me.to_string(),
                    })
                }
            }
            Err(e) => Err(e.into()),
        }
    }

    async fn write_manifest(
        &self,
        me: &str,
        state: &mut State,
        own_remote: Option<&ETag>,
        now: i64,
    ) -> Result<(), SyncError> {
        let manifest = Manifest {
            format: FORMAT,
            device: me.to_string(),
            name: self.device_name.clone(),
            created_ms: state.created_ms,
            updated_ms: now,
            high: state.published,
            segments: state.segments.clone(),
            snapshot: state.snapshot.clone(),
            seen: state.cursor.clone(),
        };
        let key = self.full(&manifest_key(me));
        let sealed = wire::seal(&self.key, &key, &manifest)?;
        let mode = if self.store.capabilities().conditional_put {
            match own_remote {
                Some(etag) => PutMode::IfMatch(etag.clone()),
                None => PutMode::IfNoneMatch,
            }
        } else {
            PutMode::Overwrite
        };
        match self.store.put(&key, sealed, mode).await {
            Ok(meta) => {
                state.own_etag = Some(meta.etag.as_str().to_string());
                Ok(())
            }
            Err(objstore::Error::PreconditionFailed { .. }) => Err(SyncError::DeviceClash {
                device: me.to_string(),
            }),
            Err(e) => Err(e.into()),
        }
    }

    /// Deletes this device's own segments folded into its snapshot and snapshots past the ones
    /// kept. Failures are left for the next compaction: nothing reads a retired object.
    async fn retire(&self, me: &str, segments: &[SegmentRef], state: &State) -> usize {
        let mut deleted = 0;
        for segment in segments {
            let key = self.full(&segment_key(me, segment.first, segment.last));
            if self.store.delete(&key).await.is_ok() {
                deleted += 1;
            }
        }
        let prefix = self.full(&format!("snapshots/{me}/"));
        let Ok(mut held) = self.store.list(&prefix).await else {
            return deleted;
        };
        held.sort_by(|a, b| b.key.cmp(&a.key));
        let current = state
            .snapshot
            .as_ref()
            .map(|snap| self.full(&snapshot_key(me, snap.ms)));
        for old in held.iter().skip(self.policy.keep_snapshots.max(1)) {
            if Some(&old.key) == current.as_ref() {
                continue;
            }
            if self.store.delete(&old.key).await.is_ok() {
                deleted += 1;
            }
        }
        deleted
    }
}
