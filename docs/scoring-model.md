# Scoring Model — Versioned Weights, Thresholds, and Change Log

> The scoring model has its own SemVer, separate from the CLI version. A user can pin `--scoring-version 1.0.0` and trust that running the tool today gives the same answer as running it next year (modulo upstream API state).

---

## Current scoring version

**`1.1.1`** — Adoption calibration patch on top of 1.1.0: deps.dev `relationProvenance` + name-match + min-version filter on the project-packages response, and a widened "documented" predicate in the Adoption confidence rule. See change-log entry below.

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

### `1.1.1` — 2026-05-05

**Adoption — calibration patch.** Two narrow tweaks identified during the H1 sweep follow-up.

1. `is_well_documented` widens the confidence rule's "documented" predicate: `documentation_maturity_score >= 0.50` OR `has_readme && (has_docs_dir || has_examples_dir)`. Promotes idiomatic library-project-layout repos (short README + `examples/`, like clap-rs/clap) to High confidence when they have packages and aren't archived. The doc-maturity *score* is unchanged.
2. The deps.dev `:packageversions` response is filtered by a combined heuristic before being projected onto `Vec<PackageRef>`:
   - Verified `relationProvenance` ∈ `{GO_ORIGIN}` (the only verified provenance deps.dev emits as of mid-2026; CARGO/NPM/PYPI/MAVEN entries all come back as `UNVERIFIED_METADATA`), **OR** owner-aware name match (CARGO/`tokio` for `tokio-rs/tokio`, NPM/`@octocat/hello-world` for `octocat/Hello-World`, GO/`github.com/owner/repo` for `owner/repo`); **AND**
   - At least two distinct versions per `(system, name)` group, to filter out single-tagged demo repos that appear as `GO_ORIGIN` simply because every GitHub repo with one git tag is reachable as a Go module path.

   Eliminates the `octocat/Hello-World` transitive-mention false-positive (where `package_systems_count` was inflated by appearances in unrelated packages' `SOURCE_REPO` field).

This is a patch bump because the confidence rule shape (the truth table) is unchanged from 1.1.0; only the constituent predicates got more generous (documented) or more strict (first-party). Per-module score arithmetic and weights are unchanged. Per-module Adoption *scores* can shift on real-world inputs because `package_systems_count` may now be lower (filtered) or unchanged; overall scores can shift by ±1 point on repos whose adoption confidence tier moved (confidence-weighted aggregation in `aggregate.rs`).

**Empirical note.** The architect's prompt anticipated a per-ecosystem verified provenance enum (`CARGO_OWNER` / `PYPI_OWNER` / etc.). Across the captured fixtures, deps.dev only emits `GO_ORIGIN` and `UNVERIFIED_METADATA`. The combined-heuristic filter (verified-provenance OR name-match) is the empirical adaptation.

### `1.1.0` — 2026-05-04

**Adoption — confidence rule re-tiered.** deps.dev v3 dropped the `weeklyDownloads` field from the project-packages endpoint as of mid-2026 (verified empty across CARGO / NPM / GO / PYPI / MAVEN). The previous Adoption confidence rule gated `High` on a downloads floor and was no longer satisfiable, dragging every real-world report's confidence to `Medium` and through confidence-weighting every overall confidence to `Medium`.

The 1.1.0 rule grades Adoption confidence on:

1. ecosystem coverage from the new `:packageversions` endpoint (`package_systems_count > 0`),
2. archived state, and
3. documentation maturity (≥ 0.50 on the maturity scale).

See `docs/methodology.md#confidence-scoring-110` for the truth table and `src/scoring/adoption.rs::compute_confidence` for the implementation.

Module weights, score arithmetic, sub-score thresholds, and the four other modules are **unchanged**. Per-module adoption scores are unchanged for any inputs (we changed confidence semantics, not weights). Overall scores can shift by ±1 point on repos whose adoption confidence changed tier, because confidence is a multiplicative weight in the aggregator (`src/scoring/aggregate.rs`).

The `weekly_downloads` sub-score and evidence row are kept additively in the model — they will light up automatically if downloads come back from any source. Adding a cross-ecosystem download federation (PyPI / npm / crates.io stats APIs) is on the v0.2 roadmap as a separate scoring-version bump.

The `no_packages` evidence row is now correctly emitted only when `package_systems_count == 0` (it previously fired whenever `weekly_downloads` was None — a proxy that became universally true under deps.dev v3, including for repos with multiple published packages). When ≥1 ecosystem is detected, a new `ecosystem_coverage` evidence row is emitted instead — Positive verdict for ≥2 ecosystems, Neutral for 1.

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
