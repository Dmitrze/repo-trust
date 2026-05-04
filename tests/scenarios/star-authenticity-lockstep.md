---
feature: star-authenticity-lockstep
status: accepted
spec: ../../specs/star-authenticity-lockstep.md
dri: "@Dmitrze"
created: 2026-05-04
updated: 2026-05-04
---

# Star Authenticity — Heuristic 2 (lockstep) — Scenarios

Link: [`specs/star-authenticity-lockstep.md`](../../specs/star-authenticity-lockstep.md)

---

## Coverage at a glance

| Category | Count | Notes |
|---|---|---|
| Happy path | 2 | smooth distribution; bursty distribution |
| Edge cases | 3 | window too short; no starred_at; combined H1+H2 condition |
| Determinism | 1 | identical input → identical z-score |
| Language posture | 1 | rationale stays probabilistic; verdict stays Concerning ceiling |

---

## Happy path

### S-001: smooth distribution → low z-score, full credit

**Given** a 90-day daily series with stars uniformly between 0 and 5 per day
**When** `lockstep_z_score(series)` is called
**Then** the result is `Some(z)` with `z < 3`; the scorer's H2 sub-score is 100; evidence rationale says "max daily z-score = X.X (smooth distribution within the 28-day rolling baseline)".

### S-002: bursty distribution → high z-score, low credit

**Given** a 90-day series mostly 0-1/day with a 50-star spike on day 60
**When** `lockstep_z_score(series)` is called
**Then** result is `Some(z)` with `z >= 8`; H2 sub-score = 30; evidence verdict = `Concerning`.

---

## Edge cases

### S-101: window too short → None + caveat

**Given** a series spanning only 20 days
**When** `lockstep_z_score(series)` is called
**Then** result is `None` (need ≥35 days = 28 baseline + 7 lag); the scorer drops H2 from the formula and emits a Neutral `lockstep_window_too_short` caveat-evidence; final formula falls back to Day-3 redistribution `0.55 × H1 + 0.45 × H3`.

### S-102: no starred_at timestamps → None

**Given** the sample carries only `StargazerEntry::Plain` (no dates — happens when API doesn't return `vnd.github.star+json`)
**When** `compute()` is called
**Then** `StarsFeatures.lockstep_z_score = None`; scorer behaves as S-101.

### S-103: combined H1≥20% AND H2≥5 emits combined evidence

**Given** sample with low_activity_share = 0.30 and a bursty series with z = 7
**When** `score()` is called
**Then** evidence list contains a `combined_low_activity_and_lockstep` Neutral item with rationale acknowledging both signals are present; verdict on this combined-evidence item is `Concerning` — NEVER `HighRisk` per CLAUDE.md §14.

---

## Determinism

### S-401: identical input → identical z-score

**Given** the same `Vec<OffsetDateTime>` series
**When** `lockstep_z_score` is called twice
**Then** both calls return identical `Some(f64)` values byte-for-byte.

---

## Language posture

### S-501: recency_biased_sample evidence on every non-below-floor run

**Given** any non-below-floor inputs (≥ 50 stars)
**When** `score()` is called
**Then** evidence contains a `recency_biased_sample` Neutral item with the rationale documented in spec §3 (Q1 follow-through). Below-floor runs short-circuit before this item is added.
