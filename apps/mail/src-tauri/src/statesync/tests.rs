use std::sync::Mutex;

use chacha20poly1305::aead::{Aead, Payload as AeadPayload};
use chacha20poly1305::{KeyInit, XChaCha20Poly1305};
use margin_sync::memory::MemoryStore;
use margin_sync::{Key, Phrase, Policy, Replica, RootKey, Space};
use objstore::{ObjectStore, PutMode};
use rusqlite::Connection;
use tauri::async_runtime::block_on;

use super::legacy;
use super::replica::{Journal, StateReplica};
use crate::db;
use crate::dto::{Destination, Pile};
use crate::state::journal::{self, Payload};
use crate::state::{device, merge, write};

struct Device(Mutex<Connection>);

impl Journal for &Device {
    fn with<T, F: FnOnce(&Connection) -> Result<T, String>>(&self, f: F) -> Result<T, String> {
        let conn = self.0.lock().map_err(|e| e.to_string())?;
        f(&conn)
    }
}

const TABLES: [&str; 12] = [
    "sender_rules",
    "piles",
    "snoozes",
    "notes",
    "renames",
    "merges",
    "clips",
    "thread_flags",
    "contacts",
    "prefs",
    "markers",
    "thread_places",
];

const ACCOUNT: &str = "ana@example.com";

impl Device {
    fn new() -> Device {
        Device(Mutex::new(db::memory().expect("in-memory databases")))
    }

    fn on<T>(&self, f: impl FnOnce(&Connection) -> T) -> T {
        f(&self.0.lock().unwrap())
    }

    fn id(&self) -> String {
        self.on(|conn| device::device_id(conn).unwrap())
    }

    fn replica(&self) -> StateReplica<&Device> {
        StateReplica(self)
    }

    fn tables(&self) -> Vec<String> {
        self.on(|conn| {
            let mut out = Vec::new();
            for table in TABLES {
                let mut stmt = conn
                    .prepare(&format!("SELECT * FROM state.{table}"))
                    .unwrap();
                let columns = stmt.column_count();
                let mut rows: Vec<String> = stmt
                    .query_map([], |row| {
                        let mut cells = Vec::new();
                        for at in 0..columns {
                            cells.push(format!("{:?}", row.get::<_, rusqlite::types::Value>(at)?));
                        }
                        Ok(format!("{table}: {}", cells.join("|")))
                    })
                    .unwrap()
                    .map(Result::unwrap)
                    .collect();
                rows.sort();
                out.extend(rows);
            }
            out
        })
    }
}

fn key() -> Key {
    Key::from_bytes([9; 32])
}

fn sync(store: &dyn ObjectStore, device: &Device) -> margin_sync::Report {
    let space = Space::new(store, &super::space_root(ACCOUNT), key())
        .unwrap()
        .with_policy(Policy {
            segment_records: 4,
            max_segments: 3,
            snapshot_every_ms: i64::MAX,
            keep_snapshots: 2,
        });
    block_on(space.sync(&device.replica())).expect("a pass")
}

fn decide(conn: &Connection, tag: &str) {
    write::set_rule(
        conn,
        &format!("{tag}@example.com"),
        false,
        Destination::Inbox,
        None,
    )
    .unwrap();
    write::add_note(conn, &format!("thread-{tag}"), "the roof is leaking", None).unwrap();
    write::set_pile(conn, &format!("thread-{tag}"), Pile::ReplyLater).unwrap();
}

#[test]
fn two_mail_devices_converge_through_a_folder_and_a_drive_like_store() {
    let dir = tempfile::tempdir().unwrap();
    let fs = objstore_fs_store(dir.path());
    let drive = MemoryStore::drive_like();
    for store in [&fs as &dyn ObjectStore, &drive] {
        let one = Device::new();
        let two = Device::new();
        one.on(|conn| {
            decide(conn, "one");
            journal::append_at(
                conn,
                5_000,
                "thread-shared",
                &Payload::Rename {
                    name: Some("one".into()),
                },
            )
            .unwrap();
        });
        two.on(|conn| {
            decide(conn, "two");
            journal::append_at(
                conn,
                6_000,
                "thread-shared",
                &Payload::Rename {
                    name: Some("two".into()),
                },
            )
            .unwrap();
            write::clear_pile(conn, "thread-two").unwrap();
        });
        sync(store, &one);
        sync(store, &two);
        sync(store, &one);
        assert_eq!(one.tables(), two.tables());
        let name: String = one.on(|conn| {
            conn.query_row(
                "SELECT name FROM state.renames WHERE thread_key = 'thread-shared'",
                [],
                |r| r.get(0),
            )
            .unwrap()
        });
        assert_eq!(name, "two");
    }
}

fn objstore_fs_store(dir: &std::path::Path) -> Box<dyn ObjectStore> {
    margin_sync::Storage::Folder {
        path: dir.join("sync").to_string_lossy().to_string(),
    }
    .open(None)
    .unwrap()
}

#[test]
fn a_snapshot_rebuilds_the_same_tables_on_a_fresh_device() {
    let store = MemoryStore::new();
    let one = Device::new();
    one.on(|conn| {
        for i in 0..10 {
            decide(conn, &format!("d{i}"));
        }
        write::clear_pile(conn, "thread-d3").unwrap();
        write::set_rule(conn, "d4@example.com", false, Destination::Feed, None).unwrap();
    });
    let first = sync(&store, &one);
    assert!(first.snapshot_written);

    let fresh = Device::new();
    let report = sync(&store, &fresh);
    assert_eq!(report.snapshots_loaded, 1);
    assert_eq!(report.downloaded_segments, 0);
    assert_eq!(fresh.tables(), one.tables());

    let compacted = one.replica().compacted().unwrap();
    let full = one.on(|conn| journal::records(conn).unwrap());
    assert!(compacted.len() < full.len());
}

fn legacy_seal(key: &Key, name: &str, plain: &[u8]) -> Vec<u8> {
    let nonce = [3u8; 24];
    let body = XChaCha20Poly1305::new(key.as_bytes().into())
        .encrypt(
            (&nonce).into(),
            AeadPayload {
                msg: plain,
                aad: name.as_bytes(),
            },
        )
        .unwrap();
    let mut out = b"MMB1".to_vec();
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&body);
    out
}

/// What a build before margin-sync left in a bucket: one device's journal in MMB1 segments.
fn write_legacy(store: &dyn ObjectStore, prefix: &str, device: &Device, key: &Key) {
    let hash = legacy::account_hash(ACCOUNT);
    let id = device.id();
    let records = device.on(|conn| merge::export(conn, &id, 0).unwrap());
    for chunk in records.chunks(2) {
        let (first, last) = (chunk[0].seq, chunk[chunk.len() - 1].seq);
        let file = format!("{first:012}-{last:012}.seg");
        let name = format!("{hash}/{id}/{file}");
        let sealed = legacy_seal(key, &name, merge::encode(chunk).unwrap().as_bytes());
        block_on(store.put(&format!("{prefix}{id}/{file}"), sealed, PutMode::Overwrite)).unwrap();
    }
}

#[test]
fn old_backups_in_r2_and_drive_layouts_are_taken_in_and_republished_in_the_new_layout() {
    let phrase = Phrase::parse(
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art",
    )
    .unwrap();
    let legacy_key = margin_crypt::mail_backup_key(&phrase).unwrap();
    let hash = legacy::account_hash(ACCOUNT);
    for prefix in [format!("{hash}/"), format!("mail/{hash}/")] {
        let store = MemoryStore::drive_like();
        let old = Device::new();
        old.on(|conn| {
            for i in 0..5 {
                decide(conn, &format!("old{i}"));
            }
        });
        write_legacy(&store, &prefix, &old, &legacy_key);

        let upgraded = Device::new();
        upgraded.on(|conn| decide(conn, "upgraded"));
        let applied = block_on(legacy::import(
            &store,
            &prefix,
            &hash,
            &legacy_key,
            &upgraded.replica(),
        ))
        .unwrap();
        assert!(applied > 0);
        let again = block_on(legacy::import(
            &store,
            &prefix,
            &hash,
            &legacy_key,
            &upgraded.replica(),
        ))
        .unwrap();
        assert_eq!(again, 0);
        sync(&store, &upgraded);

        let wrong = Key::from_bytes([1; 32]);
        let refused = Device::new();
        assert!(
            block_on(legacy::import(
                &store,
                &prefix,
                &hash,
                &wrong,
                &refused.replica()
            ))
            .is_err()
        );
        assert!(refused.tables().is_empty());

        let fresh = Device::new();
        sync(&store, &fresh);
        assert_eq!(fresh.tables(), upgraded.tables());
        for row in old.tables() {
            assert!(fresh.tables().contains(&row), "{row}");
        }
        let keys = store.keys();
        assert!(
            keys.iter()
                .any(|k| k.starts_with(&format!("mail/{hash}/devices/")))
        );
        assert!(
            keys.iter().filter(|k| k.ends_with(".seg")).count() > 0,
            "the source is left in place"
        );
    }
}

#[test]
fn the_suite_phrase_unlocks_both_the_new_layout_and_a_restore_on_a_second_app() {
    let phrase = Phrase::generate().unwrap();
    let root = RootKey::from_phrase(&phrase).unwrap();
    let store = MemoryStore::new();
    let one = Device::new();
    one.on(|conn| decide(conn, "x"));
    let space = Space::new(
        &store,
        &super::space_root(ACCOUNT),
        root.app_key(super::APP),
    )
    .unwrap();
    block_on(space.sync(&one.replica())).unwrap();

    let calendar = Space::new(
        &store,
        &super::space_root(ACCOUNT),
        root.app_key("calendar"),
    )
    .unwrap();
    assert!(block_on(calendar.check_key()).is_err());
    let again =
        RootKey::from_phrase(&Phrase::parse(&phrase.as_str().to_uppercase()).unwrap()).unwrap();
    let space = Space::new(
        &store,
        &super::space_root(ACCOUNT),
        again.app_key(super::APP),
    )
    .unwrap();
    assert_eq!(block_on(space.check_key()).unwrap(), 1);
}

#[test]
fn the_space_root_names_the_app_and_hides_the_address() {
    let root = super::space_root(ACCOUNT);
    assert!(root.starts_with("mail/"));
    assert!(!root.contains("ana"));
    assert_eq!(root, super::space_root(" Ana@Example.com "));
}

#[test]
fn a_sync_folder_must_be_an_existing_absolute_directory_outside_the_data_dir() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    let inside = data.join("accounts");
    let outside = dir.path().join("Dropbox/Margin");
    std::fs::create_dir_all(&inside).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    let file = dir.path().join("note.txt");
    std::fs::write(&file, "x").unwrap();

    assert_eq!(
        super::checked_folder(&outside, &data).unwrap(),
        outside.canonicalize().unwrap()
    );
    assert!(super::checked_folder(std::path::Path::new("relative/dir"), &data).is_err());
    assert!(super::checked_folder(&dir.path().join("missing"), &data).is_err());
    assert!(super::checked_folder(&file, &data).is_err());
    assert!(super::checked_folder(&inside, &data).is_err());
    assert!(super::checked_folder(&data, &data).is_err());
    assert!(super::checked_folder(&inside.join(".."), &data).is_err());
}
