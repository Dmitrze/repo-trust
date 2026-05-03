//! Local SQLite-backed cache.
//!
//! See [`docs/architecture.md` §6](../../docs/architecture.md) for the full
//! schema and TTL policy. The cache exposes three tables — `api_cache`,
//! `features`, `reports` — through the [`Cache`] facade, plus a
//! [`migrations`] module that runs schema migrations on first connection.

pub mod cache;
pub mod migrations;

pub use cache::{Cache, CacheInfo, CachedEntry};
