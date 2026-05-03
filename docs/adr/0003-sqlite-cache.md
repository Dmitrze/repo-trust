# 0003 — Local cache: SQLite via rusqlite

## Status
Accepted (May 2026)

## Context

The tool aggressively caches API responses to:
1. Stay within GitHub API rate limits (5,000 req/hr authenticated).
2. Make warm-cache scans complete in under 5 seconds.
3. Enable batch mode (100+ repos) without exhausting limits.
4. Support ETag-aware conditional fetching (`If-None-Match`).

We need a storage backend that is:
- Embedded (no external server).
- Cross-platform (Linux/macOS/Windows).
- Concurrency-safe for multi-tasked async workloads.
- Schema-versionable.
- Capable of holding tens of MB to a few GB without performance issues.

## Decision

Use **SQLite** via `rusqlite` (with the `bundled` feature) and `r2d2 + r2d2_sqlite` for connection pooling. Schema migrations live in `src/storage/migrations/` and run on first connection via `rusqlite_migration`.

Default path: `~/.repo-trust/cache.db` (Linux/macOS) or `%APPDATA%\repo-trust\cache.db` (Windows), resolved via `dirs` crate.

## Consequences

### Easier
- Zero external dependencies; SQLite is bundled into the binary.
- Battle-tested, ACID, well-documented schema migration patterns.
- WAL mode gives us concurrent reads with a single writer — perfect for our async batch mode.
- Easy to inspect (`sqlite3 ~/.repo-trust/cache.db`).
- ETag tracking is just one TEXT column.

### Harder
- A single corrupted cache file can break the tool (mitigated: clear, atomic backup before any schema migration; user-runnable `repo-trust cache clear`).
- Bundling SQLite adds ~1MB to the binary (acceptable for the value).
- The `bundled` feature requires a C toolchain at build time (acceptable for a Rust-native project).

## Alternatives considered

### `sled` (pure-Rust embedded KV)
**Why considered:** Pure Rust, no C dependency, fast.

**Why rejected:** Pre-1.0; smaller community; we want a proper relational schema for evidence/features tables, not a KV store.

### Plain JSON files in a directory
**Why considered:** Maximum simplicity. No deps.

**Why rejected:** Concurrent-write coordination becomes a problem in batch mode. Inefficient lookup. ETag tracking becomes a sidecar mess.

### Redis / external server
**Why considered:** Best concurrency story.

**Why rejected:** External server requirement is a non-starter for a CLI tool that should `just work` after `cargo install`.

### Mmap-backed custom format
**Why considered:** Fastest possible reads.

**Why rejected:** Engineering cost dwarfs the benefit at our scale. SQLite is fast enough.
