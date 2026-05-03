---
feature: cache-layer
status: accepted
spec: ../../specs/cache-layer.md
dri: "@Dmitrze"
created: 2026-05-03
updated: 2026-05-03
---

# Cache layer — Scenarios

Link: [`specs/cache-layer.md`](../../specs/cache-layer.md)

> Concrete success and failure cases for the SQLite-backed cache.

---

## Coverage at a glance

| Category | Count | Notes |
|---|---|---|
| Happy path | 2 | put/get round-trip; touch on 304 |
| Edge cases | 2 | TTL expiry; missing key |
| Failure modes | 2 | corrupted DB; concurrent writers |
| Security / privacy | 1 | 0600 perms on Unix |

---

## Happy path

### S-001: put then get returns same body

**Given** an empty cache opened at a tempfile path
**When** the caller calls `put("github:repos:octocat/Hello-World:metadata", Some("etag-1"), b"{...}", ttl=Duration::hours(24))`
**Then** a subsequent `get("github:repos:octocat/Hello-World:metadata")` returns `Some(CachedEntry { etag: Some("etag-1"), body: b"{...}", expires_at > now })`.

### S-002: touch updates fetched_at on 304

**Given** an entry exists with `fetched_at = T0`, `etag = Some("etag-1")`
**When** the caller invokes `touch(key)` after a 304 response
**Then** `fetched_at` advances to ≈ `now`; `etag` and `body` are unchanged.

---

## Edge cases

### S-101: get returns expired entry but flagged as stale

**Given** an entry with `expires_at = now - 1s`
**When** the caller calls `get(key)`
**Then** the entry is returned with `is_stale() == true`, so the caller can decide to send a conditional request rather than skip the network.

### S-102: get on missing key returns None

**Given** an empty cache
**When** the caller calls `get("never-stored")`
**Then** the result is `Ok(None)`. No error is raised.

---

## Failure modes

### S-201: corrupted DB file

**Given** the cache file at `~/.repo-trust/cache.db` is non-SQLite garbage
**When** `Cache::open(path)` is called
**Then** the call returns `Err` with exit code 5 (per architecture §8); the user sees an actionable message including the file path and the suggestion to `repo-trust cache clear`.

### S-202: concurrent writers under r2d2 pool

**Given** two `tokio::spawn`'d tasks both calling `put(...)` on the same key
**When** they execute concurrently
**Then** both succeed; the final stored entry is one of the two writes (last-write-wins); no panics, no `database is locked` errors leak to the caller.

---

## Security / privacy

### S-501: 0600 perms set on Unix

**Given** running on Unix
**When** `Cache::open(path)` is called and creates the file
**Then** the resulting file's mode bits are `0o600`. Verified via `std::fs::metadata().permissions().mode()`.

---

## How an agent reads this file

1. Match each scenario against the implementation behavior.
2. Failing scenario → fix the code or escalate.
3. Run the corresponding `cargo test --test cache_integration -- <scenario_name>` and report pass/fail.
