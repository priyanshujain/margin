// Calendar's Google sign-in. The protocol (PKCE, the loopback listener, the deep link, token
// requests, single-flight refresh) is margin-oauth with Google as margin-google's configuration.
// What is here is Calendar's own: the scope, the accounts table and the `auth` event.

use std::time::{Duration, Instant};

use margin_oauth::{CANCELLED, Pages, Sessions};
use tauri::{Emitter, Manager};
#[cfg(mobile)]
use tokio::sync::Mutex;

use crate::dto::{Account, AuthEvent};
use crate::secrets;

pub const SCOPES: &str = "openid email https://www.googleapis.com/auth/calendar";

const CREDENTIALS_JSON: &str = include_str!(concat!(env!("OUT_DIR"), "/google-credentials.json"));

#[cfg(mobile)]
const ANDROID_REDIRECT: &str = "studio.margin.calendar:/oauth2redirect";

const NOT_SET_UP: &str = "Google Calendar is not set up yet. Add a real OAuth client to google-credentials.json and rebuild.";

const PAGES: Pages<'static> = Pages {
    app: "Margin Calendar",
    provider: "Google",
};

/// The one client this build signs in with. A refresh token belongs to the client that got it.
pub fn load_credentials() -> Result<margin_oauth::Client, String> {
    margin_google::client(CREDENTIALS_JSON, NOT_SET_UP)
}

#[derive(Default)]
pub struct AuthState {
    pub sessions: Sessions,
    /// The sign-in in flight, which closing the add-account sheet cancels.
    pub attempts: margin_oauth::Attempts,
    #[cfg(mobile)]
    pub pending: Mutex<Option<margin_oauth::Pending>>,
}

fn with_conn<T>(
    app: &tauri::AppHandle,
    f: impl FnOnce(&rusqlite::Connection) -> Result<T, String>,
) -> Result<T, String> {
    let store = app
        .try_state::<crate::store::Store>()
        .ok_or("the local store is not open")?;
    let conn = store.conn.lock().map_err(|e| e.to_string())?;
    f(&conn)
}

pub async fn list_accounts(
    app: &tauri::AppHandle,
    _state: &tauri::State<'_, AuthState>,
) -> Result<Vec<Account>, String> {
    with_conn(app, crate::store::read::accounts)
}

fn emit_auth(app: &tauri::AppHandle, outcome: Result<(String, String), String>) {
    let event = match outcome {
        Ok((account_id, email)) => {
            crate::emit_store_changed(app, "account-connected");
            AuthEvent {
                ok: true,
                error: None,
                account_id: Some(account_id),
                email: Some(email),
                cancelled: false,
            }
        }
        Err(error) => AuthEvent {
            ok: false,
            cancelled: error == CANCELLED,
            error: Some(error),
            account_id: None,
            email: None,
        },
    };
    let _ = app.emit("auth", event);
}

/// Returns the consent URL; the answer arrives later as the `auth` event. The loopback flow runs
/// everywhere unless the credentials carry a client for this phone, which takes the deep link.
pub async fn connect(app: tauri::AppHandle) -> Result<String, String> {
    let client = load_credentials()?;
    let attempt = app.state::<AuthState>().attempts.begin();
    #[cfg(mobile)]
    if client.platform_client {
        return connect_by_deep_link(app, client, attempt).await;
    }
    connect_by_loopback(app, client, attempt).await
}

/// Ends the sign-in in flight: the loopback listener stops, a deep link waiting for its answer is
/// forgotten, and a code that already arrived is never exchanged into an account.
pub async fn cancel(app: &tauri::AppHandle) {
    let state = app.state::<AuthState>();
    state.attempts.cancel();
    #[cfg(mobile)]
    {
        state.pending.lock().await.take();
    }
}

async fn connect_by_loopback(
    app: tauri::AppHandle,
    client: margin_oauth::Client,
    attempt: margin_oauth::Attempt,
) -> Result<String, String> {
    let verifier = margin_oauth::random_b64(64);
    let challenge = margin_oauth::pkce_challenge(&verifier);
    let csrf = margin_oauth::random_b64(24);

    let (listener, redirect) = margin_oauth::bind_loopback()?;
    let url = margin_oauth::auth_url(&client, &redirect, &challenge, &csrf, SCOPES, None);

    #[cfg(desktop)]
    margin_oauth::open_in_browser(&app, &url);
    #[cfg(mobile)]
    margin_oauth::browser::open(&app, &url);

    let app_bg = app.clone();
    tauri::async_runtime::spawn(async move {
        let outcome =
            complete_auth(&app_bg, listener, csrf, verifier, redirect, client, &attempt).await;
        emit_auth(&app_bg, outcome);
    });

    Ok(url)
}

#[cfg(mobile)]
async fn connect_by_deep_link(
    app: tauri::AppHandle,
    client: margin_oauth::Client,
    _attempt: margin_oauth::Attempt,
) -> Result<String, String> {
    let verifier = margin_oauth::random_b64(64);
    let challenge = margin_oauth::pkce_challenge(&verifier);
    let csrf = margin_oauth::random_b64(24);
    let redirect = margin_google::redirect_uri(&client, ANDROID_REDIRECT);
    let url = margin_oauth::auth_url(&client, &redirect, &challenge, &csrf, SCOPES, None);

    {
        let state = app.state::<AuthState>();
        let mut pending = state.pending.lock().await;
        *pending = Some(margin_oauth::Pending {
            state: csrf,
            verifier,
            redirect: redirect.clone(),
            expires: margin_oauth::now() + margin_oauth::PENDING_TIMEOUT_SECS,
        });
    }

    #[cfg(target_os = "ios")]
    margin_oauth::browser::authenticate(
        &app,
        &url,
        margin_oauth::callback_scheme(&redirect),
        browser_finished,
    );
    #[cfg(not(target_os = "ios"))]
    margin_oauth::open_in_browser(&app, &url);
    Ok(url)
}

#[cfg(target_os = "ios")]
fn browser_finished(app: tauri::AppHandle, finished: margin_oauth::browser::Finished) {
    tauri::async_runtime::spawn(async move {
        match finished {
            margin_oauth::browser::Finished::Redirect(url) => handle_redirect(app, &url).await,
            margin_oauth::browser::Finished::Abandoned(reason) => abandon_pending(app, reason).await,
        }
    });
}

/// The consent surface closed with no callback. A cancel says so rather than reporting a failure.
#[cfg(mobile)]
pub async fn abandon_pending(app: tauri::AppHandle, reason: Option<String>) {
    let auth = app.state::<AuthState>();
    let pending = {
        let mut slot = auth.pending.lock().await;
        slot.take()
    };
    if pending.is_some() {
        emit_auth(&app, Err(reason.unwrap_or_else(|| CANCELLED.to_string())));
    }
}

/// Every URL the OS hands the app on its scheme. One with no `state` we are waiting for is ignored.
#[cfg(mobile)]
pub async fn handle_redirect(app: tauri::AppHandle, incoming: &url::Url) {
    let callback = margin_oauth::parse_callback(incoming);
    let Some(state) = callback.state.clone() else {
        return;
    };

    let auth = app.state::<AuthState>();
    let pending = {
        let mut slot = auth.pending.lock().await;
        margin_oauth::take_pending(&mut slot, &state)
    };
    let Some(pending) = pending else {
        return;
    };

    let outcome = match load_credentials().and_then(|client| {
        margin_oauth::resolve_callback(&pending, callback, &client.provider).map(|code| (client, code))
    }) {
        Ok((client, code)) => {
            finish(&app, &client, &code, &pending.redirect, &pending.verifier, || false).await
        }
        Err(e) => Err(e),
    };
    emit_auth(&app, outcome);
}

async fn complete_auth(
    app: &tauri::AppHandle,
    listener: std::net::TcpListener,
    csrf: String,
    verifier: String,
    redirect: String,
    client: margin_oauth::Client,
    attempt: &margin_oauth::Attempt,
) -> Result<(String, String), String> {
    let waiting = attempt.clone();
    let code = tauri::async_runtime::spawn_blocking(move || {
        let deadline = Instant::now() + Duration::from_secs(margin_oauth::AUTH_TIMEOUT_SECS);
        margin_oauth::await_code(listener, &csrf, deadline, PAGES, || {
            waiting.is_cancelled() || consent_closed()
        })
    })
    .await
    .map_err(|e| e.to_string())?;

    #[cfg(mobile)]
    margin_oauth::browser::close(app);

    let code = code?;
    if attempt.is_cancelled() {
        return Err(CANCELLED.to_string());
    }
    finish(app, &client, &code, &redirect, &verifier, || attempt.is_cancelled()).await
}

#[cfg(desktop)]
fn consent_closed() -> bool {
    false
}

#[cfg(mobile)]
fn consent_closed() -> bool {
    margin_oauth::browser::cancelled()
}

async fn finish(
    app: &tauri::AppHandle,
    client: &margin_oauth::Client,
    code: &str,
    redirect: &str,
    verifier: &str,
    cancelled: impl Fn() -> bool,
) -> Result<(String, String), String> {
    let tokens = margin_oauth::exchange_code(client, code, redirect, verifier).await?;
    // The exchange takes a round trip, and the sheet may have been closed during it.
    if cancelled() {
        return Err(CANCELLED.to_string());
    }
    let refresh = tokens
        .refresh_token
        .clone()
        .ok_or("Google did not return a refresh token. Remove Margin Calendar from your Google account permissions and connect again.")?;

    let claims = tokens
        .id_token
        .as_deref()
        .and_then(margin_oauth::id_token_claims)
        .unwrap_or_default();
    let email = match claims.email {
        Some(email) if !email.is_empty() => email,
        _ => margin_oauth::fetch_email(client, &tokens.access_token).await?,
    };
    // The Google user id keys everything. The email stands in only when the id_token is missing.
    let account_id = claims.sub.unwrap_or_else(|| email.clone());
    if account_id.is_empty() {
        return Err("Google returned neither a user id nor an email address.".to_string());
    }

    secrets::store(&account_id, &refresh)?;
    let reference = secrets::reference(&account_id);
    with_conn(app, |conn| {
        crate::store::write::upsert_account(conn, &account_id, "google", &email, Some(&reference))
    })?;

    let state = app.state::<AuthState>();
    let mut sessions = state.sessions.lock().await;
    sessions.insert(
        account_id.clone(),
        margin_oauth::Session {
            access_token: Some(tokens.access_token),
            access_expiry: margin_oauth::expiry_at(tokens.expires_in),
            email: Some(email.clone()),
        },
    );

    Ok((account_id, email))
}

/// Revoke, forget the token, then wipe the account's rows. The account leaves this device whether
/// or not Google answered the revoke.
pub async fn disconnect(
    app: &tauri::AppHandle,
    state: &tauri::State<'_, AuthState>,
    account_id: &str,
) -> Result<(), String> {
    if let (Ok(Some(refresh)), Ok(client)) = (secrets::load(account_id), load_credentials()) {
        let _ = margin_oauth::revoke(&client, &refresh).await;
    }
    {
        let mut sessions = state.sessions.lock().await;
        sessions.remove(account_id);
    }
    let removed = secrets::delete(account_id);
    with_conn(app, |conn| crate::store::write::wipe_account(conn, account_id))?;
    crate::emit_store_changed(app, "disconnect");
    removed
}

pub async fn valid_access_token(
    _app: &tauri::AppHandle,
    state: &AuthState,
    account_id: &str,
) -> Result<String, String> {
    state
        .sessions
        .access_token(account_id, || async {
            let refresh = secrets::load(account_id)?
                .ok_or_else(|| format!("Account {account_id} is not connected to Google."))?;
            let client = load_credentials()?;
            let tokens = margin_oauth::refresh_access_token(&client, &refresh).await?;
            if let Some(rotated) = &tokens.refresh_token
                && rotated != &refresh {
                    secrets::store(account_id, rotated)?;
                }
            Ok(tokens)
        })
        .await
}

/// The token store needs a directory and the sessions map wants the stored emails, both of which
/// are only knowable once the app is up.
pub fn init_sessions(app: &tauri::AppHandle) {
    if let Ok(dir) = margin_shell::dirs::app_data_dir(app) {
        secrets::init(dir);
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let accounts = match with_conn(&app, crate::store::read::accounts) {
            Ok(accounts) => accounts,
            Err(_) => return,
        };
        let state = app.state::<AuthState>();
        let mut sessions = state.sessions.lock().await;
        for account in accounts {
            sessions.entry(account.id).or_default().email = Some(account.email);
        }
    });
}
