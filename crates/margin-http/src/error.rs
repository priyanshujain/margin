#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HttpError {
    /// 401. Refreshing will not help.
    Unauthorized(String),
    /// A 403 for a missing OAuth scope, carrying the scope the call needed.
    InsufficientScope(String),
    /// 429, server overload, or a provider's rate limit answer. Carries how long to wait.
    RateLimited {
        retry_after_ms: u64,
    },
    NotFound(String),
    /// 410. On a sync call, the sync token is dead.
    Gone,
    /// 412. The etag did not match, so someone else won.
    Conflict,
    /// Could not reach the server: DNS, refused connection, timeout.
    Offline(String),
    /// A connection that was there and then was not. The one failure worth retrying at once.
    Dropped(String),
    Other(String),
}

impl std::fmt::Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpError::Unauthorized(m) => write!(f, "signed out: {m}"),
            HttpError::InsufficientScope(s) => write!(f, "missing permission: {s}"),
            HttpError::RateLimited { retry_after_ms } => {
                write!(f, "rate limited, retry in {retry_after_ms}ms")
            }
            HttpError::NotFound(m) => write!(f, "not found: {m}"),
            HttpError::Gone => write!(f, "gone: the resource no longer exists"),
            HttpError::Conflict => write!(f, "conflict: it changed elsewhere"),
            HttpError::Offline(m) => write!(f, "offline: {m}"),
            HttpError::Dropped(m) => write!(f, "connection lost: {m}"),
            HttpError::Other(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for HttpError {}

impl From<HttpError> for String {
    fn from(e: HttpError) -> String {
        e.to_string()
    }
}

/// A timeout is Offline rather than Dropped: the time has already gone, and retrying on the spot
/// is more of the same.
impl From<reqwest::Error> for HttpError {
    fn from(e: reqwest::Error) -> HttpError {
        let dropped = !e.is_timeout() && (e.is_request() || e.is_body());
        let offline = e.is_connect() || e.is_timeout();
        let text = transport_text(e);
        if dropped {
            HttpError::Dropped(text)
        } else if offline {
            HttpError::Offline(text)
        } else {
            HttpError::Other(text)
        }
    }
}

/// reqwest's text without the URL and with the source chain, where the useful part lives.
fn transport_text(e: reqwest::Error) -> String {
    let e = e.without_url();
    let mut text = e.to_string();
    let mut source = std::error::Error::source(&e);
    while let Some(cause) = source {
        let said = cause.to_string();
        if !said.is_empty() && !text.contains(&said) {
            text.push_str(": ");
            text.push_str(&said);
        }
        source = cause.source();
    }
    strip_urls(&text, "the server answered with a link and no reason")
}

/// The text with every sentence that carried a link taken out, or `fallback` when nothing is left.
pub fn strip_urls(text: &str, fallback: &str) -> String {
    let has_link = |s: &str| s.contains("http://") || s.contains("https://") || s.contains("www.");
    if !has_link(text) {
        return text.trim().to_string();
    }
    let mut kept: Vec<&str> = Vec::new();
    let mut start = 0;
    let bytes = text.as_bytes();
    for (at, byte) in bytes.iter().enumerate() {
        let ends = *byte == b'.'
            && bytes
                .get(at + 1)
                .is_none_or(|next| next.is_ascii_whitespace());
        if ends {
            kept.push(&text[start..=at]);
            start = at + 1;
        }
    }
    if start < text.len() {
        kept.push(&text[start..]);
    }
    let out = kept
        .into_iter()
        .map(str::trim)
        .filter(|s| !s.is_empty() && !has_link(s))
        .collect::<Vec<_>>()
        .join(" ");
    if out.is_empty() {
        fallback.to_string()
    } else {
        out
    }
}

/// When a retryable answer arrives without a `Retry-After`, this is what the caller waits.
pub const DEFAULT_RETRY_MS: u64 = 1_000;

/// The status-to-meaning decision every provider shares. A provider that reads its error bodies
/// decides its 403s first and hands the rest here.
pub fn status_error(
    status: u16,
    context: &str,
    retry_after_ms: Option<u64>,
    message: String,
) -> HttpError {
    match status {
        401 => HttpError::Unauthorized(message),
        404 => HttpError::NotFound(message),
        410 => HttpError::Gone,
        412 => HttpError::Conflict,
        408 | 429 | 500 | 502 | 503 | 504 => HttpError::RateLimited {
            retry_after_ms: retry_after_ms.unwrap_or(DEFAULT_RETRY_MS),
        },
        _ => HttpError::Other(format!("{context} failed ({status}): {message}")),
    }
}

/// Seconds, per RFC 9110. The date form is not parsed.
pub fn retry_after(headers: &reqwest::header::HeaderMap) -> Option<u64> {
    headers
        .get(reqwest::header::RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim()
        .parse::<u64>()
        .ok()
        .map(|seconds| seconds.saturating_mul(1000))
}

#[cfg(test)]
mod tests {
    use super::*;

    const NO_REASON: &str = "Google answered with a link and no reason";

    #[test]
    fn a_link_goes_with_its_sentence_and_a_message_that_was_only_a_link_says_so() {
        assert_eq!(
            strip_urls(
                "Enable it by visiting https://console.developers.google.com/x then retry. Then wait.",
                NO_REASON
            ),
            "Then wait."
        );
        assert_eq!(
            strip_urls("Invalid id value", NO_REASON),
            "Invalid id value"
        );
        assert_eq!(
            strip_urls("See https://developers.google.com/identity", NO_REASON),
            NO_REASON
        );
        assert_eq!(
            strip_urls("error sending request: connection reset by peer", NO_REASON),
            "error sending request: connection reset by peer"
        );
    }

    #[test]
    fn a_gateway_timeout_is_a_bad_day_and_not_a_refusal() {
        assert!(matches!(
            status_error(504, "history", None, String::new()),
            HttpError::RateLimited { .. }
        ));
        assert!(matches!(
            status_error(408, "history", None, String::new()),
            HttpError::RateLimited { .. }
        ));
        assert_eq!(
            status_error(429, "history", Some(8_000), String::new()),
            HttpError::RateLimited {
                retry_after_ms: 8_000
            }
        );
    }
}
