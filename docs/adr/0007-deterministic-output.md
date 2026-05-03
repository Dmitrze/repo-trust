# 0007 — Deterministic JSON output

## Status
Accepted (May 2026)

## Context

Users will:
- Diff JSON reports in CI to detect score regressions.
- Build internal dashboards keyed off the JSON shape.
- Cite scores in audits and reports that need to be defensible months later.
- Snapshot scores into version control alongside their security policies.

If the same inputs produce different JSON across runs, all of these uses become unreliable.

## Decision

The CLI is **deterministic** in the strict sense:

> Same inputs (`repo`, `mode`, `scoring_version`, `weights`, `rng_seed`) + same upstream API state → byte-identical JSON report (excluding `snapshot_at` and `runtime_seconds` fields which are explicitly time-based).

Mechanism:
1. All sampling uses `rand_chacha::ChaCha20Rng::seed_from_u64(seed)`. Default seed is derived from `(repo, scoring_version)` via `blake3` hash, so it's stable across runs without explicit `--seed`.
2. All sorts use `BTreeMap` / `Vec::sort_by_key` with explicit keys. We never rely on `HashMap` insertion order.
3. Floats in feature computation are rounded to 6 decimals before JSON serialization via a `serde_with` custom serializer.
4. Evidence items are sorted by `(module, code)` alphabetically before serialization.
5. Snapshot tests via `insta` enforce determinism: a fixture set of cached API responses is replayed by `wiremock`; CI fails on snapshot drift.

## Consequences

### Easier
- JSON diffing in CI just works.
- Reproducing a historical score requires only the inputs, not the random state.
- Users can pin `scoring_version` and trust that running the tool tomorrow gives the same answer as today (modulo upstream changes).
- Snapshot testing is reliable.

### Harder
- Concurrent computation must merge deterministically (we use `JoinSet::join_all` then sort, never first-completed-wins).
- Floating-point arithmetic must be carefully ordered (associativity is not guaranteed in IEEE 754); we round consistently and use `Decimal` where ordering matters.
- Adding any non-deterministic component (e.g. wall-clock-keyed cache eviction) requires explicit isolation from the scoring path.

## Alternatives considered

### "Best effort" determinism
**Why considered:** Simpler implementation.

**Why rejected:** Erodes trust in the reproducibility claim. Users will diff JSON, find spurious changes, and stop trusting the tool.

### Determinism only on the score, not the full JSON
**Why considered:** Easier to maintain.

**Why rejected:** Evidence ordering is part of the report's information content; reordering changes which evidence "appears first" in `top_concerns`.
