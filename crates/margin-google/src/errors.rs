use margin_http::{DEFAULT_RETRY_MS, HttpError, status_error, strip_urls};
use serde::de::DeserializeOwned;

/// What a Google message that was nothing but a link becomes.
pub const NO_REASON: &str = "Google answered with a link and no reason";

/// Google's error body carries the same sentence three times. The useful line is
/// `error.message`; a body that will not parse is truncated rather than pasted.
pub fn explain(body: &str) -> String {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(body)
        && let Some(message) = value
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
    {
        return strip_urls(message, NO_REASON);
    }
    let trimmed = body.trim();
    let cut = match trimmed.char_indices().nth(200) {
        Some((end, _)) => format!("{}…", &trimmed[..end]),
        None => trimmed.to_string(),
    };
    strip_urls(&cut, NO_REASON)
}

/// The machine-readable reason lives in `error.status`, `error.errors[].reason` or
/// `error.details[].reason` depending on the API's age, so all three are collected.
fn reasons(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return out;
    };
    let Some(error) = value.get("error") else {
        return out;
    };
    if let Some(status) = error.get("status").and_then(|s| s.as_str()) {
        out.push(status.to_string());
    }
    for key in ["errors", "details"] {
        let Some(items) = error.get(key).and_then(|e| e.as_array()) else {
            continue;
        };
        for item in items {
            if let Some(reason) = item.get("reason").and_then(|r| r.as_str()) {
                out.push(reason.to_string());
            }
        }
    }
    out
}

fn mentions(found: &[String], wanted: &[&str]) -> bool {
    found
        .iter()
        .any(|reason| wanted.iter().any(|w| reason.eq_ignore_ascii_case(w)))
}

/// `dailyLimitExceeded` is absent on purpose: retrying it only spends the next day's.
fn is_rate_limit(found: &[String]) -> bool {
    mentions(
        found,
        &[
            "rateLimitExceeded",
            "userRateLimitExceeded",
            "RESOURCE_EXHAUSTED",
        ],
    )
}

fn is_missing_scope(found: &[String], message: &str) -> bool {
    mentions(
        found,
        &[
            "insufficientPermissions",
            "ACCESS_TOKEN_SCOPE_INSUFFICIENT",
            "insufficientScopes",
        ],
    ) || message.contains("insufficient authentication scopes")
}

fn is_api_disabled(found: &[String]) -> bool {
    mentions(found, &["accessNotConfigured", "SERVICE_DISABLED"])
}

/// A Google API answer to a neutral error. `scope` is what the call needed, since Google's body
/// never names it.
pub fn error_for(
    status: u16,
    context: &str,
    scope: &str,
    retry_after_ms: Option<u64>,
    body: &str,
) -> HttpError {
    let message = explain(body);
    let found = reasons(body);
    match status {
        403 if is_rate_limit(&found) => HttpError::RateLimited {
            retry_after_ms: retry_after_ms.unwrap_or(DEFAULT_RETRY_MS),
        },
        403 if is_missing_scope(&found, &message) => {
            HttpError::InsufficientScope(scope.to_string())
        }
        403 if is_api_disabled(&found) => HttpError::Other(format!(
            "{context} failed ({status}): the API is switched off in the Google Cloud \
             project this app's credentials belong to"
        )),
        _ => status_error(status, context, retry_after_ms, message),
    }
}

pub async fn read_json<T: DeserializeOwned>(
    resp: reqwest::Response,
    context: &str,
    scope: &str,
) -> Result<T, HttpError> {
    margin_http::read_json(resp, context, |status, retry, body| {
        error_for(status, context, scope, retry, body)
    })
    .await
}

pub async fn read_empty(
    resp: reqwest::Response,
    context: &str,
    scope: &str,
) -> Result<(), HttpError> {
    margin_http::read_empty(resp, |status, retry, body| {
        error_for(status, context, scope, retry, body)
    })
    .await
}

pub async fn read_bytes(
    resp: reqwest::Response,
    context: &str,
    scope: &str,
) -> Result<Vec<u8>, HttpError> {
    margin_http::read_bytes(resp, |status, retry, body| {
        error_for(status, context, scope, retry, body)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCOPE_MODIFY: &str = "https://www.googleapis.com/auth/gmail.modify";
    const SCOPE_SETTINGS: &str = "https://www.googleapis.com/auth/gmail.settings.basic";

    const HISTORY_GONE: &str = r#"{"error":{"code":404,
        "message":"Requested entity was not found.",
        "errors":[{"message":"Requested entity was not found.","domain":"global","reason":"notFound"}],
        "status":"NOT_FOUND"}}"#;

    const INSUFFICIENT_SCOPE: &str = r#"{"error":{"code":403,
        "message":"Request had insufficient authentication scopes.",
        "errors":[{"message":"Insufficient Permission","domain":"global","reason":"insufficientPermissions"}],
        "status":"PERMISSION_DENIED",
        "details":[{"@type":"type.googleapis.com/google.rpc.ErrorInfo","reason":"ACCESS_TOKEN_SCOPE_INSUFFICIENT","domain":"googleapis.com","metadata":{"method":"google.gmail.v1.GmailService.ListMessages","service":"gmail.googleapis.com"}}]}}"#;

    const RATE_LIMIT_429: &str = r#"{"error":{"code":429,
        "message":"User-rate limit exceeded.  Retry after 2026-09-03T12:00:04.000Z",
        "errors":[{"message":"User-rate limit exceeded.  Retry after 2026-09-03T12:00:04.000Z","domain":"usageLimits","reason":"rateLimitExceeded"}],
        "status":"RESOURCE_EXHAUSTED"}}"#;

    const RATE_LIMIT_403: &str = r#"{"error":{"code":403,
        "message":"User Rate Limit Exceeded",
        "errors":[{"message":"User Rate Limit Exceeded","domain":"usageLimits","reason":"userRateLimitExceeded"}],
        "status":"PERMISSION_DENIED"}}"#;

    const INVALID_CREDENTIALS: &str = r#"{"error":{"code":401,
        "message":"Request had invalid authentication credentials. Expected OAuth 2 access token, login cookie or other valid authentication credential.",
        "errors":[{"message":"Invalid Credentials","domain":"global","reason":"authError","location":"Authorization","locationType":"header"}],
        "status":"UNAUTHENTICATED"}}"#;

    const DAILY_LIMIT: &str = r#"{"error":{"code":403,
        "message":"Daily Limit Exceeded",
        "errors":[{"message":"Daily Limit Exceeded","domain":"usageLimits","reason":"dailyLimitExceeded"}],
        "status":"PERMISSION_DENIED"}}"#;

    const API_DISABLED: &str = r#"{"error":{"code":403,
        "message":"Gmail API has not been used in project 123456 before or it is disabled. Enable it by visiting https://console.developers.google.com/apis/api/gmail.googleapis.com/overview?project=123456 then retry. If you enabled this API recently, wait a few minutes for the action to propagate to our systems and retry.",
        "errors":[{"message":"Gmail API has not been used in project 123456 before or it is disabled.","domain":"usageLimits","reason":"accessNotConfigured","extendedHelp":"https://console.developers.google.com"}],
        "status":"PERMISSION_DENIED"}}"#;

    const INVALID_CREDENTIALS_WITH_LINK: &str = r#"{"error":{"code":401,
        "message":"Request had invalid authentication credentials. Expected OAuth 2 access token, login cookie or other valid authentication credential. See https://developers.google.com/identity/sign-in/web/devconsole-project.",
        "status":"UNAUTHENTICATED"}}"#;

    #[test]
    fn a_404_is_a_not_found() {
        assert!(matches!(
            error_for(404, "Gmail history", SCOPE_MODIFY, None, HISTORY_GONE),
            HttpError::NotFound(_)
        ));
    }

    #[test]
    fn a_403_naming_an_insufficient_scope_carries_the_scope_the_call_needed() {
        assert_eq!(
            error_for(
                403,
                "Gmail message list",
                SCOPE_MODIFY,
                None,
                INSUFFICIENT_SCOPE
            ),
            HttpError::InsufficientScope(SCOPE_MODIFY.to_string())
        );
        assert_eq!(
            error_for(
                403,
                "Gmail send-as list",
                SCOPE_SETTINGS,
                None,
                INSUFFICIENT_SCOPE
            ),
            HttpError::InsufficientScope(SCOPE_SETTINGS.to_string())
        );
    }

    #[test]
    fn both_rate_limit_answers_are_rate_limited_and_carry_a_delay() {
        let limited = HttpError::RateLimited {
            retry_after_ms: DEFAULT_RETRY_MS,
        };
        assert_eq!(
            error_for(429, "Gmail history", SCOPE_MODIFY, None, RATE_LIMIT_429),
            limited
        );
        assert_eq!(
            error_for(403, "Gmail history", SCOPE_MODIFY, None, RATE_LIMIT_403),
            limited
        );
        assert_eq!(
            error_for(
                429,
                "Gmail history",
                SCOPE_MODIFY,
                Some(8_000),
                RATE_LIMIT_429
            ),
            HttpError::RateLimited {
                retry_after_ms: 8_000
            }
        );
    }

    #[test]
    fn a_401_is_a_signed_out_account_and_keeps_googles_sentence() {
        let HttpError::Unauthorized(message) = error_for(
            401,
            "Gmail history",
            SCOPE_MODIFY,
            None,
            INVALID_CREDENTIALS,
        ) else {
            panic!("a 401 is Unauthorized");
        };
        assert!(message.starts_with("Request had invalid authentication credentials."));
        assert!(!message.contains("UNAUTHENTICATED"));
    }

    #[test]
    fn a_daily_limit_is_not_retried_as_a_rate_limit() {
        assert!(matches!(
            error_for(403, "Gmail send", SCOPE_MODIFY, None, DAILY_LIMIT),
            HttpError::Other(_)
        ));
    }

    #[test]
    fn no_error_ever_carries_a_link() {
        let disabled = error_for(403, "Gmail history", SCOPE_MODIFY, None, API_DISABLED);
        let HttpError::Other(said) = &disabled else {
            panic!("a switched off API is an Other, got {disabled:?}");
        };
        assert!(!said.contains("http"), "{said}");
        assert!(said.contains("switched off"), "{said}");

        let HttpError::Unauthorized(said) = error_for(
            401,
            "Gmail history",
            SCOPE_MODIFY,
            None,
            INVALID_CREDENTIALS_WITH_LINK,
        ) else {
            panic!("a 401 is Unauthorized");
        };
        assert!(!said.contains("http"), "{said}");
        assert!(
            said.starts_with("Request had invalid authentication credentials."),
            "{said}"
        );
        assert!(said.ends_with("valid authentication credential."), "{said}");

        let HttpError::Other(said) = error_for(
            400,
            "Gmail label change",
            SCOPE_MODIFY,
            None,
            r#"{"error":{"message":"Invalid id value","status":"INVALID_ARGUMENT"}}"#,
        ) else {
            panic!("a 400 is an Other");
        };
        assert_eq!(said, "Gmail label change failed (400): Invalid id value");
    }

    #[test]
    fn a_message_that_was_only_a_link_says_so() {
        assert_eq!(
            explain(r#"{"error":{"message":"See https://developers.google.com/identity"}}"#),
            NO_REASON
        );
    }

    #[test]
    fn an_unparseable_body_is_truncated_rather_than_pasted_whole() {
        let body = "x".repeat(5000);
        let HttpError::Other(message) = error_for(400, "Gmail send", SCOPE_MODIFY, None, &body)
        else {
            panic!("a 400 is an Other");
        };
        assert!(
            message.chars().count() < 260,
            "was {}",
            message.chars().count()
        );
        assert!(message.ends_with('…'));
    }
}
