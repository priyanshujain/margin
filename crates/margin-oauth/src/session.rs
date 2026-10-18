use std::collections::HashMap;
use std::future::Future;
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::sync::{Mutex, MutexGuard};

use crate::TokenResponse;

/// Refresh this many seconds early, so a request in flight when the clock rolls over is not a 401.
pub const EXPIRY_SKEW_SECS: u64 = 60;

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// When a token that lives `expires_in` seconds should be treated as expired.
pub fn expiry_at(expires_in: u64) -> u64 {
    now() + expires_in.saturating_sub(EXPIRY_SKEW_SECS)
}

#[derive(Default)]
pub struct Session {
    pub access_token: Option<String>,
    pub access_expiry: u64,
    pub email: Option<String>,
}

/// One session per account id.
#[derive(Default)]
pub struct Sessions {
    map: Mutex<HashMap<String, Session>>,
}

impl Sessions {
    pub async fn lock(&self) -> MutexGuard<'_, HashMap<String, Session>> {
        self.map.lock().await
    }

    /// A live access token, refreshed through `refresh` when the cached one has expired.
    ///
    /// Single-flight: the map lock is held across the refresh, so concurrent callers queue on one
    /// token request. That serializes refreshes across accounts too, which is the right trade for
    /// something that happens once an hour per account. `refresh` stores a rotated refresh token.
    pub async fn access_token<F, Fut>(&self, account_id: &str, refresh: F) -> Result<String, String>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<TokenResponse, String>>,
    {
        let mut sessions = self.map.lock().await;
        if let Some(session) = sessions.get(account_id)
            && let Some(token) = &session.access_token
            && now() < session.access_expiry
        {
            return Ok(token.clone());
        }

        let tokens = refresh().await?;
        let session = sessions.entry(account_id.to_string()).or_default();
        session.access_token = Some(tokens.access_token.clone());
        session.access_expiry = expiry_at(tokens.expires_in);
        Ok(tokens.access_token)
    }
}
