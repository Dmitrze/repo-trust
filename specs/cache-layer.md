---
feature: cache-layer
status: accepted
dri: "@Dmitrze"
created: 2026-05-03
updated: 2026-05-03
related_agents: []
related_scenarios: ["tests/scenarios/cache-layer.md"]
related_runbooks: []
related_docs: ["docs/architecture.md#6-caching-and-rate-limits", "docs/adr/0003-sqlite-cache.md"]
---

# Cache layer

> Local SQLite-backed cache that stores raw upstream API responses (with ETags), computed feature snapshots, and final reports. Powers ETag-aware conditional fetching and cross-run determinism. The first dependency every other module pulls in.

---

## 1. Goal

Every collector reads/writes through `storage::cache::Cache` so that (a) repeat scans of the same repo are warm-cache fast, (b) ETag-conditional GitHub fetches do not consume rate-limit budget on `304 Not Modified`, and (c) feature/report snapshots survive across CLI invocations.

We know it worked when: the second `repo-trust scan octocat/Hello-World` against the same repo state runs in <5s warm, the GitHub API call count drops to ≈0 net consumption (all 304s), and the SQLite file at `~/.repo-trust/cache.db` is 0600 on Unix.

---

## 2. Non-functional requirements

- **Warm-cache p95:** scan latency <5s for a previously-scanned repo.
- **Concurrency:** r2d2 pool size = 8 connections; safe under `JoinSet`-driven parallel module execution.
- **Schema migration:** `rusqlite_migration` runs migrations in order on first connection; idempotent on repeat opens.
- **File permissions:** `0600` on Unix via `std::os::unix::fs::PermissionsExt`. No-op on Windows.
- **Cache size:** soft cap 500 MB via LRU eviction (LRU eviction itself is v1.1 — Phase 1 cap is documentary, eviction is manual via `repo-trust cache prune`).
- **Determinism:** TTL-based reads return identical bodies for identical keys; no clock-influenced sort order in returned data.

---

## 3. Boundaries

### In scope (Phase 1)
- r2d2 pool over `SqliteConnectionManager`.
- `migrations/0001_initial.sql` with the three tables from architecture §6.1: `api_cache`, `features`, `reports`.
- ETag CRUD: `get(key) → Option<CachedEntry { etag, body, fetched_at, expires_at }>`, `put(key, etag, body, ttl)`.
- Feature snapshot CRUD keyed by `(repo, module, scoring_ver)`.
- Report CRUD keyed by `(repo, mode, scoring_ver, computed_at)`.
- `cache info` / `cache clear` / `cache prune` CLI subcommand wiring (the existing stub).
- `0600` perms on Unix.
- CacheHandle field added to `RepositoryContext`.

### Out of scope (explicit)
- LRU eviction logic (manual prune is enough for v1).
- Cross-host distributed cache.
- Encryption-at-rest (cache contains only public API responses, no tokens).
- Cache key versioning across schema changes (deferred to v1.1; manual cache clear documented in CHANGELOG).

---

## 4. Probabilistic satisfaction threshold

N/A — deterministic feature.

---

## 5. Happy-path scenario

1. `repo-trust scan octocat/Hello-World --mode standard` invoked for the first time.
2. `Cache::open("~/.repo-trust/cache.db")` runs `0001_initial.sql` migration; file created with 0600.
3. Activity collector calls `Cache::get("github:repos:octocat/Hello-World:metadata")` → `None`.
4. Collector hits GitHub, receives `200 OK` + `ETag: "abc123"` + body; calls `Cache::put(key, Some("abc123"), body, ttl=24h)`.
5. Same scan rerun 1 minute later: `Cache::get` returns the cached entry with `expires_at` in the future; collector skips the GitHub call entirely.
6. Same scan rerun 25 hours later: `Cache::get` returns the entry but `expires_at` is past; collector sends `If-None-Match: "abc123"`; GitHub returns `304`; collector calls `Cache::touch(key)` to update `fetched_at`.

---

## 6. Architecture sketch

```
[ Collector ] --get(key)--> [ Cache (r2d2 pool) ] --SQL--> [ ~/.repo-trust/cache.db ]
                                  ^
[ Collector ] --put(key, etag, body, ttl)--+
[ Collector ] --touch(key) ----------------+
[ Cache::open(path) ] --runs--> [ migrations/0001_initial.sql ]
```

Reference: `docs/architecture.md` §6 for full schema and TTL table.

---

## 7. Closed loop

- **Goal metric:** `tests/cache_integration.rs::etag_roundtrip` passes; `cache info` shows row counts after a real scan.
- **Where it lives:** CI test output; MemPalace `collectors/cache` room.
- **Read by:** Reviewer agent on PR; Verifier on Day 5 validation sweep.
- **Improvement path:** if v1.0 users hit cache-corruption issues, add a `cache verify` integrity check in v1.1.

---

## 8. Definition of Done

- [ ] Spec status `accepted`.
- [ ] `tests/scenarios/cache-layer.md` lists ≥5 scenarios.
- [ ] `src/storage/cache.rs` exposes `Cache::open`, `get`, `put`, `touch`, `delete_by_key`, `delete_by_repo`, `info`.
- [ ] `src/storage/migrations/0001_initial.sql` defines the three tables.
- [ ] r2d2 pool size 8; `tokio::task::spawn_blocking` wrapping for async use.
- [ ] Unix perms `0600` set after open.
- [ ] Unit tests in `src/storage/cache.rs::tests` cover round-trip, TTL expiry, key not found, idempotent migration.
- [ ] Integration test in `tests/cache_integration.rs` over a `tempfile::NamedTempFile`-backed DB.
- [ ] `RepositoryContext` carries a `Cache` handle.
- [ ] CHANGELOG `[Unreleased]` entry.
- [ ] No new runtime crates (r2d2, r2d2_sqlite, rusqlite_migration, rusqlite already in Cargo.toml).
- [ ] `cargo fmt`, `cargo clippy --all-targets --all-features`, `cargo test --all-features` all green.

---

## 9. Open questions

- None — all design choices captured in architecture §6 and ADR-0003.

---

## 10. Closed questions (history)

- 2026-05-03 — should cache keys be content-hashed via blake3? — No, URL-keyed for v1 per architecture §15 open Q (deferred to post-v1.0).

---

## 11. References

- `docs/architecture.md` §6 — caching and rate limits, full schema.
- `docs/adr/0003-sqlite-cache.md` — why SQLite + rusqlite.
- `tests/scenarios/cache-layer.md` — concrete scenarios.
