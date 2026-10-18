use std::future::Future;
use std::time::Duration;

use crate::HttpError;

/// Truncated at a minute and a bit.
pub const MAX_BACKOFF_MS: u64 = 64_000;

/// How many times a rate limited call is tried before the error reaches the caller.
pub const MAX_ATTEMPTS: u32 = 5;

/// `min(2^n seconds + jitter, 64s)`. Jitter is a parameter so the schedule is a pure function.
pub fn backoff_ms(attempt: u32, jitter_ms: u64) -> u64 {
    let exponential = 1_000u64.saturating_mul(1u64 << attempt.min(16));
    exponential.saturating_add(jitter_ms).min(MAX_BACKOFF_MS)
}

/// Up to a second, so clients that hit the same limit together do not come back in step.
pub fn jitter_ms() -> u64 {
    use rand::Rng;
    rand::thread_rng().gen_range(0..1_000)
}

pub fn backoff(attempt: u32) -> u64 {
    backoff_ms(attempt, jitter_ms())
}

/// A dropped connection is tried again after these waits, on a fresh connection from the pool.
pub const DROPPED_WAITS_MS: [u64; 2] = [250, 1_250];

/// Retries rate limited answers on the backoff schedule and dropped connections on the short one.
/// When attempts run out the error carries the delay the next attempt would have used. Only for
/// calls that are safe to make twice.
pub async fn with_retry<T, F, Fut>(call: F) -> Result<T, HttpError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, HttpError>>,
{
    retrying(call, true).await
}

/// `with_retry` for a call that must not be made twice: a dropped connection is reported.
pub async fn with_retry_no_replay<T, F, Fut>(call: F) -> Result<T, HttpError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, HttpError>>,
{
    retrying(call, false).await
}

async fn retrying<T, F, Fut>(mut call: F, replay_dropped: bool) -> Result<T, HttpError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, HttpError>>,
{
    let mut attempt = 0;
    let mut dropped = 0;
    loop {
        match call().await {
            Err(HttpError::RateLimited { retry_after_ms }) if attempt + 1 < MAX_ATTEMPTS => {
                let wait = retry_after_ms.max(backoff(attempt));
                tokio::time::sleep(Duration::from_millis(wait)).await;
                attempt += 1;
            }
            Err(HttpError::RateLimited { retry_after_ms }) => {
                return Err(HttpError::RateLimited {
                    retry_after_ms: retry_after_ms.max(backoff(attempt)),
                });
            }
            Err(HttpError::Dropped(_)) if replay_dropped && dropped < DROPPED_WAITS_MS.len() => {
                tokio::time::sleep(Duration::from_millis(DROPPED_WAITS_MS[dropped])).await;
                dropped += 1;
            }
            other => return other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_backoff_schedule_increases_and_is_bounded() {
        let delays: Vec<u64> = (0..10).map(|n| backoff_ms(n, 0)).collect();
        assert_eq!(delays[0], 1_000);
        for pair in delays.windows(2) {
            assert!(pair[1] >= pair[0], "{pair:?} went backwards");
        }
        assert!(delays.iter().all(|d| *d <= MAX_BACKOFF_MS));
        assert_eq!(*delays.last().expect("ten delays"), MAX_BACKOFF_MS);
    }

    #[test]
    fn the_backoff_is_jittered_rather_than_identical_every_time() {
        let mut seen = std::collections::HashSet::new();
        for _ in 0..50 {
            seen.insert(backoff(0));
        }
        assert!(seen.len() > 1, "every delay was the same: {seen:?}");
        assert!(seen.iter().all(|d| (1_000..2_000).contains(d)), "{seen:?}");
    }
}
