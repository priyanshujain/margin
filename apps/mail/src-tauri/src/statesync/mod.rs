// Mail's decisions roaming through the suite's sync store, by way of margin-sync.
//
// Each account's state journal is one space, `mail/<account-hash>`, in the store configured once
// for the machine in the shared directory. Backups written by builds before margin-sync are read
// alongside it (legacy.rs) while a key for them is known.

pub mod legacy;
pub mod replica;

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use margin_sync::{Key, Phrase, RootKey, Space, Storage, Suite, SyncError};
use objstore::ObjectStore;
use serde::Serialize;
use tauri::Manager;
use tauri_plugin_dialog::DialogExt;

use crate::db::Db;
use crate::dto::Account;
use replica::{Account as AccountJournal, StateReplica};

pub const APP: &str = "mail";

const POLL_SECS: u64 = 45;

#[derive(Default)]
pub struct SyncState {
    running: tokio::sync::Mutex<()>,
    last_error: std::sync::Mutex<Option<String>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    pub storage: Option<Storage>,
    pub has_secret: bool,
    pub has_phrase: bool,
    pub legacy_backup: bool,
    pub last_synced_ms: Option<i64>,
    pub error: Option<String>,
    pub drive_accounts: Vec<String>,
    pub shared: bool,
}

fn shared_dir() -> Option<std::path::PathBuf> {
    margin_shell::dirs::shared_dir(margin_shell::APP_GROUP)
}

pub fn suite(app: &tauri::AppHandle) -> Result<Suite, String> {
    match shared_dir() {
        Some(dir) => Ok(Suite::new(dir)),
        None => Ok(Suite::new(margin_shell::dirs::app_data_dir(app)?)),
    }
}

pub fn space_root(email: &str) -> String {
    format!("{APP}/{}", legacy::account_hash(email))
}

fn app_key(root: &RootKey) -> Key {
    root.app_key(APP)
}

fn device_name() -> String {
    tauri_plugin_os::hostname()
}

fn drive_token(app: &tauri::AppHandle, account_id: &str) -> margin_sync::TokenProvider {
    let app = app.clone();
    let account_id = account_id.to_string();
    Arc::new(move || {
        let app = app.clone();
        let account_id = account_id.clone();
        Box::pin(async move {
            let granted = crate::accounts::granted_scopes(&app, &account_id)?;
            if !crate::google::drive::has_scope(&granted) {
                return Err(crate::google::drive::REAUTH_MESSAGE.to_string());
            }
            let state = app
                .try_state::<crate::google::AuthState>()
                .ok_or("this app has no Google session store")?;
            crate::google::auth::valid_access_token(&app, &state, &account_id).await
        })
    })
}

fn account_by_email(app: &tauri::AppHandle, email: &str) -> Result<Account, String> {
    crate::accounts::list(app)?
        .into_iter()
        .find(|account| account.email.eq_ignore_ascii_case(email))
        .ok_or_else(|| format!("Sign in to {email} in Mail to sync through its Google Drive."))
}

fn open_store(app: &tauri::AppHandle, storage: &Storage) -> Result<Box<dyn ObjectStore>, String> {
    let token = match storage {
        Storage::GoogleDrive { account } => {
            Some(drive_token(app, &account_by_email(app, account)?.id))
        }
        _ => None,
    };
    storage.open(token).map_err(|e| e.to_string())
}

/// Builds before margin-sync kept their store choice in `settings.json` and the R2 keys in
/// Mail's own token file. The first launch after the update carries that choice into the shared
/// config, leaving the old settings in place for an older build.
pub fn migrate_config(app: &tauri::AppHandle) -> Result<(), String> {
    let suite = suite(app)?;
    if suite.storage()?.is_some() {
        return Ok(());
    }
    let legacy = crate::settings::load(app)?.backup;
    let storage = match legacy.store.as_str() {
        "r2" => legacy::stored_r2()?.map(|r2| Storage::S3 {
            endpoint: r2.endpoint,
            region: "auto".to_string(),
            bucket: r2.bucket,
            access_key: r2.access_key,
            secret: r2.secret,
            prefix: None,
            allow_http: false,
        }),
        "drive" => crate::accounts::list(app)?
            .into_iter()
            .find(|account| crate::google::drive::has_scope(&account.granted_scopes))
            .map(|account| Storage::GoogleDrive {
                account: account.email,
            }),
        _ => None,
    };
    match storage {
        Some(storage) => suite.set_storage(&storage),
        None => Ok(()),
    }
}

/// Where this account's pre-margin-sync segments may be: the old settings' store, and the
/// current store at the old layout, for a device restoring an older build's backup.
fn legacy_sources(
    app: &tauri::AppHandle,
    storage: &Storage,
    account: &Account,
    hash: &str,
) -> Vec<(Option<Box<dyn ObjectStore>>, String)> {
    let mut out: Vec<(Option<Box<dyn ObjectStore>>, String)> = Vec::new();
    let old = crate::settings::load(app)
        .map(|s| s.backup.store)
        .unwrap_or_default();
    match storage {
        Storage::S3 { .. } => out.push((None, format!("{hash}/"))),
        Storage::GoogleDrive { account: email } if email.eq_ignore_ascii_case(&account.email) => {
            out.push((None, format!("{APP}/{hash}/")))
        }
        _ => {}
    }
    if old == "drive"
        && crate::google::drive::has_scope(&account.granted_scopes)
        && !matches!(storage, Storage::GoogleDrive { account: email } if email.eq_ignore_ascii_case(&account.email))
        && let Ok(store) = (Storage::GoogleDrive {
            account: account.email.clone(),
        })
        .open(Some(drive_token(app, &account.id)))
    {
        out.push((Some(store), format!("{APP}/{hash}/")));
    }
    if old == "r2"
        && !matches!(storage, Storage::S3 { .. })
        && let Ok(Some(r2)) = legacy::stored_r2()
        && let Ok(store) = (Storage::S3 {
            endpoint: r2.endpoint,
            region: "auto".to_string(),
            bucket: r2.bucket,
            access_key: r2.access_key,
            secret: r2.secret,
            prefix: None,
            allow_http: false,
        })
        .open(None)
    {
        out.push((Some(store), format!("{hash}/")));
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Depth {
    /// Poll first and only run a pass when something changed.
    Poll,
    /// A pass for every account, with the legacy layout read too.
    Full,
}

async fn run(app: &tauri::AppHandle, depth: Depth) -> Result<usize, String> {
    let state = app
        .try_state::<SyncState>()
        .ok_or("sync is not set up yet")?;
    let _guard = state.running.lock().await;
    let result = run_locked(app, depth).await;
    *state.last_error.lock().map_err(|e| e.to_string())? = result.as_ref().err().cloned();
    result
}

async fn run_locked(app: &tauri::AppHandle, depth: Depth) -> Result<usize, String> {
    let suite = suite(app)?;
    let Some(storage) = suite.storage()? else {
        return Ok(0);
    };
    let Some(root) = suite.root_key()? else {
        return Ok(0);
    };
    let key = app_key(&root);
    let store = open_store(app, &storage)?;
    let db = app
        .try_state::<Db>()
        .ok_or("the databases are not open yet")?;
    let legacy_key = if depth == Depth::Full {
        legacy::stored_key()?
    } else {
        None
    };
    let mut applied = 0;
    for account in crate::accounts::list(app)? {
        let replica = StateReplica(AccountJournal {
            db: db.inner(),
            account_id: &account.id,
        });
        let hash = legacy::account_hash(&account.email);
        if let Some(legacy_key) = &legacy_key {
            for (own, prefix) in legacy_sources(app, &storage, &account, &hash) {
                let source: &dyn ObjectStore = own.as_deref().unwrap_or(store.as_ref());
                applied += legacy::import(source, &prefix, &hash, legacy_key, &replica).await?;
            }
        }
        let space = Space::new(store.as_ref(), &space_root(&account.email), key.clone())
            .map_err(String::from)?
            .with_device_name(device_name())
            .with_store_id(storage.identity());
        if depth == Depth::Poll && !space.poll(&replica).await.map_err(String::from)? {
            continue;
        }
        applied += space.sync(&replica).await.map_err(String::from)?.applied;
    }
    if applied > 0 {
        crate::emit_store_changed(app, "state threads");
    }
    Ok(applied)
}

fn status(app: &tauri::AppHandle) -> Result<SyncStatus, String> {
    let suite = suite(app)?;
    let storage = suite.storage()?;
    let accounts = crate::accounts::list(app)?;
    let mut last_synced_ms = None;
    if let Some(db) = app.try_state::<Db>() {
        for account in &accounts {
            let replica = StateReplica(AccountJournal {
                db: db.inner(),
                account_id: &account.id,
            });
            if let Ok(Some(at)) = margin_sync::last_synced_ms(&replica) {
                last_synced_ms = last_synced_ms.max(Some(at));
            }
        }
    }
    let error = app
        .try_state::<SyncState>()
        .and_then(|state| state.last_error.lock().ok().and_then(|e| e.clone()));
    Ok(SyncStatus {
        has_secret: storage
            .as_ref()
            .and_then(|s| s.secret())
            .is_some_and(|s| !s.is_empty()),
        storage,
        has_phrase: suite.phrase()?.is_some(),
        legacy_backup: legacy::stored_key()?.is_some(),
        last_synced_ms,
        error,
        drive_accounts: accounts
            .iter()
            .filter(|a| crate::google::drive::has_scope(&a.granted_scopes))
            .map(|a| a.email.clone())
            .collect(),
        shared: shared_dir().is_some(),
    })
}

/// Opens every account's space with a key, so a phrase that does not belong to this storage is
/// refused before anything is saved or written.
async fn check(app: &tauri::AppHandle, storage: &Storage, key: &Key) -> Result<(), String> {
    let store = open_store(app, storage)?;
    store
        .list(&format!("{APP}/"))
        .await
        .map_err(|e| format!("Could not reach the storage: {e}"))?;
    for account in crate::accounts::list(app)? {
        let space = Space::new(store.as_ref(), &space_root(&account.email), key.clone())
            .map_err(String::from)?;
        match space.check_key().await {
            Ok(_) => {}
            Err(SyncError::WrongKey { .. }) => {
                return Err(
                    "That recovery phrase does not open the sync data already in this storage."
                        .to_string(),
                );
            }
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(())
}

/// A folder to sync through: absolute, an existing directory, and outside this app's own data,
/// which a sync folder inside would copy into itself.
pub fn checked_folder(path: &Path, data_dir: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute() {
        return Err("Choose a folder by its full path.".to_string());
    }
    if !path.is_dir() {
        return Err(format!(
            "{} is not a folder on this machine.",
            path.display()
        ));
    }
    let folder = path.canonicalize().map_err(|e| e.to_string())?;
    let data = data_dir
        .canonicalize()
        .unwrap_or_else(|_| data_dir.to_path_buf());
    if folder.starts_with(&data) {
        return Err("Choose a folder outside Margin Mail's own data.".to_string());
    }
    Ok(folder)
}

fn checked_storage(app: &tauri::AppHandle, storage: Storage) -> Result<Storage, String> {
    match storage {
        Storage::Folder { path } => {
            let data = margin_shell::dirs::app_data_dir(app)?;
            let folder = checked_folder(Path::new(path.trim()), &data)?;
            Ok(Storage::Folder {
                path: folder.to_string_lossy().to_string(),
            })
        }
        other => Ok(other),
    }
}

/// The native folder picker. `None` when cancelled.
#[tauri::command]
pub async fn state_sync_pick_folder(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<Option<String>, String> {
    let Some(picked) = window
        .dialog()
        .file()
        .set_parent(&window)
        .blocking_pick_folder()
    else {
        return Ok(None);
    };
    let path = picked.into_path().map_err(|e| e.to_string())?;
    let data = margin_shell::dirs::app_data_dir(&app)?;
    Ok(Some(
        checked_folder(&path, &data)?.to_string_lossy().to_string(),
    ))
}

pub fn setup(app: tauri::AppHandle) {
    app.manage(SyncState::default());
    if let Err(e) = migrate_config(&app) {
        margin_log::note("statesync", &format!("config migration failed: {e}"));
    }
    tauri::async_runtime::spawn(async move {
        let _ = run(&app, Depth::Full).await;
        let mut ticks = tokio::time::interval(Duration::from_secs(POLL_SECS));
        ticks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticks.tick().await;
            if let Err(e) = run(&app, Depth::Poll).await {
                margin_log::note("statesync", &e);
            }
        }
    });
}

#[tauri::command]
pub async fn state_sync_status(app: tauri::AppHandle) -> Result<SyncStatus, String> {
    status(&app)
}

/// `None` turns sync off on this machine for every app. A blank secret keeps the one stored.
#[tauri::command]
pub async fn state_sync_configure(
    app: tauri::AppHandle,
    storage: Option<Storage>,
) -> Result<SyncStatus, String> {
    let suite = suite(&app)?;
    let Some(storage) = storage else {
        suite.clear_storage()?;
        crate::settings::settings_set(
            app.clone(),
            serde_json::json!({ "backup": { "store": "none", "configured": false } }),
        )?;
        return status(&app);
    };
    let mut storage = checked_storage(&app, storage)?;
    if storage.secret().is_some_and(str::is_empty)
        && let Some(held) = suite.storage()?
        && held.kind() == storage.kind()
        && let Some(secret) = held.secret()
    {
        storage.set_secret(secret.to_string());
    }
    if let Some(root) = suite.root_key()? {
        check(&app, &storage, &app_key(&root)).await?;
    } else {
        open_store(&app, &storage)?
            .list(&format!("{APP}/"))
            .await
            .map_err(|e| format!("Could not reach the storage: {e}"))?;
    }
    suite.set_storage(&storage)?;
    let _ = run(&app, Depth::Full).await;
    status(&app)
}

/// A new suite recovery phrase, for the first device. Refused when one exists.
#[tauri::command]
pub async fn state_sync_phrase_create(app: tauri::AppHandle) -> Result<String, String> {
    let suite = suite(&app)?;
    if suite.phrase()?.is_some() {
        return Err("This machine already has a recovery phrase.".to_string());
    }
    let phrase = Phrase::generate().map_err(|e| e.to_string())?;
    let words = phrase.as_str().to_string();
    tauri::async_runtime::spawn_blocking(move || suite.set_phrase(&phrase))
        .await
        .map_err(|e| e.to_string())??;
    Ok(words)
}

#[tauri::command]
pub async fn state_sync_phrase_show(app: tauri::AppHandle) -> Result<String, String> {
    suite(&app)?
        .phrase()?
        .map(|phrase| phrase.as_str().to_string())
        .ok_or_else(|| "There is no recovery phrase on this machine yet.".to_string())
}

/// Attaches this machine to an existing sync store, or adopts a phrase from an older Mail
/// backup as the suite's. Nothing is saved until the phrase opens what the storage holds.
#[tauri::command]
pub async fn state_sync_phrase_enter(
    app: tauri::AppHandle,
    phrase: String,
) -> Result<SyncStatus, String> {
    let phrase = Phrase::parse(&phrase).map_err(|e| e.to_string())?;
    let derive = phrase.clone();
    let (root, mail_key) = tauri::async_runtime::spawn_blocking(move || {
        let root = RootKey::from_phrase(&derive).map_err(|e| e.to_string())?;
        let mail = margin_crypt::mail_backup_key(&derive).map_err(|e| e.to_string())?;
        Ok::<_, String>((root, mail))
    })
    .await
    .map_err(|e| e.to_string())??;
    let suite = suite(&app)?;
    if let Some(storage) = suite.storage()? {
        check(&app, &storage, &app_key(&root)).await?;
    }
    suite.set_phrase_with_key(&phrase, &root)?;
    if legacy::stored_key()?.is_none() {
        legacy::remember_key(&mail_key)?;
    }
    let _ = run(&app, Depth::Full).await;
    status(&app)
}

#[tauri::command]
pub async fn state_sync_now(app: tauri::AppHandle) -> Result<SyncStatus, String> {
    run(&app, Depth::Full).await?;
    status(&app)
}

/// What the window runs when it gains focus.
#[tauri::command]
pub async fn state_sync_poll(app: tauri::AppHandle) -> Result<SyncStatus, String> {
    let _ = run(&app, Depth::Poll).await;
    status(&app)
}
