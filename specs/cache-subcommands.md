---
feature: cache-subcommands
status: accepted
dri: "@Dmitrze"
created: 2026-05-04
updated: 2026-05-04
related_agents: []
related_scenarios: ["tests/scenarios/cache-subcommands.md"]
related_runbooks: []
related_docs: ["docs/architecture.md#6-caching-and-rate-limits"]
---

# Cache CLI subcommands (`info` / `clear` / `prune`)

> `repo-trust cache info|clear|prune` replaces the Day-0 `bail!("not yet implemented")` stub. Useful for users debugging cache state when iterating on reports, rotating GitHub tokens, or freeing disk before a benchmark sweep.

---

## 1. Goal

Three subcommands operating on the SQLite cache file:
- `info` — print cache file path, on-disk size, per-table row counts, oldest + newest `fetched_at` for `api_cache`, and rate-limit reset window if known.
- `clear` — remove all rows from `api_cache` (and optionally `features` / `reports` if `--all` set), or scoped to a single repo via `--repo {owner/name}`.
- `prune` — remove `api_cache` rows whose `expires_at < now` (TTL-based eviction).

We know it works when: `cargo run -- cache info` after a scan shows non-zero row counts; `cache clear` zeroes them; `cache prune` removes only stale rows.

---

## 2. Non-functional requirements

- **Idempotent:** `clear` on an empty cache is a no-op (exit 0 with `0 rows deleted`).
- **Atomic:** `clear` and `prune` use a single `DELETE` statement per table — no partial state.
- **Confirmation-free:** `clear` does NOT prompt — the user explicitly invoked it. (Future flag `--yes` is a no-op for compat.)
- **No telemetry:** all operations local-only.

---

## 3. Boundaries

### In scope (Day 4)
- `src/cli/cache.rs::execute` replaces the `bail!` stub.
- `CacheArgs` enum with `Info`, `Clear { repo: Option<String>, all: bool }`, `Prune` variants.
- `Cache::info()` already exists (Day 1); reuse.
- New `Cache::clear_all() -> Result<usize>` (returns rows deleted).
- New `Cache::prune_expired() -> Result<usize>`.
- `Cache::delete_by_repo` already exists; reuse for `--repo` clear scope.
- ≥4 unit tests + 1 integration test (compiled binary against a temp cache).

### Out of scope (Day 4)
- LRU eviction — v1.1.
- Cache-size cap enforcement — `info` shows the soft cap from config but doesn't enforce; v1.1.

---

## 4. Probabilistic satisfaction threshold

N/A.

---

## 5. Happy-path scenarios

```
$ repo-trust cache info
Cache: /Users/dmitry/.repo-trust/cache.db (size: 2.4 MB)
  api_cache rows:  148  (oldest: 2026-05-01T..., newest: 2026-05-04T...)
  features rows:   18
  reports rows:    7
  soft cap:        500 MB

$ repo-trust cache prune
Pruned 12 expired entries from api_cache.

$ repo-trust cache clear --repo octocat/Hello-World
Cleared 8 entries for octocat/Hello-World from api_cache.

$ repo-trust cache clear --all
Cleared 173 entries (api_cache=148, features=18, reports=7).
```

---

## 6. Architecture sketch

```
cli::cache::execute(args):
  let cfg = config::load(...)?;
  let cache = Cache::open(cfg.cache.resolved_path())?;
  match args.command {
    Info => print_info(&cache.info()?),
    Clear { repo: Some(r), .. } => print(cache.delete_by_repo(&r)?),
    Clear { all: true, .. } => print(cache.clear_all()?),
    Clear { .. } => print(cache.clear_api_cache()?),
    Prune => print(cache.prune_expired()?),
  }
```

---

## 7. Closed loop

- **Goal metric:** unit tests + 1 integration test pass; manual run on developer laptop produces the expected output.
- **Where it lives:** CI; MemPalace `collectors/cache`.
- **Read by:** any developer iterating on the tool.
- **Improvement path:** v1.1 adds `--yes` flag, LRU eviction, size enforcement.

---

## 8. Definition of Done

- [ ] Spec status `accepted`.
- [ ] `tests/scenarios/cache-subcommands.md` lists ≥4 scenarios.
- [ ] `src/cli/cache.rs::execute` replaces the `bail!` stub.
- [ ] `CacheArgs` enum supports the 3 subcommands.
- [ ] `Cache::clear_all` + `Cache::prune_expired` implemented.
- [ ] ≥4 unit tests + 1 binary integration test.
- [ ] CHANGELOG entry.
- [ ] All quality gates green.

---

## 9. Open questions

- None.

---

## 10. Closed questions (history)

- 2026-05-04 — should `clear --all` prompt? — No. The user invoked the command; if they wanted a prompt they'd use `--interactive` (deferred to v1.1).

---

## 11. References

- `docs/architecture.md` §6 — caching.
- `src/storage/cache.rs` (Day 1).
