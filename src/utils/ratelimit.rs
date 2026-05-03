//! Token-bucket rate limiter for coordinating concurrent HTTP requests.
//!
//! See `docs/architecture.md` §6.4 and `specs/github-api-client.md`.
//!
//! The limiter wraps a [`tokio::sync::Semaphore`] (default 10 permits) and
//! tracks the most recent `X-RateLimit-Remaining` / `X-RateLimit-Reset`
//! headers. When the remaining budget drops below [`PAUSE_THRESHOLD`] the
//! next call to [`RateLimiter::acquire`] sleeps until the reset time.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use reqwest::Response;
use time::OffsetDateTime;
use tokio::sync::{Mutex, OwnedSemaphorePermit, Semaphore};

/// Default number of concurrent in-flight HTTP requests against GitHub.
pub const DEFAULT_PERMITS: usize = 10;
/// Below this remaining-budget we emit a `tracing::warn!`.
pub const WARN_THRESHOLD: u32 = 100;
/// Below this remaining-budget the next acquire blocks until the reset window.
pub const PAUSE_THRESHOLD: u32 = 10;

/// Cheap-to-clone rate-limit coordinator. Both `sem` and `state` are `Arc`,
/// so cloned instances share state.
#[derive(Debug, Clone)]
pub struct RateLimiter {
    sem: Arc<Semaphore>,
    state: Arc<Mutex<State>>,
}

#[derive(Debug, Default)]
struct State {
    remaining: Option<u32>,
    reset_at: Option<OffsetDateTime>,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(DEFAULT_PERMITS)
    }
}

impl RateLimiter {
    /// Build a new limiter with `permits` concurrent slots.
    #[must_use]
    pub fn new(permits: usize) -> Self {
        Self {
            sem: Arc::new(Semaphore::new(permits)),
            state: Arc::new(Mutex::new(State::default())),
        }
    }

    /// Acquire a permit. Pauses until the next reset window when the last
    /// observed remaining budget is below [`PAUSE_THRESHOLD`].
    pub async fn acquire(&self) -> Result<OwnedSemaphorePermit> {
        // Inspect last-known state under a short-lived lock and decide
        // whether to sleep. We release the state lock before sleeping so
        // other callers can still update it from concurrent responses.
        let pause = {
            let st = self.state.lock().await;
            match (st.remaining, st.reset_at) {
                (Some(r), Some(reset)) if r < PAUSE_THRESHOLD => {
                    let now = OffsetDateTime::now_utc();
                    if reset > now {
                        let wait = (reset - now).as_seconds_f64().max(0.0) + 1.0;
                        Some((r, wait))
                    } else {
                        None
                    }
                },
                _ => None,
            }
        };
        if let Some((remaining, wait)) = pause {
            tracing::warn!(remaining, wait_seconds = wait, "github rate-limit pause");
            tokio::time::sleep(Duration::from_secs_f64(wait)).await;
        }
        let permit = self.sem.clone().acquire_owned().await?;
        Ok(permit)
    }

    /// Inspect the rate-limit headers on `resp` and update internal state.
    /// Emits a `warn` event when `X-RateLimit-Remaining` falls below
    /// [`WARN_THRESHOLD`].
    pub async fn record(&self, resp: &Response) {
        let remaining = resp
            .headers()
            .get("x-ratelimit-remaining")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.parse::<u32>().ok());
        let reset_unix = resp
            .headers()
            .get("x-ratelimit-reset")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.parse::<i64>().ok());
        let mut st = self.state.lock().await;
        if let Some(r) = remaining {
            st.remaining = Some(r);
            if r < WARN_THRESHOLD {
                tracing::warn!(remaining = r, "github rate limit getting low");
            }
        }
        if let Some(reset) = reset_unix {
            st.reset_at = OffsetDateTime::from_unix_timestamp(reset).ok();
        }
    }

    /// Snapshot of the most recent observed rate-limit state. Useful for
    /// `--debug` output and tests.
    pub async fn snapshot(&self) -> (Option<u32>, Option<OffsetDateTime>) {
        let st = self.state.lock().await;
        (st.remaining, st.reset_at)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn acquire_returns_permit_under_normal_conditions() {
        let limiter = RateLimiter::new(2);
        let _p1 = limiter.acquire().await.unwrap();
        let _p2 = limiter.acquire().await.unwrap();
        // Snapshot before any record() should be all-None.
        let (rem, reset) = limiter.snapshot().await;
        assert!(rem.is_none() && reset.is_none());
    }

    #[tokio::test]
    async fn snapshot_reflects_recorded_headers() {
        let limiter = RateLimiter::new(1);
        // We can't construct a full reqwest::Response easily; instead test
        // the state machinery via a manual lock + push.
        {
            let mut st = limiter.state.lock().await;
            st.remaining = Some(42);
            st.reset_at = OffsetDateTime::from_unix_timestamp(1_700_000_000).ok();
        }
        let (rem, reset) = limiter.snapshot().await;
        assert_eq!(rem, Some(42));
        assert!(reset.is_some());
    }
}
