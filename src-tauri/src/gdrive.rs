use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand::RngCore;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{Emitter, Manager};

const FOLDER_NAME: &str = "margin";
const DICTIONARY_NAME: &str = "custom-dictionary.txt";
const SCOPES: &str = "openid email https://www.googleapis.com/auth/drive.file";
const AUTH_TIMEOUT_SECS: u64 = 120;

const CREDENTIALS_JSON: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../google-credentials.json"));

static HTTP: LazyLock<reqwest::Client> = LazyLock::new(reqwest::Client::new);

#[derive(Deserialize)]
struct CredentialsFile {
    installed: Credentials,
}

#[derive(Deserialize)]
struct Credentials {
    client_id: String,
    client_secret: String,
    auth_uri: String,
    token_uri: String,
}

fn load_credentials() -> Result<Credentials, String> {
    let runtime_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../google-credentials.json");
    let raw = fs::read_to_string(runtime_path).unwrap_or_else(|_| CREDENTIALS_JSON.to_string());
    let parsed: CredentialsFile = serde_json::from_str(&raw)
        .map_err(|e| format!("invalid google-credentials.json: {e}"))?;
    let creds = parsed.installed;
    if creds.client_id.starts_with("YOUR_CLIENT_ID") || creds.client_secret.starts_with("YOUR_CLIENT_SECRET") {
        return Err("Google Drive is not set up yet. Add a real OAuth desktop client to google-credentials.json.".to_string());
    }
    Ok(creds)
}

#[derive(Default)]
pub struct GDriveState(pub Mutex<Session>);

#[derive(Default)]
pub struct Session {
    refresh_token: Option<String>,
    access_token: Option<String>,
    access_expiry: u64,
    email: Option<String>,
    folder_id: Option<String>,
}

#[derive(Serialize, Deserialize, Default)]
struct BackupState {
    #[serde(default)]
    refresh_token: Option<String>,
    email: Option<String>,
    folder_id: Option<String>,
    last_backup: Option<u64>,
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
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreResult {
    restored: usize,
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
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

fn dictionary_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(crate::library::app_data_dir(app)?.join(DICTIONARY_NAME))
}

fn state_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(crate::library::app_data_dir(app)?.join("backup.json"))
}

fn load_state(app: &tauri::AppHandle) -> BackupState {
    let path = match state_path(app) {
        Ok(p) => p,
        Err(_) => return BackupState::default(),
    };
    match fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
        Err(_) => BackupState::default(),
    }
}

fn save_state(app: &tauri::AppHandle, state: &BackupState) -> Result<(), String> {
    let path = state_path(app)?;
    let contents = serde_json::to_string_pretty(state).map_err(|e| e.to_string())?;
    crate::project::atomic_write(&path, contents.as_bytes(), false)
}

fn random_b64(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    rand::thread_rng().fill_bytes(&mut buf);
    URL_SAFE_NO_PAD.encode(buf)
}

fn pkce_challenge(verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(hasher.finalize())
}

fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn urlencode(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

fn drive_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\'', "\\'")
}

fn safe_book_name(name: &str) -> bool {
    match name.strip_suffix(".margin") {
        Some(stem) => !stem.is_empty() && stem.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
        None => false,
    }
}

fn write_http_message(stream: &mut TcpStream, message: &str) {
    let body = format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>margin</title></head>\
         <body style=\"font-family:system-ui,sans-serif;text-align:center;padding-top:80px;color:#222\">\
         <h2>{message}</h2></body></html>"
    );
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

fn await_code(listener: TcpListener, expected_state: &str, deadline: Instant) -> Result<String, String> {
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    loop {
        if Instant::now() > deadline {
            return Err("Timed out waiting for Google authorization.".to_string());
        }
        match listener.accept() {
            Ok((mut stream, _)) => {
                stream.set_nonblocking(false).ok();
                stream.set_read_timeout(Some(Duration::from_secs(5))).ok();
                let mut buf = [0u8; 8192];
                let n = stream.read(&mut buf).unwrap_or(0);
                let request = String::from_utf8_lossy(&buf[..n]);
                let path = request.lines().next().and_then(|line| line.split_whitespace().nth(1)).unwrap_or("");
                if path == "/favicon.ico" {
                    write_http_message(&mut stream, "margin");
                    continue;
                }
                let full = format!("http://127.0.0.1{path}");
                let parsed = match url::Url::parse(&full) {
                    Ok(p) => p,
                    Err(_) => {
                        write_http_message(&mut stream, "Waiting for Google…");
                        continue;
                    }
                };
                let mut code = None;
                let mut state = None;
                let mut error = None;
                for (key, value) in parsed.query_pairs() {
                    match key.as_ref() {
                        "code" => code = Some(value.into_owned()),
                        "state" => state = Some(value.into_owned()),
                        "error" => error = Some(value.into_owned()),
                        _ => {}
                    }
                }
                if let Some(error) = error {
                    write_http_message(&mut stream, "Authorization was cancelled. You can close this tab.");
                    return Err(format!("Google authorization failed: {error}"));
                }
                match (code, state) {
                    (Some(code), Some(state)) if state == expected_state => {
                        write_http_message(&mut stream, "Connected to margin. You can close this tab.");
                        return Ok(code);
                    }
                    (Some(_), _) => {
                        write_http_message(&mut stream, "Could not verify the request. You can close this tab.");
                        return Err("State mismatch during Google authorization.".to_string());
                    }
                    _ => {
                        write_http_message(&mut stream, "Waiting for Google…");
                    }
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(150));
            }
            Err(e) => return Err(e.to_string()),
        }
    }
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: u64,
}

#[derive(Deserialize)]
struct UserInfo {
    #[serde(default)]
    email: Option<String>,
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

async fn read_json<T: DeserializeOwned>(resp: reqwest::Response, context: &str) -> Result<T, String> {
    let status = resp.status();
    let text = resp.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!("{context} failed ({status}): {text}"));
    }
    serde_json::from_str(&text).map_err(|e| format!("{context}: could not parse response: {e}"))
}

async fn exchange_code(creds: &Credentials, code: &str, redirect: &str, verifier: &str) -> Result<TokenResponse, String> {
    let resp = HTTP
        .post(&creds.token_uri)
        .form(&[
            ("client_id", creds.client_id.as_str()),
            ("client_secret", creds.client_secret.as_str()),
            ("code", code),
            ("code_verifier", verifier),
            ("grant_type", "authorization_code"),
            ("redirect_uri", redirect),
        ])
        .send()
        .await
        .map_err(|e| e.to_string())?;
    read_json(resp, "Google token exchange").await
}

async fn refresh_access_token(creds: &Credentials, refresh_token: &str) -> Result<TokenResponse, String> {
    let resp = HTTP
        .post(&creds.token_uri)
        .form(&[
            ("client_id", creds.client_id.as_str()),
            ("client_secret", creds.client_secret.as_str()),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await
        .map_err(|e| e.to_string())?;
    read_json(resp, "Google token refresh").await
}

async fn fetch_email(access_token: &str) -> Result<String, String> {
    let resp = HTTP
        .get("https://www.googleapis.com/oauth2/v2/userinfo")
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let info: UserInfo = read_json(resp, "Google account lookup").await?;
    Ok(info.email.unwrap_or_default())
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

async fn find_file(access_token: &str, folder_id: &str, name: &str) -> Result<Option<DriveFile>, String> {
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
    let query = format!("'{}' in parents and trashed = false", drive_escape(folder_id));
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
        let FileList { files: page, next_page_token } = read_json(resp, "Drive file list").await?;
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
    body.extend_from_slice(format!("--{boundary}\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n").as_bytes());
    body.extend_from_slice(serde_json::to_string(&metadata).unwrap_or_default().as_bytes());
    body.extend_from_slice(format!("\r\n--{boundary}\r\nContent-Type: application/octet-stream\r\n\r\n").as_bytes());
    body.extend_from_slice(bytes);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

    let request = match &existing_id {
        Some(id) => HTTP.patch(format!("https://www.googleapis.com/upload/drive/v3/files/{id}")),
        None => HTTP.post("https://www.googleapis.com/upload/drive/v3/files"),
    };
    let resp = request
        .bearer_auth(access_token)
        .query(&[("uploadType", "multipart"), ("fields", "id")])
        .header(reqwest::header::CONTENT_TYPE, format!("multipart/related; boundary={boundary}"))
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
        (session.access_token.clone(), session.access_expiry, session.refresh_token.clone())
    };
    if let Some(token) = access {
        if now() < expiry {
            return Ok(token);
        }
    }
    let refresh = refresh.ok_or("Not connected to Google Drive.")?;
    let creds = load_credentials()?;
    let tokens = refresh_access_token(&creds, &refresh).await?;
    let access_token = tokens.access_token.clone();
    {
        let mut session = state.0.lock().unwrap();
        session.access_token = Some(tokens.access_token);
        session.access_expiry = now() + tokens.expires_in.saturating_sub(60);
        if let Some(rotated) = &tokens.refresh_token {
            session.refresh_token = Some(rotated.clone());
        }
    }
    if let Some(rotated) = &tokens.refresh_token {
        let mut stored = load_state(app);
        stored.refresh_token = Some(rotated.clone());
        let _ = save_state(app, &stored);
    }
    Ok(access_token)
}

async fn ensure_folder_id(app: &tauri::AppHandle, state: &GDriveState, access_token: &str) -> Result<String, String> {
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

fn file_pending(path: &std::path::Path, name: &str, stored: &BackupState, last_backup: u64) -> bool {
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
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if safe_book_name(name) && file_pending(&path, name, stored, last_backup) {
                    return true;
                }
            }
        }
    }
    match dictionary_path(app) {
        Ok(dict) if dict.exists() => file_pending(&dict, DICTIONARY_NAME, stored, last_backup),
        _ => false,
    }
}

fn status_inner(app: &tauri::AppHandle, state: &GDriveState) -> Status {
    let (connected, email) = {
        let session = state.0.lock().unwrap();
        (session.refresh_token.is_some(), session.email.clone())
    };
    let stored = load_state(app);
    let pending = connected && compute_pending(app, &stored);
    Status {
        connected,
        email: email.or(stored.email),
        last_backup: stored.last_backup,
        pending,
    }
}

pub fn init_session(app: &tauri::AppHandle) {
    let state = app.state::<GDriveState>();
    let stored = load_state(app);
    let mut session = state.0.lock().unwrap();
    session.refresh_token = stored.refresh_token;
    session.email = stored.email;
    session.folder_id = stored.folder_id;
}

async fn complete_auth(
    app: &tauri::AppHandle,
    listener: TcpListener,
    csrf: String,
    verifier: String,
    redirect: String,
    creds: Credentials,
) -> Result<(), String> {
    let code = tauri::async_runtime::spawn_blocking(move || {
        let deadline = Instant::now() + Duration::from_secs(AUTH_TIMEOUT_SECS);
        await_code(listener, &csrf, deadline)
    })
    .await
    .map_err(|e| e.to_string())??;

    let tokens = exchange_code(&creds, &code, &redirect, &verifier).await?;
    let email = fetch_email(&tokens.access_token).await?;
    let folder_id = ensure_folder(&tokens.access_token).await?;

    {
        let state = app.state::<GDriveState>();
        let mut session = state.0.lock().unwrap();
        if let Some(refresh) = &tokens.refresh_token {
            session.refresh_token = Some(refresh.clone());
        }
        session.access_token = Some(tokens.access_token.clone());
        session.access_expiry = now() + tokens.expires_in.saturating_sub(60);
        session.email = Some(email.clone());
        session.folder_id = Some(folder_id.clone());
    }

    let mut stored = load_state(app);
    if let Some(refresh) = &tokens.refresh_token {
        stored.refresh_token = Some(refresh.clone());
    }
    stored.email = Some(email);
    stored.folder_id = Some(folder_id);
    save_state(app, &stored)?;
    Ok(())
}

#[tauri::command]
pub async fn gdrive_connect(app: tauri::AppHandle) -> Result<String, String> {
    let creds = load_credentials()?;
    let verifier = random_b64(64);
    let challenge = pkce_challenge(&verifier);
    let csrf = random_b64(24);

    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let redirect = format!("http://127.0.0.1:{port}");

    let auth_url = format!(
        "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&code_challenge={}&code_challenge_method=S256&state={}&access_type=offline&prompt=consent",
        creds.auth_uri,
        urlencode(&creds.client_id),
        urlencode(&redirect),
        urlencode(SCOPES),
        challenge,
        urlencode(&csrf),
    );

    use tauri_plugin_opener::OpenerExt;
    let _ = app.opener().open_url(auth_url.clone(), None::<&str>);

    let app_bg = app.clone();
    tauri::async_runtime::spawn(async move {
        let event = match complete_auth(&app_bg, listener, csrf, verifier, redirect, creds).await {
            Ok(()) => AuthEvent { ok: true, error: None },
            Err(error) => AuthEvent { ok: false, error: Some(error) },
        };
        let _ = app_bg.emit("gdrive-auth", event);
    });

    Ok(auth_url)
}

#[tauri::command]
pub async fn gdrive_disconnect(app: tauri::AppHandle, state: tauri::State<'_, GDriveState>) -> Result<Status, String> {
    let refresh = {
        let session = state.0.lock().unwrap();
        session.refresh_token.clone()
    };
    if let Some(refresh) = refresh {
        let _ = HTTP
            .post("https://oauth2.googleapis.com/revoke")
            .form(&[("token", refresh.as_str())])
            .send()
            .await;
    }
    {
        let mut session = state.0.lock().unwrap();
        session.refresh_token = None;
        session.access_token = None;
        session.access_expiry = 0;
        session.email = None;
    }
    let mut stored = load_state(&app);
    stored.refresh_token = None;
    stored.email = None;
    save_state(&app, &stored)?;
    Ok(status_inner(&app, &state))
}

#[tauri::command]
pub async fn gdrive_status(app: tauri::AppHandle, state: tauri::State<'_, GDriveState>) -> Result<Status, String> {
    Ok(status_inner(&app, &state))
}

#[tauri::command]
pub async fn gdrive_backup(app: tauri::AppHandle, state: tauri::State<'_, GDriveState>) -> Result<BackupOutcome, String> {
    let access_token = valid_access_token(&app, &state).await?;
    let folder_id = ensure_folder_id(&app, &state, &access_token).await?;
    let files = collect_local_files(&app)?;
    let mut stored = load_state(&app);
    let mut uploaded = 0;
    for (name, bytes) in &files {
        let hash = hash_bytes(bytes);
        let existing = stored.files.get(name).cloned();
        if let Some(record) = &existing {
            if record.hash == hash {
                continue;
            }
        }
        let drive_id = match &existing {
            Some(record) => Some(record.drive_id.clone()),
            None => find_file(&access_token, &folder_id, name).await?.map(|file| file.id),
        };
        let result = upload_file(&access_token, &folder_id, name, bytes, drive_id).await?;
        stored.files.insert(name.clone(), FileRecord { hash, drive_id: result.id });
        uploaded += 1;
    }
    if uploaded > 0 {
        stored.last_backup = Some(now());
        save_state(&app, &stored)?;
    }
    Ok(BackupOutcome {
        uploaded,
        status: Status {
            connected: true,
            email: stored.email.clone(),
            last_backup: stored.last_backup,
            pending: false,
        },
    })
}

#[tauri::command]
pub async fn gdrive_restore(app: tauri::AppHandle, state: tauri::State<'_, GDriveState>) -> Result<RestoreResult, String> {
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
            FileRecord { hash: hash_bytes(&bytes), drive_id: file.id.clone() },
        );
        restored += 1;
    }
    stored.last_backup = Some(now());
    save_state(&app, &stored)?;
    Ok(RestoreResult { restored })
}

#[tauri::command]
pub async fn gdrive_list_backups(app: tauri::AppHandle, state: tauri::State<'_, GDriveState>) -> Result<Vec<RemoteBackup>, String> {
    let access_token = valid_access_token(&app, &state).await?;
    let folder_id = ensure_folder_id(&app, &state, &access_token).await?;
    let remote = list_in_folder(&access_token, &folder_id).await?;
    Ok(remote
        .into_iter()
        .filter(|file| safe_book_name(&file.name))
        .map(|file| RemoteBackup { name: file.name })
        .collect())
}
