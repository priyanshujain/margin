use std::sync::LazyLock;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::Client;

static HTTP: LazyLock<reqwest::Client> =
    LazyLock::new(|| margin_http::client(margin_http::Profile::Auth));

/// The client every token request goes through.
pub fn http() -> &'static reqwest::Client {
    &HTTP
}

/// Reads the body to a String first so the provider's error payload survives into the message.
pub async fn read_json<T: DeserializeOwned>(
    resp: reqwest::Response,
    context: &str,
) -> Result<T, String> {
    let status = resp.status();
    let text = resp.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!("{context} failed ({status}): {text}"));
    }
    serde_json::from_str(&text).map_err(|e| format!("{context}: could not parse response: {e}"))
}

#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub expires_in: u64,
    #[serde(default)]
    pub id_token: Option<String>,
    /// Space separated, and absent from a refresh response.
    #[serde(default)]
    pub scope: Option<String>,
}

#[derive(Deserialize)]
struct UserInfo {
    #[serde(default)]
    email: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct IdClaims {
    /// The provider's stable user id, which survives an email change.
    #[serde(default)]
    pub sub: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
}

/// The id_token came over TLS straight from the token endpoint, so only the payload is read.
pub fn id_token_claims(id_token: &str) -> Option<IdClaims> {
    let payload = id_token.split('.').nth(1)?;
    let bytes = URL_SAFE_NO_PAD.decode(payload.trim_end_matches('=')).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// A public client omits `client_secret` entirely, since an empty one is rejected.
fn with_secret<'a>(
    client: &'a Client,
    mut form: Vec<(&'a str, &'a str)>,
) -> Vec<(&'a str, &'a str)> {
    if let Some(secret) = client.client_secret.as_deref() {
        form.push(("client_secret", secret));
    }
    form
}

fn form_body(pairs: &[(&str, &str)]) -> String {
    let mut form = url::form_urlencoded::Serializer::new(String::new());
    for (key, value) in pairs {
        form.append_pair(key, value);
    }
    form.finish()
}

const FORM_TYPE: (&str, &str) = ("content-type", "application/x-www-form-urlencoded");

async fn post_form(uri: &str, form: &[(&str, &str)]) -> Result<reqwest::Response, String> {
    HTTP.post(uri)
        .header(FORM_TYPE.0, FORM_TYPE.1)
        .body(form_body(form))
        .send()
        .await
        .map_err(|e| e.to_string())
}

pub async fn exchange_code(
    client: &Client,
    code: &str,
    redirect: &str,
    verifier: &str,
) -> Result<TokenResponse, String> {
    let form = with_secret(
        client,
        vec![
            ("client_id", client.client_id.as_str()),
            ("code", code),
            ("code_verifier", verifier),
            ("grant_type", "authorization_code"),
            ("redirect_uri", redirect),
        ],
    );
    let resp = post_form(&client.token_uri, &form).await?;
    read_json(resp, &format!("{} token exchange", client.provider)).await
}

pub async fn refresh_access_token(
    client: &Client,
    refresh_token: &str,
) -> Result<TokenResponse, String> {
    let form = with_secret(
        client,
        vec![
            ("client_id", client.client_id.as_str()),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ],
    );
    let resp = post_form(&client.token_uri, &form).await?;
    read_json(resp, &format!("{} token refresh", client.provider)).await
}

pub async fn fetch_email(client: &Client, access_token: &str) -> Result<String, String> {
    let resp = HTTP
        .get(&client.userinfo_uri)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let info: UserInfo = read_json(resp, &format!("{} account lookup", client.provider)).await?;
    Ok(info.email.unwrap_or_default())
}

/// Hands the grant back. The answer is kept: a revoke that never arrived leaves the grant standing.
pub async fn revoke(client: &Client, token: &str) -> Result<(), String> {
    let resp = post_form(&client.revoke_uri, &[("token", token)]).await?;
    let status = resp.status();
    if status.is_success() {
        return Ok(());
    }
    let text = resp.text().await.unwrap_or_default();
    Err(format!("{} answered {status}: {text}", client.provider))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_token_claims_reads_the_subject_and_email() {
        let payload = URL_SAFE_NO_PAD.encode(br#"{"sub":"11829","email":"a@b.test","aud":"x"}"#);
        let claims = id_token_claims(&format!("header.{payload}.signature")).expect("claims");
        assert_eq!(claims.sub.as_deref(), Some("11829"));
        assert_eq!(claims.email.as_deref(), Some("a@b.test"));
    }

    #[test]
    fn a_malformed_id_token_is_none_rather_than_a_panic() {
        assert!(id_token_claims("not-a-jwt").is_none());
        assert!(id_token_claims("header..signature").is_none());
    }
}
