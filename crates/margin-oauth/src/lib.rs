mod flow;
mod loopback;
mod session;
mod token;

#[cfg(all(feature = "tauri", any(target_os = "ios", target_os = "android")))]
pub mod browser;

pub use flow::{
    Callback, Pending, auth_url, callback_scheme, granted_scopes, parse_callback, pkce_challenge,
    random_b64, resolve_callback, scope_string, take_pending,
};
pub use loopback::{Pages, Redirect, await_code, bind_loopback, parse_redirect, request_path};
pub use session::{EXPIRY_SKEW_SECS, Session, Sessions, expiry_at, now};
pub use token::{
    IdClaims, TokenResponse, exchange_code, fetch_email, http, id_token_claims, read_json,
    refresh_access_token, revoke,
};

/// One OAuth client of one provider. Providers differ only in what is filled in here.
#[derive(Debug, Clone)]
pub struct Client {
    /// The provider's name as people see it, in messages such as "Google token refresh".
    pub provider: String,
    pub client_id: String,
    /// Absent for public clients. Then PKCE is the only protection for an intercepted code.
    pub client_secret: Option<String>,
    pub auth_uri: String,
    pub token_uri: String,
    pub revoke_uri: String,
    pub userinfo_uri: String,
    /// Provider specific consent URL parameters, such as Google's `access_type=offline`.
    pub auth_params: Vec<(String, String)>,
    /// The custom scheme redirect for the deep link flow, when the provider's console gives one.
    pub redirect_uri: Option<String>,
    /// True when this client came out of a platform block, which selects the deep link flow.
    pub platform_client: bool,
}

/// Returned by every path that means the user chose not to sign in, so callers can compare against
/// it and report a change of mind as a cancel rather than a failure.
pub const CANCELLED: &str = "Sign-in was cancelled.";

/// How long the loopback listener waits. A phone has to cover a password typed on glass and often
/// a 2FA prompt in another app.
#[cfg(not(any(target_os = "ios", target_os = "android")))]
pub const AUTH_TIMEOUT_SECS: u64 = 120;
#[cfg(any(target_os = "ios", target_os = "android"))]
pub const AUTH_TIMEOUT_SECS: u64 = 900;

/// How long a deep link flow's verifier waits for the browser to come back.
pub const PENDING_TIMEOUT_SECS: u64 = 900;

/// Hands the URL to the user's own browser, in its own process. Never an in-app webview.
#[cfg(feature = "tauri")]
pub fn open_in_browser(app: &tauri::AppHandle, url: &str) {
    use tauri_plugin_opener::OpenerExt;
    let _ = app.opener().open_url(url.to_string(), None::<&str>);
}
