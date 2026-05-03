# 0005 — Federate upstream tools, do not replicate

## Status
Accepted (May 2026)

## Context

Multiple high-quality OSS tools already cover specific aspects of repo trust:
- **OpenSSF Scorecard** — 18 security health checks. Updated weekly to BigQuery, available via API at `api.scorecard.dev`.
- **deps.dev (Google Open Source Insights)** — dependency graph and package metadata for 50M+ packages. Public API.
- **OSV.dev** — open vulnerability database. Public API.
- **ecosyste.ms** — ecosystem-wide package metrics.

We could either:
1. Re-implement these checks ourselves.
2. Consume their outputs as inputs to our own modules.

## Decision

**Federate, do not replicate.** Repo Trust consumes upstream APIs and uses their outputs as inputs to our modules:

- `Security & Readiness` module pulls Scorecard score from `api.scorecard.dev`.
- `Adoption Signals` module pulls dependent counts and download stats from `deps.dev`.
- `Security & Readiness` module pulls vulnerability counts from OSV.dev.

We maintain our own collectors and feature pipelines for the things upstream tools do *not* cover — fake-star heuristics, multi-window activity analysis, maintainer-concentration metrics, ecosystem-aware adoption signals.

## Consequences

### Easier
- Lower maintenance burden (we don't re-build OpenSSF Scorecard's 18 checks).
- We benefit immediately from upstream improvements.
- We can credibly position as complementary, not competitive, with these projects.
- Our methodology gains weight by reusing peer-reviewed components.

### Harder
- We depend on upstream API availability.
- An upstream API change can break us.
- Rate limits stack: we are subject to GitHub + Scorecard + deps.dev + OSV limits simultaneously.

### Mitigations
- Aggressive caching with module-aware TTLs (Scorecard updates weekly → 7-day TTL).
- Graceful degradation: if Scorecard is unavailable, the module reports `Low confidence` rather than failing.
- Abstract API layer in `src/api/` per upstream so we can swap implementations.

## Alternatives considered

### Re-implement everything
**Why considered:** Maximum independence; no external dependency.

**Why rejected:** Wasteful. Scorecard's 18 checks took years to design and validate; replicating them adds zero user value.

### Federate from raw data sources only (GHArchive, BigQuery)
**Why considered:** Most independent.

**Why rejected:** GHArchive ingest is a heavy ETL pipeline. We are a CLI, not a data warehouse. We accept dependence on upstream APIs in exchange for being shippable.
