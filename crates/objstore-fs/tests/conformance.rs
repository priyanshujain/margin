use objstore::{ObjectStore, PutMode};
use objstore_fs::FsStore;

#[tokio::test]
async fn conformance() {
    let dir = tempfile::tempdir().unwrap();
    let store = FsStore::new(dir.path()).unwrap();
    objstore_conformance::run(&store, &objstore_conformance::unique_root("fs")).await;
}

#[tokio::test]
async fn ignores_sync_tool_files() {
    let dir = tempfile::tempdir().unwrap();
    let store = FsStore::new(dir.path()).unwrap();
    store
        .put("app/doc.bin", b"real".to_vec(), PutMode::Overwrite)
        .await
        .unwrap();
    let app = dir.path().join("app");
    for name in [
        "doc (conflicted copy 2026-01-02).bin",
        "doc (Priyanshu's conflicted copy 2026-01-02).bin",
        "doc.sync-conflict-20260102-101010-ABCDEFG.bin",
        ".DS_Store",
        ".syncthing.doc.bin.tmp",
        "~syncthing~doc.bin.tmp",
        ".doc.bin.icloud",
        "desktop.ini",
    ] {
        std::fs::write(app.join(name), b"noise").unwrap();
    }
    std::fs::create_dir_all(dir.path().join(".stversions/app")).unwrap();
    std::fs::write(dir.path().join(".stversions/app/doc.bin"), b"old").unwrap();
    let keys: Vec<String> = store
        .list("")
        .await
        .unwrap()
        .into_iter()
        .map(|m| m.key)
        .collect();
    assert_eq!(keys, vec!["app/doc.bin".to_string()]);
    assert!(
        store
            .put("app/.hidden", b"x".to_vec(), PutMode::Overwrite)
            .await
            .is_err()
    );
    assert!(
        store
            .put(
                "app/x (conflicted copy).bin",
                b"x".to_vec(),
                PutMode::Overwrite
            )
            .await
            .is_err()
    );
}

#[tokio::test]
async fn leaves_no_temporary_files() {
    let dir = tempfile::tempdir().unwrap();
    let store = FsStore::new(dir.path()).unwrap();
    let first = store
        .put("k", b"1".to_vec(), PutMode::IfNoneMatch)
        .await
        .unwrap();
    store
        .put("k", b"2".to_vec(), PutMode::IfMatch(first.etag.clone()))
        .await
        .unwrap();
    assert!(
        store
            .put("k", b"3".to_vec(), PutMode::IfMatch(first.etag))
            .await
            .is_err()
    );
    assert!(
        store
            .put("k", b"4".to_vec(), PutMode::IfNoneMatch)
            .await
            .is_err()
    );
    let names: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(names, vec![std::ffi::OsString::from("k")]);
    assert_eq!(std::fs::read(dir.path().join("k")).unwrap(), b"2");
}

#[tokio::test]
async fn concurrent_if_match_has_one_winner() {
    let dir = tempfile::tempdir().unwrap();
    let store = std::sync::Arc::new(FsStore::new(dir.path()).unwrap());
    let base = store
        .put("k", b"base".to_vec(), PutMode::Overwrite)
        .await
        .unwrap()
        .etag;
    let mut tasks = Vec::new();
    for at in 0..16u8 {
        let store = store.clone();
        let base = base.clone();
        tasks.push(tokio::spawn(async move {
            store
                .put("k", vec![at; 8], PutMode::IfMatch(base))
                .await
                .is_ok()
        }));
    }
    let mut winners = 0;
    for task in tasks {
        winners += task.await.unwrap() as usize;
    }
    assert_eq!(winners, 1);
    assert!(store.get("k").await.is_ok());
}
