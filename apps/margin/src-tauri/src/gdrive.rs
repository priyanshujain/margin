use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant, UNIX_EPOCH};
use tauri::{Emitter, Manager};

const FOLDER_NAME: &str = "margin";
const DICTIONARY_NAME: &str = crate::proofing::DICTIONARY_NAME;
const STATE_NAME: &str = "backup.json";
const DRIVE_SCOPE: &str = "https://www.googleapis.com/auth/drive.file";
const SCOPES: &str = "openid email https://www.googleapis.com/auth/drive.file";

const TOKEN_FILE: &str = "gdrive-token.enc";
const SALT_FILE: &str = "gdrive-token.salt";
const TOKEN_CONTEXT: &str = "margin gdrive token v1";

const CREDENTIALS_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../google-credentials.json"
));

const NOT_SET_UP: &str =
    "Google Drive is not set up yet. Add a real OAuth desktop client to google-credentials.json.";

const PAGES: margin_oauth::Pages<'static> = margin_oauth::Pages {
    app: "Margin",
    provider: "Google",
};

static HTTP: LazyLock<reqwest::Client> =
    LazyLock::new(|| margin_http::client(margin_http::Profile::Api));

fn load_credentials() -> Result<margin_oauth::Client, String> {
    margin_google::client(CREDENTIALS_JSON, NOT_SET_UP)
}

/// The sealed refresh token. Same files, context and format the app has always written.
pub fn token_store(dir: &Path) -> margin_secrets::Single {
    margin_secrets::Single::new(
        margin_secrets::Vault::new(dir, SALT_FILE, TOKEN_CONTEXT),
        TOKEN_FILE,
    )
}

#[derive(Default)]
pub struct GDriveState(pub Mutex<Session>, pub margin_oauth::Attempts);

#[derive(Default)]
pub struct Session {
    refresh_token: Option<String>,
    access_token: Option<String>,
    access_expiry: u64,
    email: Option<String>,
    folder_id: Option<String>,
    scopes: Option<String>,
}

#[derive(Serialize, Deserialize, Default)]
struct BackupState {
    #[serde(default, skip_serializing)]
    refresh_token: Option<String>,
    email: Option<String>,
    folder_id: Option<String>,
    last_backup: Option<u64>,
    #[serde(default)]
    scopes: Option<String>,
    #[serde(default)]
    files: HashMap<String, FileRecord>,
}

#[derive(Serialize, Deserialize, Clone)]
struct FileRecord {
    hash: String,
    drive_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    connected: bool,
    email: Option<String>,
    last_backup: Option<u64>,
    pending: bool,
    needs_reauth: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreResult {
    restored: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncOutcome {
    uploaded: usize,
    downloaded: usize,
    #[serde(flatten)]
    status: Status,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupOutcome {
    uploaded: usize,
    #[serde(flatten)]
    status: Status,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteBackup {
    name: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AuthEvent {
    ok: bool,
    error: Option<String>,
    cancelled: bool,
}

fn now() -> u64 {
    margin_oauth::now()
}

fn dictionary_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(crate::library::app_data_dir(app)?.join(DICTIONARY_NAME))
}

fn load_state(app: &tauri::AppHandle) -> BackupState {
    match crate::library::app_data_dir(app) {
        Ok(dir) => load_state_in(&dir),
        Err(_) => BackupState::default(),
    }
}

fn save_state(app: &tauri::AppHandle, state: &BackupState) -> Result<(), String> {
    save_state_in(&crate::library::app_data_dir(app)?, state)
}

fn load_state_in(dir: &Path) -> BackupState {
    let mut state: BackupState = match fs::read_to_string(dir.join(STATE_NAME)) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
        Err(_) => BackupState::default(),
    };
    if state.refresh_token.is_some() {
        let _ = save_state_in(dir, &state);
    } else {
        state.refresh_token = token_store(dir).load().ok().flatten();
    }
    state
}

fn save_state_in(dir: &Path, state: &BackupState) -> Result<(), String> {
    let store = token_store(dir);
    match &state.refresh_token {
        Some(token) => store.store(token)?,
        None => store.delete()?,
    }
    let contents = serde_json::to_string_pretty(state).map_err(|e| e.to_string())?;
    crate::project::atomic_write(&dir.join(STATE_NAME), contents.as_bytes(), false)
}

fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn has_drive_scope(scope: &str) -> bool {
    scope.split_whitespace().any(|s| s == DRIVE_SCOPE)
}

const REAUTH_MESSAGE: &str = "margin needs the \"See, edit, create and delete only the specific Google Drive files you use with this app\" permission. Connect again and tick that box.";

fn drive_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\'', "\\'")
}

fn safe_book_name(name: &str) -> bool {
    match name.strip_suffix(".margin") {
        Some(stem) => {
            !stem.is_empty()
                && stem
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        }
        None => false,
    }
}

#[derive(Deserialize)]
struct DriveFile {
    id: String,
    #[serde(default)]
    name: String,
}

#[derive(Deserialize)]
struct FileList {
    #[serde(default)]
    files: Vec<DriveFile>,
    #[serde(default, rename = "nextPageToken")]
    next_page_token: Option<String>,
}

async fn read_json<T: DeserializeOwned>(
    resp: reqwest::Response,
    context: &str,
) -> Result<T, String> {
    margin_google::read_json(resp, context, DRIVE_SCOPE)
        .await
        .map_err(String::from)
}

async fn ensure_folder(access_token: &str) -> Result<String, String> {
    let query = format!(
        "name = '{}' and mimeType = 'application/vnd.google-apps.folder' and trashed = false",
        drive_escape(FOLDER_NAME)
    );
    let resp = HTTP
        .get("https://www.googleapis.com/drive/v3/files")
        .bearer_auth(access_token)
        .query(&[
            ("q", query.as_str()),
            ("fields", "files(id,name)"),
            ("spaces", "drive"),
            ("pageSize", "1"),
        ])
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let list: FileList = read_json(resp, "Drive folder lookup").await?;
    if let Some(folder) = list.files.into_iter().next() {
        return Ok(folder.id);
    }
    let body = serde_json::json!({
        "name": FOLDER_NAME,
        "mimeType": "application/vnd.google-apps.folder",
    });
    let resp = HTTP
        .post("https://www.googleapis.com/drive/v3/files")
        .bearer_auth(access_token)
        .query(&[("fields", "id")])
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let created: DriveFile = read_json(resp, "Drive folder creation").await?;
    Ok(created.id)
}

async fn find_file(
    access_token: &str,
    folder_id: &str,
    name: &str,
) -> Result<Option<DriveFile>, String> {
    let query = format!(
        "'{}' in parents and name = '{}' and trashed = false",
        drive_escape(folder_id),
        drive_escape(name)
    );
    let resp = HTTP
        .get("https://www.googleapis.com/drive/v3/files")
        .bearer_auth(access_token)
        .query(&[
            ("q", query.as_str()),
            ("fields", "files(id,name)"),
            ("spaces", "drive"),
            ("pageSize", "1"),
        ])
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let list: FileList = read_json(resp, "Drive file lookup").await?;
    Ok(list.files.into_iter().next())
}

async fn list_in_folder(access_token: &str, folder_id: &str) -> Result<Vec<DriveFile>, String> {
    let query = format!(
        "'{}' in parents and trashed = false",
        drive_escape(folder_id)
    );
    let mut files = Vec::new();
    let mut page_token: Option<String> = None;
    loop {
        let mut request = HTTP
            .get("https://www.googleapis.com/drive/v3/files")
            .bearer_auth(access_token)
            .query(&[
                ("q", query.as_str()),
                ("fields", "nextPageToken,files(id,name)"),
                ("spaces", "drive"),
                ("pageSize", "100"),
            ]);
        if let Some(token) = &page_token {
            request = request.query(&[("pageToken", token.as_str())]);
        }
        let resp = request.send().await.map_err(|e| e.to_string())?;
        let FileList {
            files: page,
            next_page_token,
        } = read_json(resp, "Drive file list").await?;
        files.extend(page);
        match next_page_token {
            Some(token) => page_token = Some(token),
            None => break,
        }
    }
    Ok(files)
}

async fn upload_file(
    access_token: &str,
    folder_id: &str,
    name: &str,
    bytes: &[u8],
    existing_id: Option<String>,
) -> Result<DriveFile, String> {
    let boundary = "margin7f3e2a1b9c8d";
    let metadata = match &existing_id {
        Some(_) => serde_json::json!({ "name": name }),
        None => serde_json::json!({ "name": name, "parents": [folder_id] }),
    };
    let mut body: Vec<u8> = Vec::new();
    body.extend_from_slice(
        format!("--{boundary}\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n").as_bytes(),
    );
    body.extend_from_slice(
        serde_json::to_string(&metadata)
            .unwrap_or_default()
            .as_bytes(),
    );
    body.extend_from_slice(
        format!("\r\n--{boundary}\r\nContent-Type: application/octet-stream\r\n\r\n").as_bytes(),
    );
    body.extend_from_slice(bytes);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

    let request = match &existing_id {
        Some(id) => HTTP.patch(format!(
            "https://www.googleapis.com/upload/drive/v3/files/{id}"
        )),
        None => HTTP.post("https://www.googleapis.com/upload/drive/v3/files"),
    };
    let resp = request
        .bearer_auth(access_token)
        .query(&[("uploadType", "multipart"), ("fields", "id")])
        .header(
            reqwest::header::CONTENT_TYPE,
            format!("multipart/related; boundary={boundary}"),
        )
        .body(body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    read_json(resp, &format!("Drive upload of {name}")).await
}

async fn download_file(access_token: &str, id: &str) -> Result<Vec<u8>, String> {
    let resp = HTTP
        .get(format!("https://www.googleapis.com/drive/v3/files/{id}"))
        .bearer_auth(access_token)
        .query(&[("alt", "media")])
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Drive download failed ({status}): {text}"));
    }
    Ok(resp.bytes().await.map_err(|e| e.to_string())?.to_vec())
}

async fn valid_access_token(app: &tauri::AppHandle, state: &GDriveState) -> Result<String, String> {
    let (access, expiry, refresh) = {
        let session = state.0.lock().unwrap();
        (
            session.access_token.clone(),
            session.access_expiry,
            session.refresh_token.clone(),
        )
    };
    if let Some(token) = access
        && now() < expiry
    {
        return Ok(token);
    }
    let refresh = refresh.ok_or("Not connected to Google Drive.")?;
    let creds = load_credentials()?;
    let tokens = match margin_oauth::refresh_access_token(&creds, &refresh).await {
        Ok(tokens) => tokens,
        Err(e) => {
            if e.contains("invalid_grant") {
                forget_credentials(app, state);
                return Err(
                    "Google Drive access has expired. Connect again to keep backing up."
                        .to_string(),
                );
            }
            return Err(e);
        }
    };
    if let Some(granted) = &tokens.scope
        && !has_drive_scope(granted)
    {
        let mut session = state.0.lock().unwrap();
        session.scopes = Some(granted.clone());
        drop(session);
        let mut stored = load_state(app);
        stored.scopes = Some(granted.clone());
        let _ = save_state(app, &stored);
        return Err(REAUTH_MESSAGE.to_string());
    }
    let access_token = tokens.access_token.clone();
    {
        let mut session = state.0.lock().unwrap();
        session.access_token = Some(tokens.access_token);
        session.access_expiry = margin_oauth::expiry_at(tokens.expires_in);
        if let Some(rotated) = &tokens.refresh_token {
            session.refresh_token = Some(rotated.clone());
        }
        if let Some(granted) = &tokens.scope {
            session.scopes = Some(granted.clone());
        }
    }
    if tokens.refresh_token.is_some() || tokens.scope.is_some() {
        let mut stored = load_state(app);
        if let Some(rotated) = &tokens.refresh_token {
            stored.refresh_token = Some(rotated.clone());
        }
        if let Some(granted) = &tokens.scope {
            stored.scopes = Some(granted.clone());
        }
        let _ = save_state(app, &stored);
    }
    Ok(access_token)
}

async fn ensure_folder_id(
    app: &tauri::AppHandle,
    state: &GDriveState,
    access_token: &str,
) -> Result<String, String> {
    {
        let session = state.0.lock().unwrap();
        if let Some(folder) = &session.folder_id {
            return Ok(folder.clone());
        }
    }
    let folder_id = ensure_folder(access_token).await?;
    {
        let mut session = state.0.lock().unwrap();
        session.folder_id = Some(folder_id.clone());
    }
    let mut stored = load_state(app);
    stored.folder_id = Some(folder_id.clone());
    save_state(app, &stored)?;
    Ok(folder_id)
}

fn collect_local_files(app: &tauri::AppHandle) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut files = Vec::new();
    let library = crate::library::library_dir(app)?;
    for entry in fs::read_dir(&library).map_err(|e| e.to_string())? {
        let path = match entry {
            Ok(entry) => entry.path(),
            Err(_) => continue,
        };
        if path.extension().and_then(|e| e.to_str()) != Some("margin") {
            continue;
        }
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) if safe_book_name(name) => name.to_string(),
            _ => continue,
        };
        let bytes = fs::read(&path).map_err(|e| e.to_string())?;
        files.push((name, bytes));
    }
    let dictionary = dictionary_path(app)?;
    if dictionary.exists() {
        let bytes = fs::read(&dictionary).map_err(|e| e.to_string())?;
        files.push((DICTIONARY_NAME.to_string(), bytes));
    }
    Ok(files)
}

fn file_pending(
    path: &std::path::Path,
    name: &str,
    stored: &BackupState,
    last_backup: u64,
) -> bool {
    let record = match stored.files.get(name) {
        Some(record) => record,
        None => return true,
    };
    let touched = fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|since| since.as_secs() > last_backup)
        .unwrap_or(true);
    if !touched {
        return false;
    }
    match fs::read(path) {
        Ok(bytes) => record.hash != hash_bytes(&bytes),
        Err(_) => false,
    }
}

fn compute_pending(app: &tauri::AppHandle, stored: &BackupState) -> bool {
    let last_backup = stored.last_backup.unwrap_or(0);
    let library = match crate::library::library_dir(app) {
        Ok(dir) => dir,
        Err(_) => return false,
    };
    if let Ok(entries) = fs::read_dir(&library) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("margin") {
                continue;
            }
            if let Some(name) = path.file_name().and_then(|n| n.to_str())
                && safe_book_name(name)
                && file_pending(&path, name, stored, last_backup)
            {
                return true;
            }
        }
    }
    match dictionary_path(app) {
        Ok(dict) if dict.exists() => file_pending(&dict, DICTIONARY_NAME, stored, last_backup),
        _ => false,
    }
}

fn status_inner(app: &tauri::AppHandle, state: &GDriveState) -> Status {
    let (connected, email, scopes) = {
        let session = state.0.lock().unwrap();
        (
            session.refresh_token.is_some(),
            session.email.clone(),
            session.scopes.clone(),
        )
    };
    let stored = load_state(app);
    let pending = connected && compute_pending(app, &stored);
    let granted = scopes.or_else(|| stored.scopes.clone());
    Status {
        connected,
        email: email.or(stored.email),
        last_backup: stored.last_backup,
        pending,
        needs_reauth: connected && granted.map(|s| !has_drive_scope(&s)).unwrap_or(false),
    }
}

pub fn init_session(app: &tauri::AppHandle) {
    let state = app.state::<GDriveState>();
    let stored = load_state(app);
    let mut session = state.0.lock().unwrap();
    session.refresh_token = stored.refresh_token;
    session.email = stored.email;
    session.folder_id = stored.folder_id;
    session.scopes = stored.scopes;
}

fn forget_credentials(app: &tauri::AppHandle, state: &GDriveState) {
    {
        let mut session = state.0.lock().unwrap();
        session.refresh_token = None;
        session.access_token = None;
        session.access_expiry = 0;
        session.scopes = None;
    }
    let mut stored = load_state(app);
    stored.refresh_token = None;
    stored.scopes = None;
    let _ = save_state(app, &stored);
}

async fn complete_auth(
    app: &tauri::AppHandle,
    listener: TcpListener,
    csrf: String,
    verifier: String,
    redirect: String,
    creds: margin_oauth::Client,
    attempt: &margin_oauth::Attempt,
) -> Result<(), String> {
    let waiting = attempt.clone();
    let code = tauri::async_runtime::spawn_blocking(move || {
        let deadline = Instant::now() + Duration::from_secs(margin_oauth::AUTH_TIMEOUT_SECS);
        margin_oauth::await_code(listener, &csrf, deadline, PAGES, || waiting.is_cancelled())
    })
    .await
    .map_err(|e| e.to_string())??;
    if attempt.is_cancelled() {
        return Err(margin_oauth::CANCELLED.to_string());
    }

    let tokens = margin_oauth::exchange_code(&creds, &code, &redirect, &verifier).await?;
    let granted = tokens.scope.clone().unwrap_or_default();
    if !has_drive_scope(&granted) {
        let _ = margin_oauth::revoke(&creds, &tokens.access_token).await;
        return Err(REAUTH_MESSAGE.to_string());
    }
    let email = margin_oauth::fetch_email(&creds, &tokens.access_token).await?;
    let folder_id = ensure_folder(&tokens.access_token).await?;
    if attempt.is_cancelled() {
        let _ = margin_oauth::revoke(&creds, &tokens.access_token).await;
        return Err(margin_oauth::CANCELLED.to_string());
    }

    {
        let state = app.state::<GDriveState>();
        let mut session = state.0.lock().unwrap();
        if let Some(refresh) = &tokens.refresh_token {
            session.refresh_token = Some(refresh.clone());
        }
        session.access_token = Some(tokens.access_token.clone());
        session.access_expiry = margin_oauth::expiry_at(tokens.expires_in);
        session.email = Some(email.clone());
        session.folder_id = Some(folder_id.clone());
        session.scopes = Some(granted.clone());
    }

    let mut stored = load_state(app);
    if let Some(refresh) = &tokens.refresh_token {
        stored.refresh_token = Some(refresh.clone());
    }
    stored.email = Some(email);
    stored.folder_id = Some(folder_id);
    stored.scopes = Some(granted);
    save_state(app, &stored)?;
    Ok(())
}

#[tauri::command]
pub async fn gdrive_connect(app: tauri::AppHandle) -> Result<String, String> {
    let creds = load_credentials()?;
    let attempt = app.state::<GDriveState>().1.begin();
    let verifier = margin_oauth::random_b64(64);
    let challenge = margin_oauth::pkce_challenge(&verifier);
    let csrf = margin_oauth::random_b64(24);

    let (listener, redirect) = margin_oauth::bind_loopback()?;
    let url = margin_oauth::auth_url(&creds, &redirect, &challenge, &csrf, SCOPES, None);
    margin_oauth::open_in_browser(&app, &url);

    let app_bg = app.clone();
    tauri::async_runtime::spawn(async move {
        let outcome =
            complete_auth(&app_bg, listener, csrf, verifier, redirect, creds, &attempt).await;
        let event = match outcome {
            Ok(()) => AuthEvent {
                ok: true,
                error: None,
                cancelled: false,
            },
            Err(error) => AuthEvent {
                ok: false,
                cancelled: error == margin_oauth::CANCELLED,
                error: Some(error),
            },
        };
        let _ = app_bg.emit("gdrive-auth", event);
    });

    Ok(url)
}

#[tauri::command]
pub fn gdrive_connect_cancel(state: tauri::State<'_, GDriveState>) {
    state.1.cancel();
}

#[tauri::command]
pub async fn gdrive_disconnect(
    app: tauri::AppHandle,
    state: tauri::State<'_, GDriveState>,
) -> Result<Status, String> {
    let refresh = {
        let session = state.0.lock().unwrap();
        session.refresh_token.clone()
    };
    if let Some(refresh) = refresh
        && let Ok(creds) = load_credentials()
    {
        let _ = margin_oauth::revoke(&creds, &refresh).await;
    }
    {
        let mut session = state.0.lock().unwrap();
        session.refresh_token = None;
        session.access_token = None;
        session.access_expiry = 0;
        session.email = None;
        session.scopes = None;
    }
    let mut stored = load_state(&app);
    stored.refresh_token = None;
    stored.email = None;
    stored.scopes = None;
    save_state(&app, &stored)?;
    Ok(status_inner(&app, &state))
}

#[tauri::command]
pub async fn gdrive_status(
    app: tauri::AppHandle,
    state: tauri::State<'_, GDriveState>,
) -> Result<Status, String> {
    Ok(status_inner(&app, &state))
}

async fn push_local_files(
    app: &tauri::AppHandle,
    access_token: &str,
    folder_id: &str,
    stored: &mut BackupState,
) -> Result<usize, String> {
    let files = collect_local_files(app)?;
    let mut uploaded = 0;
    for (name, bytes) in &files {
        let hash = hash_bytes(bytes);
        let existing = stored.files.get(name).cloned();
        if let Some(record) = &existing
            && record.hash == hash
        {
            continue;
        }
        let drive_id = match existing.filter(|record| !record.drive_id.is_empty()) {
            Some(record) => Some(record.drive_id),
            None => find_file(access_token, folder_id, name)
                .await?
                .map(|file| file.id),
        };
        let result = upload_file(access_token, folder_id, name, bytes, drive_id).await?;
        stored.files.insert(
            name.clone(),
            FileRecord {
                hash,
                drive_id: result.id,
            },
        );
        uploaded += 1;
    }
    Ok(uploaded)
}

#[tauri::command]
pub async fn gdrive_backup(
    app: tauri::AppHandle,
    state: tauri::State<'_, GDriveState>,
) -> Result<BackupOutcome, String> {
    let access_token = valid_access_token(&app, &state).await?;
    let folder_id = ensure_folder_id(&app, &state, &access_token).await?;
    let mut stored = load_state(&app);
    let uploaded = push_local_files(&app, &access_token, &folder_id, &mut stored).await?;
    if uploaded > 0 {
        stored.last_backup = Some(now());
    }
    save_state(&app, &stored)?;
    Ok(BackupOutcome {
        uploaded,
        status: Status {
            connected: true,
            email: stored.email.clone(),
            last_backup: stored.last_backup,
            pending: false,
            needs_reauth: false,
        },
    })
}

#[tauri::command]
pub async fn gdrive_sync(
    app: tauri::AppHandle,
    state: tauri::State<'_, GDriveState>,
) -> Result<SyncOutcome, String> {
    let access_token = valid_access_token(&app, &state).await?;
    let folder_id = ensure_folder_id(&app, &state, &access_token).await?;
    let remote = list_in_folder(&access_token, &folder_id).await?;
    let mut stored = load_state(&app);
    let mut downloaded = 0;

    for file in &remote {
        let is_dictionary = file.name == DICTIONARY_NAME;
        if !is_dictionary && !safe_book_name(&file.name) {
            continue;
        }
        let destination = if is_dictionary {
            dictionary_path(&app)?
        } else {
            crate::library::library_dir(&app)?.join(&file.name)
        };
        if destination.exists() {
            let record = stored
                .files
                .entry(file.name.clone())
                .or_insert_with(|| FileRecord {
                    hash: String::new(),
                    drive_id: file.id.clone(),
                });
            record.drive_id = file.id.clone();
            continue;
        }
        let bytes = download_file(&access_token, &file.id).await?;
        crate::project::atomic_write(&destination, &bytes, false)?;
        stored.files.insert(
            file.name.clone(),
            FileRecord {
                hash: hash_bytes(&bytes),
                drive_id: file.id.clone(),
            },
        );
        downloaded += 1;
    }

    let uploaded = push_local_files(&app, &access_token, &folder_id, &mut stored).await?;
    if uploaded > 0 || downloaded > 0 {
        stored.last_backup = Some(now());
    }
    save_state(&app, &stored)?;
    Ok(SyncOutcome {
        uploaded,
        downloaded,
        status: Status {
            connected: true,
            email: stored.email.clone(),
            last_backup: stored.last_backup,
            pending: false,
            needs_reauth: false,
        },
    })
}

#[tauri::command]
pub async fn gdrive_restore(
    app: tauri::AppHandle,
    state: tauri::State<'_, GDriveState>,
) -> Result<RestoreResult, String> {
    let access_token = valid_access_token(&app, &state).await?;
    let folder_id = ensure_folder_id(&app, &state, &access_token).await?;
    let remote = list_in_folder(&access_token, &folder_id).await?;
    let mut stored = load_state(&app);
    let mut restored = 0;
    for file in &remote {
        let is_dictionary = file.name == DICTIONARY_NAME;
        let is_book = safe_book_name(&file.name);
        if !is_dictionary && !is_book {
            continue;
        }
        let bytes = download_file(&access_token, &file.id).await?;
        let destination = if is_dictionary {
            dictionary_path(&app)?
        } else {
            crate::library::library_dir(&app)?.join(&file.name)
        };
        crate::project::atomic_write(&destination, &bytes, false)?;
        stored.files.insert(
            file.name.clone(),
            FileRecord {
                hash: hash_bytes(&bytes),
                drive_id: file.id.clone(),
            },
        );
        restored += 1;
    }
    stored.last_backup = Some(now());
    save_state(&app, &stored)?;
    Ok(RestoreResult { restored })
}

#[tauri::command]
pub async fn gdrive_list_backups(
    app: tauri::AppHandle,
    state: tauri::State<'_, GDriveState>,
) -> Result<Vec<RemoteBackup>, String> {
    let access_token = valid_access_token(&app, &state).await?;
    let folder_id = ensure_folder_id(&app, &state, &access_token).await?;
    let remote = list_in_folder(&access_token, &folder_id).await?;
    Ok(remote
        .into_iter()
        .filter(|file| safe_book_name(&file.name))
        .map(|file| RemoteBackup { name: file.name })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plaintext_token_moves_into_the_sealed_file() {
        let dir = tempfile::tempdir().unwrap();
        let legacy = r#"{"refresh_token":"1//legacy","email":"a@b.c","folder_id":"f","last_backup":5,"files":{}}"#;
        fs::write(dir.path().join(STATE_NAME), legacy).unwrap();

        let state = load_state_in(dir.path());
        assert_eq!(state.refresh_token.as_deref(), Some("1//legacy"));
        let written = fs::read_to_string(dir.path().join(STATE_NAME)).unwrap();
        assert!(!written.contains("1//legacy"));
        assert!(dir.path().join(TOKEN_FILE).exists());
        assert!(dir.path().join(SALT_FILE).exists());

        let again = load_state_in(dir.path());
        assert_eq!(again.refresh_token.as_deref(), Some("1//legacy"));
        assert_eq!(again.email.as_deref(), Some("a@b.c"));
        assert_eq!(again.last_backup, Some(5));
    }

    #[test]
    fn forgetting_the_token_deletes_the_sealed_file() {
        let dir = tempfile::tempdir().unwrap();
        let mut state = BackupState {
            refresh_token: Some("t".into()),
            ..Default::default()
        };
        save_state_in(dir.path(), &state).unwrap();
        assert_eq!(
            token_store(dir.path()).load().unwrap().as_deref(),
            Some("t")
        );
        state.refresh_token = None;
        save_state_in(dir.path(), &state).unwrap();
        assert!(!dir.path().join(TOKEN_FILE).exists());
        assert_eq!(load_state_in(dir.path()).refresh_token, None);
    }

    #[test]
    fn only_plain_book_names_are_backed_up() {
        assert!(safe_book_name("abc-1_2.margin"));
        for bad in [
            "../x.margin",
            ".margin",
            "a b.margin",
            "a.txt",
            "a/b.margin",
        ] {
            assert!(!safe_book_name(bad), "{bad}");
        }
    }
}
