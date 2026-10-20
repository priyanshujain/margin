mod fake_drive;

use std::sync::Arc;

use objstore::{Error, ObjectStore, PutMode};
use objstore_gdrive::{Config, DriveStore, TokenProvider};

fn tokens(token: &'static str) -> TokenProvider {
    Arc::new(move || Box::pin(async move { Ok(token.to_string()) }))
}

fn store(base: &str, root: &[&str], token: TokenProvider) -> DriveStore {
    let mut config = Config::new(root.iter().copied());
    config.api_base = base.to_string();
    DriveStore::new(config, token).unwrap()
}

#[tokio::test]
async fn conformance_against_fake_drive() {
    let (base, _) = fake_drive::start().await;
    let drive = store(&base, &["margin", "test"], tokens(fake_drive::TOKEN));
    objstore_conformance::run(&drive, &objstore_conformance::unique_root("gdrive")).await;
}

#[tokio::test]
async fn reads_and_writes_mail_backup_layout() {
    let (base, state) = fake_drive::start().await;
    {
        let mut state = state.lock().unwrap();
        let margin = state.folder("margin", None);
        let mail = state.folder("mail", Some(&margin));
        let account = state.folder("abc", Some(&mail));
        let device = state.folder("device-1", Some(&account));
        state.add(
            "000000000001-000000000004.seg",
            "application/octet-stream",
            Some(&device),
            b"sealed",
        );
    }
    let drive = store(&base, &["margin", "mail"], tokens(fake_drive::TOKEN));
    let keys: Vec<String> = drive
        .list("abc/")
        .await
        .unwrap()
        .into_iter()
        .map(|m| m.key)
        .collect();
    assert_eq!(
        keys,
        vec!["abc/device-1/000000000001-000000000004.seg".to_string()]
    );
    assert_eq!(drive.get(&keys[0]).await.unwrap().bytes, b"sealed");

    drive
        .put(
            "abc/device-2/000000000001-000000000002.seg",
            b"new".to_vec(),
            PutMode::Overwrite,
        )
        .await
        .unwrap();
    let state = state.lock().unwrap();
    let folders = |name: &str| state.files.iter().filter(|f| f.name == name).count();
    assert_eq!(
        (
            folders("margin"),
            folders("mail"),
            folders("abc"),
            folders("device-2")
        ),
        (1, 1, 1, 1)
    );
}

#[tokio::test]
async fn token_failures_are_unauthorized() {
    let (base, _) = fake_drive::start().await;
    let failing: TokenProvider = Arc::new(|| Box::pin(async { Err("signed out".to_string()) }));
    assert!(matches!(
        store(&base, &["m"], failing).get("k").await,
        Err(Error::Unauthorized { .. })
    ));
    assert!(matches!(
        store(&base, &["m"], tokens("stale")).get("k").await,
        Err(Error::Unauthorized { .. })
    ));
}
