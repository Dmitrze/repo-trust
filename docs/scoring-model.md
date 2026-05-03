# Scoring Model — Versioned Weights, Thresholds, and Change Log

> The scoring model has its own SemVer, separate from the CLI version. A user can pin `--scoring-version 1.0.0` and trust that running the tool today gives the same answer as running it next year (modulo upstream API state).

---

## Current scoring version

**`1.0.0`** — unreleased; ships with the first public release.

## Default module weights (v1.0.0)

| Module | Weight | Rationale |
| --- | --- | --- |
| Star Authenticity | 0.20 | Most-asked question; novel value over existing tools |
| Activity Health | 0.25 | Strongest single predictor of long-term project survival |
| Maintainer Health | 0.20 | Bus-factor risk is real and underweighted by popularity-only views |
| Adoption Signals | 0.20 | Real-world usage is the antidote to vanity metrics |
| Security & Readiness | 0.15 | Critical but well-served by OSSF Scorecard; we federate, not replicate |

Weights sum to 1.00. They are configurable via `--weights weights.toml` and via env var `REPO_TRUST_WEIGHTS`.

## Confidence factors

| Confidence | Numeric factor (used in aggregation) |
| --- | --- |
| High | 1.00 |
| Medium | 0.75 |
| Low | 0.50 |

## Category buckets

| Score range | Category |
| --- | --- |
| 85–100 | Strong |
| 70–84 | Good |
| 50–69 | Mixed |
| 30–49 | Weak |
| 0–29 | High Risk |

## Module weight presets

Three presets ship with the binary (selectable via `--weights-preset`):

### `default` (described above)

### `security_first`
```toml
stars       = 0.10
activity    = 0.20
maintainers = 0.20
adoption    = 0.15
security    = 0.35
```
For users who care most about CI pipeline integration risk.

### `lenient`
```toml
stars       = 0.10
activity    = 0.30
maintainers = 0.15
adoption    = 0.30
security    = 0.15
```
For users who weight "is this useful?" over "is this trustworthy?". Use for early-stage discovery, not for production-dependency diligence.

### `strict`
```toml
stars       = 0.25
activity    = 0.25
maintainers = 0.25
adoption    = 0.10
security    = 0.15
```
For users who care most about authenticity signals (e.g. funds doing diligence on "VC-bait" repos).

---

## Change log

### `1.0.0` — unreleased

Initial scoring model. Establishes:
- Five-module structure (see [ADR-0006](adr/0006-five-modules.md)).
- Default weights 20 / 25 / 20 / 20 / 15.
- StarScout / Dagster heuristic-based fake-star detection.
- Federation with Scorecard, deps.dev, OSV.
- Confidence factors 0.5 / 0.75 / 1.0.
- Category buckets at 30 / 50 / 70 / 85.
- JSON report `schema_version: 1.0.0`.

### Breaking-change policy

A scoring-model major bump is reserved for changes that would meaningfully alter scores for the same input. Examples:
- Changing default weights.
- Changing low-activity profile thresholds.
- Adding a sixth module (would shift weights).

A minor bump is for additive changes that don't alter existing scores:
- New evidence codes (display-only).
- New optional sub-signals.
- New presets.

A patch bump is for documentation refinements and non-semantic clarifications.
