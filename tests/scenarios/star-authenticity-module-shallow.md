---
feature: star-authenticity-module-shallow
status: accepted
spec: ../../specs/star-authenticity-module-shallow.md
dri: "@Dmitrze"
created: 2026-05-05
updated: 2026-05-05
---

# Star Authenticity module (shallow) — Scenarios

Link: [`specs/star-authenticity-module-shallow.md`](../../specs/star-authenticity-module-shallow.md)

---

## Coverage at a glance

| Category | Count | Notes |
|---|---|---|
| Happy path | 2 | organic-looking; suspicious-looking |
| Edge cases | 4 | tiny repo (<50 stars); new repo; small sample; ecosystem multiplier |
| Determinism | 1 | same seed → same sample |
| Language posture | 1 | rationale uses only probabilistic phrasing |

---

## Happy path

### S-001: organic profile scores high

**Given** sampled 200 stargazers, only 4% match the 9-signal low-activity profile, fork/star ratio = 0.10, watcher/star ratio = 0.02
**When** `StarsModule::run(&ctx)` is called
**Then** `result.score >= 80`; confidence = `High`; evidence has Positive verdicts on `low_activity_stargazer_share`, `fork_to_star_ratio`, `watcher_to_star_ratio`.

### S-002: suspicious profile lowers score (Concerning, not HighRisk)

**Given** sampled 200 stargazers, 38% match the low-activity profile, fork/star ratio = 0.005, watcher/star = 0.0005
**When** `StarsModule::run` is called
**Then** `result.score <= 30`; evidence on `low_activity_stargazer_share` has verdict `Concerning` (NOT `HighRisk` standalone, per methodology §Module 1 + CLAUDE.md §14); rationale is probabilistic ("38% of sampled stargazers match a low-activity profile") — never "fake" / "fraud" / "bot".

---

## Edge cases

### S-101: tiny repo (<50 stars) skips with Low confidence

**Given** repo metadata `stargazers_count = 25`
**When** `StarsModule::run` is called
**Then** `result.confidence = Low`; `missing_data` includes `"below_sampling_floor"`; evidence has a Neutral item explaining the sample floor.

### S-102: new repo (<6 months) gets 5pp leniency

**Given** repo `created_at` 60 days ago; sampled 200 stargazers, 22% match low-activity profile
**When** `StarsModule::run` is called
**Then** the 20% concern threshold has shifted to 25%; the 22% share lands in the previous band; rationale calls out the leniency.

### S-103: sample <100 demotes confidence to Medium

**Given** stargazers_count = 60; sample = 60
**When** `StarsModule::run` is called
**Then** confidence = `Medium`; evidence has a Neutral `small_sample` caveat.

### S-104: ecosystem multiplier shifts ratio bands

**Given** TypeScript repo (multiplier 0.7 for fork/star, 0.8 for watcher/star); fork/star = 0.06
**When** `StarsModule::run` is called
**Then** the ratio sub-score reflects the adjusted threshold; evidence rationale mentions the ecosystem-aware adjustment.

---

## Determinism

### S-401: same seed → same sample → same scores

**Given** the same wiremock fixture with 1,000 stargazers, the same `(repo, scoring_version)` blake3-derived seed
**When** `StarsModule::run` is invoked twice
**Then** both runs produce byte-identical `(score, sub_scores, evidence)` output. ChaCha20Rng deterministic sampling is enforced.

---

## Language posture

### S-501: rationale uses probabilistic phrasing only

**Given** any input shape
**When** `StarsModule::run` is called
**Then** **no** evidence rationale contains the substrings `"fake"`, `"fraud"`, or `"bot"`; the verbiage is `"X% of sampled stargazers match a low-activity profile"` per `methodology.md` §Module 1 + `CLAUDE.md` §14.
