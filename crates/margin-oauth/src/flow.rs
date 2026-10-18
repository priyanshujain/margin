use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand::RngCore;
use sha2::{Digest, Sha256};

use crate::Client;
use crate::session::now;

pub fn random_b64(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    rand::thread_rng().fill_bytes(&mut buf);
    URL_SAFE_NO_PAD.encode(buf)
}

pub fn pkce_challenge(verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(hasher.finalize())
}

fn urlencode(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

/// The base list plus extras, space separated. Installed apps get no incremental authorization,
/// so extras are always asked for alongside everything else.
pub fn scope_string(base: &[&str], extra: &[String]) -> String {
    let mut scopes: Vec<&str> = base.to_vec();
    for scope in extra {
        let scope = scope.trim();
        if !scope.is_empty() && !scopes.contains(&scope) {
            scopes.push(scope);
        }
    }
    scopes.join(" ")
}

/// What the token response says was granted, which is not always what was asked for.
pub fn granted_scopes(scope: Option<&str>) -> Vec<String> {
    scope
        .unwrap_or_default()
        .split_whitespace()
        .map(str::to_string)
        .collect()
}

/// The consent URL. A hint skips the account chooser and goes straight to consent for that
/// address; without one the chooser is shown so a signed-in account is not silently reused.
pub fn auth_url(
    client: &Client,
    redirect: &str,
    challenge: &str,
    csrf: &str,
    scopes: &str,
    hint: Option<&str>,
) -> String {
    let prompt = match hint {
        Some(_) => "consent",
        None => "select_account consent",
    };
    let mut url = format!(
        "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&code_challenge={}&code_challenge_method=S256&state={}",
        client.auth_uri,
        urlencode(&client.client_id),
        urlencode(redirect),
        urlencode(scopes),
        challenge,
        urlencode(csrf),
    );
    for (key, value) in &client.auth_params {
        url.push_str(&format!("&{}={}", urlencode(key), urlencode(value)));
    }
    url.push_str(&format!("&prompt={}", urlencode(prompt)));
    if let Some(hint) = hint {
        url.push_str(&format!("&login_hint={}", urlencode(hint)));
    }
    url
}

/// `ASWebAuthenticationSession` matches on the bare scheme.
pub fn callback_scheme(redirect: &str) -> &str {
    redirect.split(':').next().unwrap_or(redirect)
}

/// A consent round trip on the deep link flow that has left for the browser and not come back.
pub struct Pending {
    pub state: String,
    pub verifier: String,
    pub redirect: String,
    pub expires: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Callback {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
}

pub fn parse_callback(incoming: &url::Url) -> Callback {
    let mut callback = Callback::default();
    for (key, value) in incoming.query_pairs() {
        match key.as_ref() {
            "code" => callback.code = Some(value.into_owned()),
            "state" => callback.state = Some(value.into_owned()),
            "error" => callback.error = Some(value.into_owned()),
            _ => {}
        }
    }
    callback
}

/// Takes the pending request when the state matches, so a replayed link cannot start a second
/// exchange. A link whose state matches nothing is stale or forged, and `None` means ignore it.
pub fn take_pending(slot: &mut Option<Pending>, state: &str) -> Option<Pending> {
    match slot.as_ref() {
        Some(p) if p.state == state => slot.take(),
        _ => None,
    }
}

/// The code to exchange, or the error to report, for a callback that matched `pending`.
pub fn resolve_callback(
    pending: &Pending,
    callback: Callback,
    provider: &str,
) -> Result<String, String> {
    if let Some(error) = callback.error {
        return Err(format!("{provider} authorization failed: {error}"));
    }
    if now() > pending.expires {
        return Err("The sign-in took too long. Try connecting again.".to_string());
    }
    callback
        .code
        .ok_or_else(|| format!("{provider} returned no authorization code."))
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE_SCOPES: [&str; 3] = [
        "openid",
        "email",
        "https://www.googleapis.com/auth/gmail.modify",
    ];

    fn client() -> Client {
        Client {
            provider: "Google".to_string(),
            client_id: "123.apps.googleusercontent.com".to_string(),
            client_secret: Some("secret".to_string()),
            auth_uri: "https://accounts.google.com/o/oauth2/auth".to_string(),
            token_uri: "https://oauth2.googleapis.com/token".to_string(),
            revoke_uri: "https://oauth2.googleapis.com/revoke".to_string(),
            userinfo_uri: "https://www.googleapis.com/oauth2/v2/userinfo".to_string(),
            auth_params: vec![("access_type".to_string(), "offline".to_string())],
            redirect_uri: None,
            platform_client: false,
        }
    }

    fn query(url: &str, key: &str) -> Option<String> {
        url::Url::parse(url)
            .ok()?
            .query_pairs()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.into_owned())
    }

    #[test]
    fn pkce_challenge_matches_the_rfc7636_vector() {
        assert_eq!(
            pkce_challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn a_verifier_is_url_safe_and_unpadded() {
        let verifier = random_b64(64);
        assert!(
            verifier
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        );
    }

    #[test]
    fn a_challenge_is_url_safe_and_derived_from_the_verifier() {
        let verifier = random_b64(64);
        let challenge = pkce_challenge(&verifier);
        assert!(
            challenge
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        );
        assert_eq!(challenge, pkce_challenge(&verifier));
        assert_ne!(challenge, pkce_challenge(&random_b64(64)));
    }

    #[test]
    fn the_state_parameter_survives_the_round_trip() {
        let csrf = random_b64(24);
        let url = auth_url(
            &client(),
            "http://127.0.0.1:41234",
            &pkce_challenge(&random_b64(64)),
            &csrf,
            &scope_string(&BASE_SCOPES, &[]),
            None,
        );
        let returned = query(&url, "state").expect("a state parameter");
        assert_eq!(returned, csrf);
        assert_eq!(
            crate::parse_redirect(&format!("/?code=4/0Ab&state={returned}"), &csrf),
            crate::Redirect::Code("4/0Ab".to_string())
        );
    }

    #[test]
    fn the_consent_url_carries_the_scopes_and_the_redirect() {
        let url = auth_url(
            &client(),
            "http://127.0.0.1:41234",
            "challenge",
            "csrf",
            &scope_string(
                &BASE_SCOPES,
                &["https://www.googleapis.com/auth/drive.file".to_string()],
            ),
            None,
        );
        assert_eq!(
            query(&url, "redirect_uri").as_deref(),
            Some("http://127.0.0.1:41234")
        );
        assert_eq!(
            query(&url, "code_challenge_method").as_deref(),
            Some("S256")
        );
        assert_eq!(query(&url, "access_type").as_deref(), Some("offline"));
        assert_eq!(
            query(&url, "prompt").as_deref(),
            Some("select_account consent")
        );
        assert!(query(&url, "login_hint").is_none());
        let scopes = query(&url, "scope").expect("a scope parameter");
        assert!(scopes.starts_with("openid email "));
        assert!(scopes.ends_with(" https://www.googleapis.com/auth/drive.file"));
    }

    #[test]
    fn a_grant_names_the_account_it_is_for() {
        let url = auth_url(
            &client(),
            "http://127.0.0.1:41234",
            "challenge",
            "csrf",
            &scope_string(&BASE_SCOPES, &[]),
            Some("someone@example.test"),
        );
        assert_eq!(
            query(&url, "login_hint").as_deref(),
            Some("someone@example.test")
        );
        assert_eq!(query(&url, "prompt").as_deref(), Some("consent"));
    }

    #[test]
    fn extra_scopes_are_asked_for_alongside_the_base_list_and_never_alone() {
        let scopes = scope_string(
            &BASE_SCOPES,
            &["https://www.googleapis.com/auth/calendar.events".to_string()],
        );
        let asked: Vec<&str> = scopes.split(' ').collect();
        assert_eq!(&asked[..BASE_SCOPES.len()], &BASE_SCOPES[..]);
        assert_eq!(
            asked.last(),
            Some(&"https://www.googleapis.com/auth/calendar.events")
        );
    }

    #[test]
    fn an_extra_that_is_already_asked_for_is_not_asked_for_twice() {
        let scopes = scope_string(
            &BASE_SCOPES,
            &[BASE_SCOPES[2].to_string(), "  ".to_string()],
        );
        assert_eq!(scopes, BASE_SCOPES.join(" "));
    }

    #[test]
    fn the_granted_list_comes_from_the_token_response() {
        let response: crate::TokenResponse = serde_json::from_str(
            r#"{"access_token":"at","expires_in":3599,"scope":"openid https://www.googleapis.com/auth/gmail.modify","token_type":"Bearer"}"#,
        )
        .expect("parse");
        assert_eq!(
            granted_scopes(response.scope.as_deref()),
            vec![
                "openid".to_string(),
                "https://www.googleapis.com/auth/gmail.modify".to_string()
            ]
        );
    }

    #[test]
    fn a_token_response_without_a_scope_field_parses_to_nothing_granted() {
        let response: crate::TokenResponse =
            serde_json::from_str(r#"{"access_token":"at","expires_in":3599}"#).expect("parse");
        assert!(response.scope.is_none());
        assert!(granted_scopes(response.scope.as_deref()).is_empty());
        assert!(granted_scopes(Some("   ")).is_empty());
    }
}
