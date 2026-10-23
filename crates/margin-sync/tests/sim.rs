use std::collections::BTreeSet;
use std::sync::Mutex;

use margin_sync::memory::MemoryStore;
use margin_sync::{Key, MemoryReplica, Phrase, Policy, Replica, RootKey, Space, SyncError};
use objstore::{BoxFuture, Capabilities, ListPage, Object, ObjectMeta, ObjectStore, PutMode};

const ROOT: &str = "notes/space";

fn key() -> Key {
    Key::from_bytes([42; 32])
}

fn space<'a>(store: &'a dyn ObjectStore, name: &str) -> Space<'a> {
    Space::new(store, ROOT, key())
        .unwrap()
        .with_device_name(name)
}

fn small(store: &dyn ObjectStore) -> Space<'_> {
    space(store, "small").with_policy(Policy {
        segment_records: 10,
        max_segments: 3,
        snapshot_every_ms: i64::MAX,
        keep_snapshots: 2,
    })
}

struct Stores {
    _dir: tempfile::TempDir,
    fs: objstore_fs::FsStore,
    drive: MemoryStore,
    conditional: MemoryStore,
}

fn stores() -> Stores {
    let dir = tempfile::tempdir().unwrap();
    let fs = objstore_fs::FsStore::new(dir.path().join("store")).unwrap();
    Stores {
        _dir: dir,
        fs,
        drive: MemoryStore::drive_like(),
        conditional: MemoryStore::new(),
    }
}

impl Stores {
    fn all(&self) -> [(&'static str, &dyn ObjectStore); 3] {
        [
            ("fs", &self.fs),
            ("drive-like", &self.drive),
            ("memory", &self.conditional),
        ]
    }
}

/// Records which device wrote which key, so a test can prove no object has two writers.
struct Watched<'a> {
    inner: &'a dyn ObjectStore,
    device: String,
    writes: &'a Mutex<Vec<(String, String)>>,
}

impl ObjectStore for Watched<'_> {
    fn capabilities(&self) -> Capabilities {
        self.inner.capabilities()
    }
    fn put<'b>(
        &'b self,
        key: &'b str,
        bytes: Vec<u8>,
        mode: PutMode,
    ) -> BoxFuture<'b, objstore::Result<ObjectMeta>> {
        self.writes
            .lock()
            .unwrap()
            .push((key.to_string(), self.device.clone()));
        self.inner.put(key, bytes, mode)
    }
    fn get<'b>(&'b self, key: &'b str) -> BoxFuture<'b, objstore::Result<Object>> {
        self.inner.get(key)
    }
    fn head<'b>(&'b self, key: &'b str) -> BoxFuture<'b, objstore::Result<ObjectMeta>> {
        self.inner.head(key)
    }
    fn delete<'b>(&'b self, key: &'b str) -> BoxFuture<'b, objstore::Result<()>> {
        self.writes
            .lock()
            .unwrap()
            .push((key.to_string(), self.device.clone()));
        self.inner.delete(key)
    }
    fn list_page<'b>(
        &'b self,
        prefix: &'b str,
        token: Option<String>,
    ) -> BoxFuture<'b, objstore::Result<ListPage>> {
        self.inner.list_page(prefix, token)
    }
}

#[tokio::test]
async fn two_devices_converge_both_ways_on_every_kind_of_store() {
    let stores = stores();
    for (name, store) in stores.all() {
        let a = MemoryReplica::new("aaaa");
        let b = MemoryReplica::new("bbbb");
        a.set("note", "1", "body", Some("from a"));
        b.set("note", "2", "body", Some("from b"));

        space(store, "a").sync(&a).await.unwrap();
        space(store, "b").sync(&b).await.unwrap();
        space(store, "a").sync(&a).await.unwrap();

        assert_eq!(a.view(), b.view(), "{name}");
        assert_eq!(
            a.get("note", "2", "body").as_deref(),
            Some("from b"),
            "{name}"
        );
        assert_eq!(
            b.get("note", "1", "body").as_deref(),
            Some("from a"),
            "{name}"
        );
    }
}

#[tokio::test]
async fn concurrent_edits_resolve_per_field_and_the_same_way_on_both_devices() {
    let stores = stores();
    for (name, store) in stores.all() {
        let a = MemoryReplica::new("aaaa");
        let b = MemoryReplica::new("bbbb");
        a.set_at(1_000, "contact", "x", "note", Some("base"));
        space(store, "a").sync(&a).await.unwrap();
        space(store, "b").sync(&b).await.unwrap();

        a.set_at(2_000, "contact", "x", "note", Some("a edits note"));
        b.set_at(3_000, "contact", "x", "note", Some("b edits note later"));
        a.set_at(4_000, "contact", "x", "notify", Some("true"));
        b.set_at(5_000, "contact", "y", "note", Some("b only"));
        a.set_at(5_000, "contact", "z", "note", Some("same ms, a"));
        b.set_at(5_000, "contact", "z", "note", Some("same ms, b"));

        for _ in 0..2 {
            space(store, "a").sync(&a).await.unwrap();
            space(store, "b").sync(&b).await.unwrap();
        }
        assert_eq!(a.view(), b.view(), "{name}");
        assert_eq!(
            a.get("contact", "x", "note").as_deref(),
            Some("b edits note later")
        );
        assert_eq!(a.get("contact", "x", "notify").as_deref(), Some("true"));
        assert_eq!(a.get("contact", "z", "note").as_deref(), Some("same ms, b"));
    }
}

#[tokio::test]
async fn a_delete_beats_an_older_edit_and_loses_to_a_newer_one() {
    let store = MemoryStore::drive_like();
    let a = MemoryReplica::new("aaaa");
    let b = MemoryReplica::new("bbbb");
    a.set_at(1_000, "clip", "c1", "", Some("kept"));
    a.set_at(1_000, "clip", "c2", "", Some("kept"));
    space(&store, "a").sync(&a).await.unwrap();
    space(&store, "b").sync(&b).await.unwrap();

    a.set_at(3_000, "clip", "c1", "", None);
    b.set_at(2_000, "clip", "c1", "", Some("edited before the delete"));
    a.set_at(3_000, "clip", "c2", "", None);
    b.set_at(4_000, "clip", "c2", "", Some("edited after the delete"));

    for _ in 0..2 {
        space(&store, "b").sync(&b).await.unwrap();
        space(&store, "a").sync(&a).await.unwrap();
    }
    assert_eq!(a.view(), b.view());
    assert_eq!(a.get("clip", "c1", ""), None);
    assert_eq!(
        b.get("clip", "c2", "").as_deref(),
        Some("edited after the delete")
    );
}

#[tokio::test]
async fn a_device_with_a_slow_clock_still_wins_with_an_edit_made_after_it_synced() {
    let store = MemoryStore::new();
    let fast = MemoryReplica::new("fast");
    let slow = MemoryReplica::new("slow");
    fast.set_at(10_000_000, "pref", "signature", "", Some("from the future"));
    space(&store, "fast").sync(&fast).await.unwrap();
    space(&store, "slow").sync(&slow).await.unwrap();

    slow.set_at(
        5_000,
        "pref",
        "signature",
        "",
        Some("typed after seeing it"),
    );
    space(&store, "slow").sync(&slow).await.unwrap();
    space(&store, "fast").sync(&fast).await.unwrap();
    assert_eq!(
        fast.get("pref", "signature", "").as_deref(),
        Some("typed after seeing it")
    );
}

#[tokio::test]
async fn an_offline_device_catches_up_after_compactions_and_publishes_its_own_edits() {
    let stores = stores();
    for (name, store) in stores.all() {
        let a = MemoryReplica::new("aaaa");
        let b = MemoryReplica::new("bbbb");
        a.set("note", "seed", "", Some("seed"));
        small(store).sync(&a).await.unwrap();
        small(store).sync(&b).await.unwrap();

        for round in 0..8 {
            for i in 0..15 {
                a.set("note", &format!("{round}-{i}"), "", Some("a"));
            }
            small(store).sync(&a).await.unwrap();
        }
        b.set("note", "offline", "", Some("written offline"));
        b.set("note", "seed", "", Some("b overwrote seed"));

        let report = small(store).sync(&b).await.unwrap();
        assert!(report.snapshots_loaded >= 1, "{name}: {report:?}");
        small(store).sync(&a).await.unwrap();
        assert_eq!(a.view(), b.view(), "{name}");
        assert_eq!(
            a.get("note", "offline", "").as_deref(),
            Some("written offline")
        );
        assert_eq!(a.view().len(), 1 + 8 * 15 + 1);
    }
}

#[tokio::test]
async fn compaction_while_another_device_is_part_way_through_reading() {
    let stores = stores();
    for (name, store) in stores.all() {
        let a = MemoryReplica::new("aaaa");
        let b = MemoryReplica::new("bbbb");
        for i in 0..25 {
            a.set("note", &format!("early-{i}"), "", Some("a"));
        }
        small(store).sync(&a).await.unwrap();
        small(store).sync(&b).await.unwrap();

        for i in 0..25 {
            a.set("note", &format!("mid-{i}"), "", Some("a"));
        }
        small(store).sync(&a).await.unwrap();
        for i in 0..40 {
            a.set("note", &format!("late-{i}"), "", Some("a"));
        }
        let compacted = small(store).sync(&a).await.unwrap();
        assert!(compacted.snapshot_written, "{name}");
        assert!(compacted.deleted > 0, "{name}");

        small(store).sync(&b).await.unwrap();
        assert_eq!(a.view(), b.view(), "{name}");
        assert_eq!(b.view().len(), 90);
    }
}

#[tokio::test]
async fn drive_deletes_only_go_to_trash_and_no_object_has_two_writers() {
    let drive = MemoryStore::drive_like();
    let writes = Mutex::new(Vec::new());
    let watched = |device: &str| Watched {
        inner: &drive,
        device: device.to_string(),
        writes: &writes,
    };
    let replicas: Vec<MemoryReplica> = ["aaaa", "bbbb", "cccc"]
        .into_iter()
        .map(MemoryReplica::new)
        .collect();
    for round in 0..6 {
        for replica in &replicas {
            let id = replica.device_id().unwrap();
            for i in 0..12 {
                replica.set("rule", &format!("{i}"), "", Some(&format!("{id} {round}")));
            }
            let store = watched(&id);
            small(&store).sync(replica).await.unwrap();
        }
    }
    for replica in &replicas {
        let store = watched(&replica.device_id().unwrap());
        small(&store).sync(replica).await.unwrap();
    }
    assert_eq!(replicas[0].view(), replicas[1].view());
    assert_eq!(replicas[1].view(), replicas[2].view());

    let writers: BTreeSet<(String, String)> = writes.lock().unwrap().iter().cloned().collect();
    let keys: BTreeSet<&String> = writers.iter().map(|(key, _)| key).collect();
    assert_eq!(
        keys.len(),
        writers.len(),
        "an object was written by two devices"
    );
    for (key, device) in &writers {
        assert!(key.contains(device.as_str()), "{device} wrote {key}");
    }
    assert!(!drive.trashed().is_empty());
}

#[tokio::test]
async fn a_fresh_device_restores_from_a_snapshot_without_replaying_every_segment() {
    let stores = stores();
    for (name, store) in stores.all() {
        let a = MemoryReplica::new("aaaa");
        let b = MemoryReplica::new("bbbb");
        for i in 0..60 {
            a.set("note", &format!("a{i}"), "", Some("a"));
            b.set("note", &format!("b{i}"), "", Some("b"));
        }
        a.set("note", "a0", "", None);
        small(store).sync(&a).await.unwrap();
        small(store).sync(&b).await.unwrap();
        small(store).sync(&a).await.unwrap();

        let fresh = MemoryReplica::new("ffff");
        let report = small(store).sync(&fresh).await.unwrap();
        assert!(report.snapshots_loaded >= 1, "{name}: {report:?}");
        assert!(report.downloaded_segments <= 2, "{name}: {report:?}");
        assert_eq!(fresh.view(), a.view(), "{name}");
        assert_eq!(fresh.get("note", "a0", ""), None);
        assert_eq!(fresh.view().len(), 119);
    }
}

#[tokio::test]
async fn a_wrong_phrase_is_refused_before_anything_is_written() {
    let store = MemoryStore::drive_like();
    let a = MemoryReplica::new("aaaa");
    a.set("note", "1", "", Some("secret"));
    let real = RootKey::from_phrase(&Phrase::generate().unwrap()).unwrap();
    Space::new(&store, ROOT, real.app_key("notes"))
        .unwrap()
        .sync(&a)
        .await
        .unwrap();

    let before: Vec<(String, Vec<u8>)> = store
        .keys()
        .into_iter()
        .map(|k| {
            let raw = store.raw(&k).unwrap();
            (k, raw)
        })
        .collect();
    let wrong = RootKey::from_phrase(&Phrase::generate().unwrap()).unwrap();
    let intruder = MemoryReplica::new("ffff");
    intruder.set("note", "2", "", Some("would be unreadable"));
    let space = Space::new(&store, ROOT, wrong.app_key("notes")).unwrap();
    assert!(matches!(
        space.check_key().await,
        Err(SyncError::WrongKey { .. })
    ));
    assert!(matches!(
        space.sync(&intruder).await,
        Err(SyncError::WrongKey { .. })
    ));
    let after: Vec<(String, Vec<u8>)> = store
        .keys()
        .into_iter()
        .map(|k| {
            let raw = store.raw(&k).unwrap();
            (k, raw)
        })
        .collect();
    assert_eq!(before, after);
    assert_eq!(intruder.view().len(), 1);

    let other_app = Space::new(&store, ROOT, real.app_key("calendar")).unwrap();
    assert!(matches!(
        other_app.check_key().await,
        Err(SyncError::WrongKey { .. })
    ));
}

#[tokio::test]
async fn the_store_sees_only_ciphertext() {
    let store = MemoryStore::new();
    let a = MemoryReplica::new("aaaa");
    a.set(
        "contact",
        "someone@example.com",
        "note",
        Some("private words"),
    );
    space(&store, "a").sync(&a).await.unwrap();
    for key in store.keys() {
        let raw = String::from_utf8_lossy(&store.raw(&key).unwrap()).to_string();
        assert!(!raw.contains("private words") && !raw.contains("someone@example.com"));
    }
}

#[tokio::test]
async fn moving_an_object_between_names_is_detected() {
    let store = MemoryStore::new();
    let a = MemoryReplica::new("aaaa");
    let b = MemoryReplica::new("bbbb");
    a.set("note", "1", "", Some("a"));
    b.set("note", "2", "", Some("b"));
    space(&store, "a").sync(&a).await.unwrap();
    space(&store, "b").sync(&b).await.unwrap();
    let a_manifest = store.raw(&format!("{ROOT}/devices/aaaa")).unwrap();
    store.insert_raw(&format!("{ROOT}/devices/bbbb"), a_manifest);
    let c = MemoryReplica::new("cccc");
    assert!(matches!(
        space(&store, "c").sync(&c).await,
        Err(SyncError::WrongKey { .. })
    ));
}

#[tokio::test]
async fn polling_is_one_request_and_only_says_yes_when_something_changed() {
    let store = MemoryStore::drive_like();
    let a = MemoryReplica::new("aaaa");
    let b = MemoryReplica::new("bbbb");
    a.set("note", "1", "", Some("a"));
    space(&store, "a").sync(&a).await.unwrap();
    space(&store, "b").sync(&b).await.unwrap();
    space(&store, "a").sync(&a).await.unwrap();

    let before = store.requests();
    assert!(!space(&store, "a").poll(&a).await.unwrap());
    assert_eq!(store.requests() - before, 1);
    assert!(!space(&store, "b").poll(&b).await.unwrap());

    b.set("note", "2", "", Some("b"));
    assert!(space(&store, "b").poll(&b).await.unwrap());
    space(&store, "b").sync(&b).await.unwrap();
    assert!(space(&store, "a").poll(&a).await.unwrap());
    space(&store, "a").sync(&a).await.unwrap();
    assert!(!space(&store, "a").poll(&a).await.unwrap());
    assert_eq!(a.view(), b.view());
}

#[tokio::test]
async fn a_copied_data_folder_is_caught_where_the_store_has_conditional_writes() {
    let store = MemoryStore::new();
    let original = MemoryReplica::new("aaaa");
    original.set("note", "1", "", Some("a"));
    space(&store, "a").sync(&original).await.unwrap();

    let copy = MemoryReplica::new("aaaa");
    copy.absorb(&original.log()).unwrap();
    copy.set_meta(
        "margin-sync",
        &original.meta("margin-sync").unwrap().unwrap(),
    )
    .unwrap();
    original.set("note", "2", "", Some("original"));
    copy.set("note", "2", "", Some("copy"));
    space(&store, "a").sync(&original).await.unwrap();
    assert!(matches!(
        space(&store, "a").sync(&copy).await,
        Err(SyncError::DeviceClash { .. })
    ));
}

#[tokio::test]
async fn snapshots_are_kept_to_the_policy_and_the_registry_lists_every_device() {
    let store = MemoryStore::new();
    let a = MemoryReplica::new("aaaa");
    let b = MemoryReplica::new("bbbb");
    for round in 0..10 {
        for i in 0..40 {
            a.set("note", &format!("{i}"), "", Some(&round.to_string()));
        }
        small(&store).sync(&a).await.unwrap();
    }
    let snapshots = store
        .keys()
        .into_iter()
        .filter(|k| k.starts_with(&format!("{ROOT}/snapshots/aaaa/")))
        .count();
    assert!(snapshots <= 2, "{snapshots}");
    small(&store).sync(&b).await.unwrap();
    let devices = space(&store, "b").devices(&b).unwrap();
    assert_eq!(devices.len(), 2);
    assert!(
        devices
            .iter()
            .any(|d| d.id == "aaaa" && d.name == "small" && !d.this_device)
    );
}

#[tokio::test]
async fn an_empty_device_registers_itself_without_writing_records() {
    let store = MemoryStore::new();
    let a = MemoryReplica::new("aaaa");
    let report = space(&store, "a").sync(&a).await.unwrap();
    assert_eq!(report.uploaded_segments, 0);
    assert!(!report.snapshot_written);
    assert_eq!(store.keys(), vec![format!("{ROOT}/devices/aaaa")]);
    let again = store.requests();
    space(&store, "a").sync(&a).await.unwrap();
    assert_eq!(store.requests() - again, 1);
}

/// Serves an older copy of some objects once, as an eventually consistent store does, and can
/// fail a chosen put to cut a pass off half way.
struct Unreliable<'a> {
    inner: &'a dyn ObjectStore,
    stale: Mutex<std::collections::BTreeMap<String, Vec<u8>>>,
    fail_put: Mutex<Option<usize>>,
}

impl<'a> Unreliable<'a> {
    fn new(inner: &'a dyn ObjectStore) -> Unreliable<'a> {
        Unreliable {
            inner,
            stale: Mutex::new(Default::default()),
            fail_put: Mutex::new(None),
        }
    }
}

impl ObjectStore for Unreliable<'_> {
    fn capabilities(&self) -> Capabilities {
        self.inner.capabilities()
    }
    fn put<'b>(
        &'b self,
        key: &'b str,
        bytes: Vec<u8>,
        mode: PutMode,
    ) -> BoxFuture<'b, objstore::Result<ObjectMeta>> {
        let mut fail = self.fail_put.lock().unwrap();
        if let Some(left) = fail.as_mut() {
            if *left == 0 {
                *fail = None;
                return Box::pin(async {
                    Err(objstore::Error::Transport {
                        message: "connection reset".into(),
                    })
                });
            }
            *left -= 1;
        }
        self.inner.put(key, bytes, mode)
    }
    fn get<'b>(&'b self, key: &'b str) -> BoxFuture<'b, objstore::Result<Object>> {
        if let Some(bytes) = self.stale.lock().unwrap().remove(key) {
            return Box::pin(async move {
                let mut object = self.inner.get(key).await?;
                object.bytes = bytes;
                Ok(object)
            });
        }
        self.inner.get(key)
    }
    fn head<'b>(&'b self, key: &'b str) -> BoxFuture<'b, objstore::Result<ObjectMeta>> {
        self.inner.head(key)
    }
    fn delete<'b>(&'b self, key: &'b str) -> BoxFuture<'b, objstore::Result<()>> {
        self.inner.delete(key)
    }
    fn list_page<'b>(
        &'b self,
        prefix: &'b str,
        token: Option<String>,
    ) -> BoxFuture<'b, objstore::Result<ListPage>> {
        self.inner.list_page(prefix, token)
    }
}

#[tokio::test]
async fn a_reader_holding_a_stale_manifest_recovers_when_compaction_removed_its_segments() {
    let drive = MemoryStore::drive_like();
    let a = MemoryReplica::new("aaaa");
    let b = MemoryReplica::new("bbbb");
    small(&drive).sync(&b).await.unwrap();
    for i in 0..20 {
        a.set("note", &format!("{i}"), "", Some("a"));
    }
    small(&drive).sync(&a).await.unwrap();
    small(&drive).sync(&a).await.unwrap();
    for i in 20..30 {
        a.set("note", &format!("{i}"), "", Some("a"));
    }
    small(&drive).sync(&a).await.unwrap();
    let manifest = format!("{ROOT}/devices/aaaa");
    let old = drive.raw(&manifest).unwrap();
    for i in 30..80 {
        a.set("note", &format!("{i}"), "", Some("a"));
    }
    assert!(small(&drive).sync(&a).await.unwrap().snapshot_written);

    let flaky = Unreliable::new(&drive);
    flaky.stale.lock().unwrap().insert(manifest, old);
    small(&flaky).sync(&b).await.unwrap();
    assert_eq!(b.view(), a.view());
    assert_eq!(b.view().len(), 80);
}

#[tokio::test]
async fn a_pass_cut_off_half_way_through_uploading_is_finished_by_the_next() {
    let stores = stores();
    for (name, store) in stores.all() {
        let a = MemoryReplica::new("aaaa");
        let b = MemoryReplica::new("bbbb");
        a.set("note", "seed", "", Some("a"));
        small(store).sync(&a).await.unwrap();
        for i in 0..25 {
            a.set("note", &format!("{i}"), "", Some("a"));
        }
        let flaky = Unreliable::new(store);
        *flaky.fail_put.lock().unwrap() = Some(1);
        assert!(small(&flaky).sync(&a).await.is_err(), "{name}");
        small(store).sync(&b).await.unwrap();
        small(store).sync(&a).await.unwrap();
        small(store).sync(&b).await.unwrap();
        assert_eq!(b.view(), a.view(), "{name}");
        assert_eq!(b.view().len(), 26);
    }
}

#[tokio::test]
async fn a_crash_after_the_manifest_but_before_local_state_is_not_mistaken_for_a_copy() {
    let store = MemoryStore::new();
    let a = MemoryReplica::new("aaaa");
    a.set("note", "1", "", Some("a"));
    space(&store, "a").sync(&a).await.unwrap();
    let saved = a.meta("margin-sync").unwrap().unwrap();
    a.set("note", "2", "", Some("a"));
    space(&store, "a").sync(&a).await.unwrap();
    a.set_meta("margin-sync", &saved).unwrap();
    a.set("note", "3", "", Some("a"));
    space(&store, "a").sync(&a).await.unwrap();

    let b = MemoryReplica::new("bbbb");
    space(&store, "b").sync(&b).await.unwrap();
    assert_eq!(b.view(), a.view());
}

#[tokio::test]
async fn a_restored_device_adds_no_snapshot_but_one_holding_records_nobody_published_does() {
    let store = MemoryStore::new();
    let a = MemoryReplica::new("aaaa");
    a.set("note", "1", "", Some("a"));
    space(&store, "a").sync(&a).await.unwrap();

    let restored = MemoryReplica::new("ffff");
    let report = space(&store, "f").sync(&restored).await.unwrap();
    assert!(!report.snapshot_written);

    let carrier = MemoryReplica::new("cccc");
    let offline = MemoryReplica::new("oooo");
    offline.set("note", "from an old build", "", Some("only on the carrier"));
    carrier.absorb(&offline.log()).unwrap();
    assert!(
        space(&store, "c")
            .sync(&carrier)
            .await
            .unwrap()
            .snapshot_written
    );
    let fresh = MemoryReplica::new("gggg");
    space(&store, "g").sync(&fresh).await.unwrap();
    assert_eq!(
        fresh.get("note", "from an old build", "").as_deref(),
        Some("only on the carrier")
    );
}

#[tokio::test]
async fn moving_to_another_store_publishes_everything_there() {
    let first = MemoryStore::new();
    let second = MemoryStore::drive_like();
    let a = MemoryReplica::new("aaaa");
    a.set("note", "1", "", Some("a"));
    space(&first, "a")
        .with_store_id("one")
        .sync(&a)
        .await
        .unwrap();
    space(&second, "a")
        .with_store_id("two")
        .sync(&a)
        .await
        .unwrap();
    let b = MemoryReplica::new("bbbb");
    space(&second, "b")
        .with_store_id("two")
        .sync(&b)
        .await
        .unwrap();
    assert_eq!(b.view(), a.view());
    a.set("note", "2", "", Some("a"));
    space(&second, "a")
        .with_store_id("two")
        .sync(&a)
        .await
        .unwrap();
    space(&second, "b")
        .with_store_id("two")
        .sync(&b)
        .await
        .unwrap();
    assert_eq!(b.view().len(), 2);
}

#[tokio::test]
async fn a_manifest_naming_a_snapshot_the_store_lacks_is_not_adopted() {
    let store = MemoryStore::new();
    let a = MemoryReplica::new("aaaa");
    a.set("note", "1", "", Some("a"));
    space(&store, "a").sync(&a).await.unwrap();
    for key in store.keys() {
        if key.contains("/snapshots/") {
            objstore::ObjectStore::delete(&store, &key).await.unwrap();
        }
    }
    a.set_meta("margin-sync", "{}").unwrap();
    space(&store, "a").sync(&a).await.unwrap();
    let b = MemoryReplica::new("bbbb");
    space(&store, "b").sync(&b).await.unwrap();
    assert_eq!(b.view(), a.view());
}
