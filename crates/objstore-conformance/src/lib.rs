use objstore::{Error, ObjectStore, Prefixed, PutMode};

pub async fn run(store: &dyn ObjectStore, root: &str) {
    round_trip(store, root).await;
    missing(store, root).await;
    overwrite(store, root).await;
    listing(store, root).await;
    many(store, root).await;
    conditional(store, root).await;
    invalid_keys(store).await;
    prefixed(store, root).await;
    cleanup(store, root).await;
}

async fn round_trip(store: &dyn ObjectStore, root: &str) {
    let key = format!("{root}/round/one.bin");
    let bytes: Vec<u8> = (0..=255u8).cycle().take(70_000).collect();
    let put = store
        .put(&key, bytes.clone(), PutMode::Overwrite)
        .await
        .unwrap();
    assert_eq!(put.key, key);
    assert_eq!(put.size, bytes.len() as u64);
    let head = store.head(&key).await.unwrap();
    assert_eq!(head.size, bytes.len() as u64);
    assert_eq!(head.etag, put.etag, "head etag matches put etag");
    let got = store.get(&key).await.unwrap();
    assert_eq!(got.bytes, bytes);
    assert_eq!(got.meta.etag, put.etag);

    let empty = format!("{root}/round/empty");
    store
        .put(&empty, Vec::new(), PutMode::Overwrite)
        .await
        .unwrap();
    assert!(store.get(&empty).await.unwrap().bytes.is_empty());
    assert_eq!(store.head(&empty).await.unwrap().size, 0);

    let spaced = format!("{root}/round/with space+plus%percent é.txt");
    store
        .put(&spaced, b"odd".to_vec(), PutMode::Overwrite)
        .await
        .unwrap();
    assert_eq!(store.get(&spaced).await.unwrap().bytes, b"odd");
    let listed = store.list(&format!("{root}/round/")).await.unwrap();
    assert!(
        listed.iter().any(|meta| meta.key == spaced),
        "odd key listed verbatim"
    );
}

async fn missing(store: &dyn ObjectStore, root: &str) {
    let key = format!("{root}/missing/nothing");
    assert!(matches!(store.get(&key).await, Err(Error::NotFound { .. })));
    assert!(matches!(
        store.head(&key).await,
        Err(Error::NotFound { .. })
    ));
    store.delete(&key).await.unwrap();
    assert!(
        store
            .list(&format!("{root}/missing/"))
            .await
            .unwrap()
            .is_empty()
    );
}

async fn overwrite(store: &dyn ObjectStore, root: &str) {
    let key = format!("{root}/over/k");
    let first = store
        .put(&key, b"first".to_vec(), PutMode::Overwrite)
        .await
        .unwrap();
    let second = store
        .put(&key, b"second!".to_vec(), PutMode::Overwrite)
        .await
        .unwrap();
    assert_ne!(first.etag, second.etag);
    assert_eq!(store.get(&key).await.unwrap().bytes, b"second!");
    store.delete(&key).await.unwrap();
    assert!(matches!(store.get(&key).await, Err(Error::NotFound { .. })));
}

async fn listing(store: &dyn ObjectStore, root: &str) {
    let base = format!("{root}/list");
    for key in ["a/x", "a/y/z", "ab/w", "b/v", "a.txt"] {
        store
            .put(
                &format!("{base}/{key}"),
                key.as_bytes().to_vec(),
                PutMode::Overwrite,
            )
            .await
            .unwrap();
    }
    let keys = |prefix: String| async move {
        store
            .list(&prefix)
            .await
            .unwrap()
            .into_iter()
            .map(|meta| meta.key)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        keys(format!("{base}/a/")).await,
        vec![format!("{base}/a/x"), format!("{base}/a/y/z")]
    );
    assert_eq!(
        keys(format!("{base}/a")).await,
        vec![
            format!("{base}/a.txt"),
            format!("{base}/a/x"),
            format!("{base}/a/y/z"),
            format!("{base}/ab/w")
        ]
    );
    assert_eq!(keys(format!("{base}/")).await.len(), 5);
    assert!(keys(format!("{base}/c")).await.is_empty());
    let sizes = store.list(&format!("{base}/b/")).await.unwrap();
    assert_eq!(sizes[0].size, 3);
    assert_eq!(
        sizes[0].etag,
        store.head(&format!("{base}/b/v")).await.unwrap().etag
    );
}

async fn many(store: &dyn ObjectStore, root: &str) {
    let base = format!("{root}/many");
    let mut want = Vec::new();
    for at in 0..25 {
        let key = format!("{base}/k{at:03}");
        store
            .put(&key, vec![at as u8], PutMode::Overwrite)
            .await
            .unwrap();
        want.push(key);
    }
    let got: Vec<String> = store
        .list(&format!("{base}/"))
        .await
        .unwrap()
        .into_iter()
        .map(|m| m.key)
        .collect();
    assert_eq!(got, want);
}

async fn conditional(store: &dyn ObjectStore, root: &str) {
    let key = format!("{root}/cond/k");
    if !store.capabilities().conditional_put {
        for mode in [
            PutMode::IfNoneMatch,
            PutMode::IfMatch(objstore::ETag::new("x")),
        ] {
            assert!(matches!(
                store.put(&key, b"x".to_vec(), mode).await,
                Err(Error::Unsupported { .. })
            ));
        }
        return;
    }
    let created = store
        .put(&key, b"one".to_vec(), PutMode::IfNoneMatch)
        .await
        .unwrap();
    assert!(matches!(
        store.put(&key, b"two".to_vec(), PutMode::IfNoneMatch).await,
        Err(Error::PreconditionFailed { .. })
    ));
    let updated = store
        .put(
            &key,
            b"two!".to_vec(),
            PutMode::IfMatch(created.etag.clone()),
        )
        .await
        .unwrap();
    assert_eq!(store.head(&key).await.unwrap().etag, updated.etag);
    assert!(matches!(
        store
            .put(&key, b"three".to_vec(), PutMode::IfMatch(created.etag))
            .await,
        Err(Error::PreconditionFailed { .. })
    ));
    assert_eq!(store.get(&key).await.unwrap().bytes, b"two!");
    assert!(matches!(
        store
            .put(
                &format!("{root}/cond/absent"),
                b"x".to_vec(),
                PutMode::IfMatch(updated.etag)
            )
            .await,
        Err(Error::PreconditionFailed { .. } | Error::NotFound { .. })
    ));
}

async fn invalid_keys(store: &dyn ObjectStore) {
    for key in [
        "",
        "/lead",
        "trail/",
        "a//b",
        "a/../b",
        "./a",
        "back\\slash",
    ] {
        assert!(
            matches!(
                store.put(key, b"x".to_vec(), PutMode::Overwrite).await,
                Err(Error::InvalidKey { .. })
            ),
            "{key:?} refused"
        );
    }
}

async fn prefixed(store: &dyn ObjectStore, root: &str) {
    let scoped = Prefixed::new(store, &format!("{root}/apps/notes")).unwrap();
    let meta = scoped
        .put("doc/1", b"scoped".to_vec(), PutMode::Overwrite)
        .await
        .unwrap();
    assert_eq!(meta.key, "doc/1");
    assert_eq!(
        store
            .get(&format!("{root}/apps/notes/doc/1"))
            .await
            .unwrap()
            .bytes,
        b"scoped"
    );
    store
        .put(
            &format!("{root}/apps/notes-other/doc/1"),
            b"no".to_vec(),
            PutMode::Overwrite,
        )
        .await
        .unwrap();
    let listed: Vec<String> = scoped
        .list("")
        .await
        .unwrap()
        .into_iter()
        .map(|m| m.key)
        .collect();
    assert_eq!(listed, vec!["doc/1".to_string()]);
    assert!(matches!(scoped.get("doc/2").await, Err(Error::NotFound { key }) if key == "doc/2"));
    assert!(matches!(
        scoped.get("../x").await,
        Err(Error::InvalidKey { .. })
    ));
}

async fn cleanup(store: &dyn ObjectStore, root: &str) {
    for meta in store.list(&format!("{root}/")).await.unwrap() {
        store.delete(&meta.key).await.unwrap();
    }
    assert!(store.list(&format!("{root}/")).await.unwrap().is_empty());
}

pub fn unique_root(name: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("conformance-{name}-{nanos}")
}
