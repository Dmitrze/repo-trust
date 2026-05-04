---
feature: cache-subcommands
status: accepted
spec: ../../specs/cache-subcommands.md
dri: "@Dmitrze"
created: 2026-05-04
updated: 2026-05-04
---

# Cache subcommands — Scenarios

Link: [`specs/cache-subcommands.md`](../../specs/cache-subcommands.md)

---

## Coverage at a glance

| Category | Count | Notes |
|---|---|---|
| Happy path | 3 | info; clear; prune |
| Edge cases | 2 | empty cache info; --repo scope |

---

## Happy path

### S-001: cache info prints path + counts + sizes

**Given** a cache populated with 5 api_cache + 2 features + 1 reports rows
**When** `repo-trust cache info` runs
**Then** stdout contains: cache file path, file size in bytes/KB/MB, "api_cache rows: 5", "features rows: 2", "reports rows: 1", soft cap.

### S-002: cache clear removes everything by default

**Given** a populated cache
**When** `repo-trust cache clear` runs (no flags)
**Then** all `api_cache` rows are deleted; `features` and `reports` are NOT touched (clear without --all is api_cache-only); stdout reports the count.

### S-003: cache prune removes only expired rows

**Given** the cache has 3 fresh + 4 expired `api_cache` rows
**When** `repo-trust cache prune` runs
**Then** 4 rows are deleted; the 3 fresh rows remain; stdout reports "Pruned 4 expired entries".

---

## Edge cases

### S-101: empty cache info doesn't crash

**Given** an empty cache (just-migrated, no rows)
**When** `repo-trust cache info` runs
**Then** stdout shows zeroes for all row counts; exit 0.

### S-102: clear --repo scopes deletion to one repo

**Given** the cache has rows for `acme/widget` and `octocat/Hello-World`
**When** `repo-trust cache clear --repo acme/widget` runs
**Then** only `acme/widget` rows are deleted; `octocat/Hello-World` rows remain; stdout reports the count.
