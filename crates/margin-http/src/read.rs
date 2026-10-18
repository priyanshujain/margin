use serde::de::DeserializeOwned;

use crate::{HttpError, retry_after};

/// The body becomes a String first, so an error payload reaches `on_error` instead of being lost
/// to a deserialisation failure. `on_error` gets the status, the `Retry-After` in milliseconds and
/// the body.
pub async fn read_json<T: DeserializeOwned>(
    resp: reqwest::Response,
    context: &str,
    on_error: impl FnOnce(u16, Option<u64>, &str) -> HttpError,
) -> Result<T, HttpError> {
    let status = resp.status().as_u16();
    let retry = retry_after(resp.headers());
    let text = resp.text().await?;
    if !(200..300).contains(&status) {
        return Err(on_error(status, retry, &text));
    }
    serde_json::from_str(&text)
        .map_err(|e| HttpError::Other(format!("{context}: could not parse response: {e}")))
}

/// For calls whose success is an empty body.
pub async fn read_empty(
    resp: reqwest::Response,
    on_error: impl FnOnce(u16, Option<u64>, &str) -> HttpError,
) -> Result<(), HttpError> {
    let status = resp.status().as_u16();
    let retry = retry_after(resp.headers());
    if (200..300).contains(&status) {
        return Ok(());
    }
    let text = resp.text().await.unwrap_or_default();
    Err(on_error(status, retry, &text))
}

pub async fn read_bytes(
    resp: reqwest::Response,
    on_error: impl FnOnce(u16, Option<u64>, &str) -> HttpError,
) -> Result<Vec<u8>, HttpError> {
    let status = resp.status().as_u16();
    let retry = retry_after(resp.headers());
    if !(200..300).contains(&status) {
        let text = resp.text().await.unwrap_or_default();
        return Err(on_error(status, retry, &text));
    }
    Ok(resp.bytes().await?.to_vec())
}
