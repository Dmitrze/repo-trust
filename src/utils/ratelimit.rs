//! Token-bucket rate limiter for coordinating concurrent HTTP requests.
//!
//! See `docs/architecture.md` §6.4. Inspects `X-RateLimit-Remaining` and
//! pauses when remaining drops below the configured threshold.

// TODO: implement on top of tokio::sync::Semaphore + reqwest middleware.
