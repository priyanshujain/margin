use serde::Deserialize;

pub const AUTH_URI: &str = "https://accounts.google.com/o/oauth2/auth";
pub const TOKEN_URI: &str = "https://oauth2.googleapis.com/token";
pub const REVOKE_URI: &str = "https://oauth2.googleapis.com/revoke";
pub const USERINFO_URI: &str = "https://www.googleapis.com/oauth2/v2/userinfo";

/// Google's `google-credentials.json`: a Desktop client under `installed`, and optional public
/// clients under `android` and `ios` that redirect to a custom scheme instead of loopback.
#[derive(Deserialize)]
struct CredentialsFile {
    installed: Credentials,
    #[serde(default)]
    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    android: Option<Credentials>,
    #[serde(default)]
    #[cfg_attr(not(target_os = "ios"), allow(dead_code))]
    ios: Option<Credentials>,
}

#[derive(Deserialize)]
struct Credentials {
    client_id: String,
    #[serde(default)]
    client_secret: Option<String>,
    #[serde(default = "default_auth_uri")]
    auth_uri: String,
    #[serde(default = "default_token_uri")]
    token_uri: String,
    #[serde(default)]
    redirect_uri: Option<String>,
}

fn default_auth_uri() -> String {
    AUTH_URI.to_string()
}

fn default_token_uri() -> String {
    TOKEN_URI.to_string()
}

/// The one client this build signs in with: the platform block when there is one, otherwise the
/// Desktop client and the loopback flow. `not_set_up` is returned for the placeholder values.
pub fn client(credentials_json: &str, not_set_up: &str) -> Result<margin_oauth::Client, String> {
    let parsed: CredentialsFile = serde_json::from_str(credentials_json)
        .map_err(|e| format!("invalid google-credentials.json: {e}"))?;

    #[cfg(target_os = "android")]
    let (creds, platform_client) = match parsed.android {
        Some(creds) => (creds, true),
        None => (parsed.installed, false),
    };
    #[cfg(target_os = "ios")]
    let (creds, platform_client) = match parsed.ios {
        Some(creds) => (creds, true),
        None => (parsed.installed, false),
    };
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    let (creds, platform_client) = (parsed.installed, false);

    if creds.client_id.starts_with("YOUR_CLIENT_ID")
        || creds
            .client_secret
            .as_deref()
            .is_some_and(|secret| secret.starts_with("YOUR_CLIENT_SECRET"))
    {
        return Err(not_set_up.to_string());
    }
    Ok(margin_oauth::Client {
        provider: "Google".to_string(),
        client_id: creds.client_id,
        client_secret: creds.client_secret,
        auth_uri: creds.auth_uri,
        token_uri: creds.token_uri,
        revoke_uri: REVOKE_URI.to_string(),
        userinfo_uri: USERINFO_URI.to_string(),
        auth_params: vec![("access_type".to_string(), "offline".to_string())],
        redirect_uri: creds.redirect_uri,
        platform_client,
    })
}

/// iOS clients must redirect to the reversed client id.
pub fn reversed_client_id(client_id: &str) -> String {
    let base = client_id
        .strip_suffix(".apps.googleusercontent.com")
        .unwrap_or(client_id);
    format!("com.googleusercontent.apps.{base}:/oauth2redirect")
}

/// The custom scheme redirect for a platform client. On Android it is `android_redirect`, the
/// package name form such as `studio.margin.mail:/oauth2redirect`.
pub fn redirect_uri(client: &margin_oauth::Client, android_redirect: &str) -> String {
    if let Some(explicit) = &client.redirect_uri {
        return explicit.clone();
    }
    if cfg!(target_os = "android") {
        android_redirect.to_string()
    } else {
        reversed_client_id(&client.client_id)
    }
}
