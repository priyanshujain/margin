// Mail's Google sign-in. The protocol (PKCE, the loopback listener, the deep link, token requests,
// single-flight refresh) is margin-oauth with Google as margin-google's configuration. What is
// here is Mail's own: six scopes and the one it cannot work without, the account registry, and
// the `auth` event the connect panel waits on.

use std::time::{Duration, Instant};

use margin_oauth::{IdClaims, Pages, Sessions};
use tauri::{Emitter, Manager};
#[cfg(mobile)]
use tokio::sync::Mutex;

use crate::accounts;
use crate::dto::{AccountKind, AuthEvent};
use crate::secrets;
use margin_oauth::CANCELLED;

/// What every connect asks for, in the order the consent screen lists them. The mirror of `SCOPES`
/// in `src/ipc.ts`, and the two files have to agree: the frontend renders the consent copy from
/// its copy of the list.
pub const BASE_SCOPES: [&str; 6] = [
    "openid",
    "email",
    "https://www.googleapis.com/auth/gmail.modify",
    "https://www.googleapis.com/auth/gmail.settings.basic",
    "https://www.googleapis.com/auth/contacts.readonly",
    "https://www.googleapis.com/auth/contacts.other.readonly",
];

/// Without this there is no app. Reading mail, marking it read and moving it are the whole of what
/// Margin Mail does, so an account that withheld it is refused rather than added in a state where
/// every screen is an error.
pub const REQUIRED_SCOPE: &str = "https://www.googleapis.com/auth/gmail.modify";

const CREDENTIALS_JSON: &str = include_str!(concat!(env!("OUT_DIR"), "/google-credentials.json"));

#[cfg(mobile)]
const ANDROID_REDIRECT: &str = "studio.margin.mail:/oauth2redirect";

const NOT_SET_UP: &str =
    "Margin Mail is not set up yet. Add a real OAuth client to google-credentials.json and rebuild.";

const PAGES: Pages<'static> = Pages {
    app: "Margin Mail",
    provider: "Google",
};

/// The one client this build signs in with. Every caller has to agree on the answer, because a
/// refresh token belongs to the client that obtained it.
pub fn load_credentials() -> Result<margin_oauth::Client, String> {
    margin_google::client(CREDENTIALS_JSON, NOT_SET_UP)
}

/// One session per connected account, and on mobile the deep link round trip in flight.
#[derive(Default)]
pub struct AuthState {
    pub sessions: Sessions,
    /// The sign-in in flight, which closing the add-account sheet cancels.
    pub attempts: margin_oauth::Attempts,
    #[cfg(mobile)]
    pub pending: Mutex<Option<margin_oauth::Pending>>,
}

fn scope_string(extra: &[String]) -> String {
    margin_oauth::scope_string(&BASE_SCOPES, extra)
}

fn missing_required(granted: &[String]) -> Vec<String> {
    if granted.iter().any(|scope| scope == REQUIRED_SCOPE) {
        return Vec::new();
    }
    vec![REQUIRED_SCOPE.to_string()]
}

/// The name an account starts life with. `profile` is not in the scope list, so Google tells us
/// nothing but the address and the local part is the best guess available. It is a starting value,
/// not a fact: `account_set_name` exists because this will sometimes be wrong.
fn display_name(claims: &IdClaims, email: &str) -> String {
    match claims.name.as_deref() {
        Some(name) if !name.trim().is_empty() => name.trim().to_string(),
        _ => email.split('@').next().unwrap_or(email).to_string(),
    }
}

/// One finished consent. `missing_required` non-empty means nothing was stored: the token was
/// revoked again and there is no account, because the app cannot work without that scope.
struct Granted {
    account_id: String,
    email: String,
    scopes: Vec<String>,
    missing_required: Vec<String>,
}

fn emit_auth(app: &tauri::AppHandle, outcome: Result<Granted, String>) {
    let event = match outcome {
        Ok(granted) if granted.missing_required.is_empty() => {
            // The account is written and nothing more. The engine is handed it by `account_start`,
            // once the window it is to hold has been chosen on the panel that waits for it; the
            // registry alone is what a launch attaches, with the default window, if that never
            // happens. It used to be attached nowhere at all after a connect from Settings, and
            // sat on "Nothing here" until the next launch.
            crate::emit_store_changed(app, "accounts");
            AuthEvent {
                ok: true,
                error: None,
                account_id: Some(granted.account_id),
                email: Some(granted.email),
                cancelled: false,
                granted_scopes: granted.scopes,
                missing_required: Vec::new(),
            }
        }
        // No account id, because there is no account: the frontend has enough here to name the
        // address, name the permission and offer to try again.
        Ok(refused) => AuthEvent {
            ok: false,
            error: Some(format!(
                "{} was not added: Margin Mail cannot do anything without permission to read and change your mail.",
                refused.email
            )),
            account_id: None,
            email: Some(refused.email),
            cancelled: false,
            granted_scopes: refused.scopes,
            missing_required: refused.missing_required,
        },
        // Compared against the constant the cancel paths raise, rather than matched on its text,
        // so rewording it cannot quietly turn a cancel back into an error on screen.
        Err(error) => AuthEvent {
            ok: false,
            cancelled: error == CANCELLED,
            error: Some(error),
            account_id: None,
            email: None,
            granted_scopes: Vec::new(),
            missing_required: Vec::new(),
        },
    };
    let _ = app.emit("auth", event);
}

/// A new account. Returns the consent URL; the answer arrives later as the `auth` event.
///
/// The hint is the address typed on the connect screen, so the consent page opens on that account
/// rather than on the chooser. Anything that is not an address is treated as no hint at all.
pub async fn connect(
    app: tauri::AppHandle,
    extra_scopes: Vec<String>,
    hint: Option<String>,
) -> Result<String, String> {
    let hint = hint
        .map(|typed| typed.trim().to_string())
        .filter(|typed| typed.contains('@'));
    start(app, extra_scopes, hint).await
}

/// The same consent again for an account that is already connected, to pick up a scope that was
/// withheld or never asked for.
///
/// Before making this cleverer: there is no way to add `calendar.events` to a live token. Google
/// does not offer incremental authorization to installed apps, so the only thing that can be done
/// is the whole flow again with the base list plus the extras, replacing the stored token. That is
/// why answering one invite costs a trip through the consent screen, and why the button that leads
/// here says so first.
pub async fn grant(
    app: tauri::AppHandle,
    account_id: String,
    extra_scopes: Vec<String>,
) -> Result<String, String> {
    let entry = accounts::find(&app, &account_id)?
        .ok_or_else(|| format!("Account {account_id} is not connected."))?;
    start(app, extra_scopes, Some(entry.email)).await
}

/// One consent request, on all five platforms.
///
/// The loopback flow is the default everywhere, phones included. Google lets a Desktop client
/// redirect to loopback on any port with nothing registered in advance, and the token endpoint
/// checks the client id, the secret and the redirect and has no idea which OS asked. What used to
/// make this impossible on a phone was not the protocol but the browser: sending the user out to
/// Safari suspends this process, and a suspended process is not accepting on its socket. An in-app
/// browser does not leave, which is why `browser.rs` exists and why this branch is now the common
/// one.
///
/// The custom scheme flow runs instead when the credentials file has a client for this platform.
/// That is Google's stated guidance, and the thing to reach for if they ever start enforcing it,
/// but on iOS it is worth having for its own sake: it is the only way to reach
/// `ASWebAuthenticationSession`, which is the only iOS browser that shares Safari's cookies.
async fn start(
    app: tauri::AppHandle,
    extra_scopes: Vec<String>,
    hint: Option<String>,
) -> Result<String, String> {
    let client = load_credentials()?;
    let scopes = scope_string(&extra_scopes);
    let attempt = app.state::<AuthState>().attempts.begin();
    #[cfg(mobile)]
    if client.platform_client {
        return connect_by_deep_link(app, client, scopes, hint).await;
    }
    connect_by_loopback(app, client, scopes, hint, attempt).await
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
    scopes: String,
    hint: Option<String>,
    attempt: margin_oauth::Attempt,
) -> Result<String, String> {
    let verifier = margin_oauth::random_b64(64);
    let challenge = margin_oauth::pkce_challenge(&verifier);
    let csrf = margin_oauth::random_b64(24);

    let (listener, redirect) = margin_oauth::bind_loopback()?;
    let url = margin_oauth::auth_url(&client, &redirect, &challenge, &csrf, &scopes, hint.as_deref());

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

/// The custom scheme flow. What is stashed here is the PKCE verifier and the CSRF state, which is
/// the only thing tying the code that comes back to the request that went out, since there is no
/// listener holding either on its own stack.
///
/// Where the answer comes back from differs by platform, and so does what it is worth.
///
/// iOS hands the URL to `ASWebAuthenticationSession`, which reports the callback straight to a
/// completion handler. It is the only iOS surface that shares Safari's cookies, so an account
/// already signed in on the phone is offered by name rather than asking for a password again. That
/// is the reason this flow is worth the console visit on iOS, and `browser.rs` says the rest.
///
/// Android opens the external browser and waits for the deep link, which is a genuinely separate
/// app: this process may be backgrounded, or killed outright, for the whole of it. Android needs
/// none of this, because a Custom Tab already shares Chrome's cookies and the loopback flow above
/// already works, so this is only ever reached when somebody has gone and made an Android client.
#[cfg(mobile)]
async fn connect_by_deep_link(
    app: tauri::AppHandle,
    client: margin_oauth::Client,
    scopes: String,
    hint: Option<String>,
) -> Result<String, String> {
    let verifier = margin_oauth::random_b64(64);
    let challenge = margin_oauth::pkce_challenge(&verifier);
    let csrf = margin_oauth::random_b64(24);
    let redirect = margin_google::redirect_uri(&client, ANDROID_REDIRECT);
    let url = margin_oauth::auth_url(&client, &redirect, &challenge, &csrf, &scopes, hint.as_deref());

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

/// The consent surface ended with no callback URL at all: the user closed it, or it could not be
/// shown. Either way the pending verifier is spent, and the panel is waiting on an answer that is
/// never coming, so it is told. A cancel says so plainly rather than reporting a failure.
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

/// Every URL the OS hands the app on a registered scheme, including ones that have nothing to do
/// with consent. A link that carries no `state` we are waiting for is ignored in silence rather
/// than reported, because another feature may want that scheme later and a stray link is not an
/// authorization failure worth putting on screen.
#[cfg(mobile)]
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
) -> Result<Granted, String> {
    let waiting = attempt.clone();
    let code = tauri::async_runtime::spawn_blocking(move || {
        let deadline = Instant::now() + Duration::from_secs(margin_oauth::AUTH_TIMEOUT_SECS);
        margin_oauth::await_code(listener, &csrf, deadline, PAGES, || {
            waiting.is_cancelled() || consent_closed()
        })
    })
    .await
    .map_err(|e| e.to_string())?;

    // Nothing takes the consent page down on its own once the listener has its answer, and what it
    // is showing by then is the listener's own "you can close this" page. Taken away here rather
    // than after the exchange, and whatever the outcome was, so the app comes back the moment the
    // browser has nothing left to do. Desktop has no such surface and compiles this out.
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

/// Code to stored account. Everything past the point where the two flows stop differing.
async fn finish(
    app: &tauri::AppHandle,
    client: &margin_oauth::Client,
    code: &str,
    redirect: &str,
    verifier: &str,
    cancelled: impl Fn() -> bool,
) -> Result<Granted, String> {
    let tokens = margin_oauth::exchange_code(client, code, redirect, verifier).await?;
    // The exchange takes a round trip, and the sheet may have been closed during it.
    if cancelled() {
        return Err(CANCELLED.to_string());
    }
    let refresh = tokens
        .refresh_token
        .clone()
        .ok_or("Google did not return a refresh token. Remove Margin from third party access on your Google account, which signs out every Margin app, and connect again.")?;

    let claims = tokens
        .id_token
        .as_deref()
        .and_then(margin_oauth::id_token_claims)
        .unwrap_or_default();
    let email = match claims.email.clone() {
        Some(email) if !email.is_empty() => email,
        _ => margin_oauth::fetch_email(client, &tokens.access_token).await?,
    };
    // The Google user id keys everything. It falls back to the email only when the id_token is
    // missing, which should not happen while `openid` is in the scope set.
    let account_id = claims.sub.clone().unwrap_or_else(|| email.clone());
    if account_id.is_empty() {
        return Err("Google returned neither a user id nor an email address.".to_string());
    }

    let scopes = margin_oauth::granted_scopes(tokens.scope.as_deref());
    let missing = missing_required(&scopes);
    if !missing.is_empty() {
        // The token is dropped without being stored, and deliberately without being revoked. One
        // OAuth client covers every Margin app and revocation acts on the authorization rather
        // than on the one string, so tidying up here could take the sibling apps down with it, or
        // this account's own working token when the refusal came from `grant` rather than
        // `connect`. A refresh token nobody kept is the cheaper thing to leave behind.
        return Ok(Granted {
            account_id,
            email,
            scopes,
            missing_required: missing,
        });
    }

    secrets::store(&account_id, &refresh)?;
    accounts::upsert(
        app,
        &account_id,
        &email,
        &display_name(&claims, &email),
        scopes.clone(),
    )?;

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

    Ok(Granted {
        account_id,
        email,
        scopes,
        missing_required: Vec::new(),
    })
}

/// Revoke, forget, then let go of the databases: the one way an account leaves this device.
///
/// The grant goes back to Google first, while the token is still here to name it. It is the whole
/// grant, for every Margin app on every machine, because the suite shares one OAuth client and the
/// endpoint acts on the authorization behind a token rather than on the string it is handed; the
/// settings screen says so before it offers the button. An IMAP account has nothing at Google to
/// hand back, and its passwords are simply forgotten.
///
/// The order after that is the point. The registry entry goes before anything else touches disk,
/// so a crash halfway through leaves a database with no token, which `Db::on_disk` can find and
/// clear up, rather than a live token for an account the user has told us to forget.
///
/// The account goes from this device whether or not Google answered: the person asked for it to
/// go, and keeping a token here because Google was unreachable would be the opposite of what they
/// pressed. What is reported is whether Google heard about it, which is the half the button
/// promised and the half they cannot see for themselves.
///
/// `keep_data` leaves the mirror and the state database on disk, set aside where nothing lists
/// them, so the decisions come back if the account is ever added again.
pub async fn remove(
    app: &tauri::AppHandle,
    state: &tauri::State<'_, AuthState>,
    account_id: &str,
    keep_data: bool,
) -> Result<(), String> {
    let kind = accounts::find(app, account_id)?.map(|entry| entry.kind);
    let revoked = match kind {
        Some(AccountKind::Imap) => Ok(()),
        _ => match secrets::load(account_id) {
            Ok(Some(refresh)) => match load_credentials() {
                Ok(client) => margin_oauth::revoke(&client, &refresh).await,
                Err(e) => Err(e),
            },
            Ok(None) => Ok(()),
            Err(e) => Err(e),
        },
    };

    {
        let mut sessions = state.sessions.lock().await;
        sessions.remove(account_id);
    }
    crate::sync::detach(account_id);
    accounts::remove(app, account_id)?;
    // Both kinds of secret whatever kind the account was: a registry entry that had already gone
    // says nothing about which it held, and deleting a key that is not there is nothing.
    let forgotten =
        secrets::delete(account_id).and_then(|_| crate::imap::delete_passwords(account_id));

    // The mirror is derived and the state database is not, but both are this account's and the
    // person asked for the account to go. `Db` is absent in a test and in any build that has not
    // opened one yet, which is the only case where there is nothing on disk to remove.
    if let Some(db) = app.try_state::<crate::db::Db>() {
        db.close(account_id)?;
        if keep_data {
            db.set_aside(account_id)?;
        } else {
            let dir = db.account_dir(account_id);
            if dir.exists() {
                std::fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
            }
        }
    }
    crate::emit_store_changed(app, "accounts");
    forgotten?;
    revoked.map_err(|e| {
        format!(
            "The account was removed from this device, but Google could not be reached to revoke Margin's access, so the grant is still standing: {e}"
        )
    })
}

/// Single-flight: the map lock is held across the refresh await, so concurrent callers queue on
/// one token request rather than each firing their own. That serializes refreshes across accounts
/// as well, which is the right trade for something that happens once an hour per account.
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
                && rotated != &refresh
            {
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
        let accounts = match accounts::list(&app) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extra_scopes_are_asked_for_alongside_the_base_list_and_never_alone() {
        let scopes = scope_string(&["https://www.googleapis.com/auth/calendar.events".to_string()]);
        let asked: Vec<&str> = scopes.split(' ').collect();
        assert_eq!(&asked[..BASE_SCOPES.len()], &BASE_SCOPES[..]);
        assert_eq!(
            asked.last(),
            Some(&"https://www.googleapis.com/auth/calendar.events")
        );
    }

    #[test]
    fn an_extra_that_is_already_asked_for_is_not_asked_for_twice() {
        let scopes = scope_string(&[REQUIRED_SCOPE.to_string(), "  ".to_string()]);
        assert_eq!(scopes, BASE_SCOPES.join(" "));
    }

    #[test]
    fn an_account_without_the_gmail_scope_is_refused_and_says_which_one() {
        let granted = margin_oauth::granted_scopes(Some(
            "openid https://www.googleapis.com/auth/userinfo.email https://www.googleapis.com/auth/contacts.readonly",
        ));
        assert_eq!(missing_required(&granted), vec![REQUIRED_SCOPE.to_string()]);
        assert!(missing_required(&margin_oauth::granted_scopes(None)).len() == 1);
    }

    #[test]
    fn the_gmail_scope_is_the_only_one_an_account_is_refused_for() {
        let granted = margin_oauth::granted_scopes(Some(REQUIRED_SCOPE));
        assert!(missing_required(&granted).is_empty());
    }

    #[test]
    fn a_name_falls_back_to_the_address_because_profile_is_not_asked_for() {
        let claims = IdClaims::default();
        assert_eq!(display_name(&claims, "sam.reed@example.test"), "sam.reed");
        let named = IdClaims {
            name: Some("Sam Reed".to_string()),
            ..IdClaims::default()
        };
        assert_eq!(display_name(&named, "sam.reed@example.test"), "Sam Reed");
    }
}
