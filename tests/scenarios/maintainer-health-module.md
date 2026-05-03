---
feature: maintainer-health-module
status: accepted
spec: ../../specs/maintainer-health-module.md
dri: "@Dmitrze"
created: 2026-05-04
updated: 2026-05-04
---

# Maintainer Health module — Scenarios

Link: [`specs/maintainer-health-module.md`](../../specs/maintainer-health-module.md)

---

## Coverage at a glance

| Category | Count | Notes |
|---|---|---|
| Happy path | 2 | multi-maintainer healthy; solo with caveat |
| Edge cases | 3 | bot-author exclusion; new repo; archived |
| Failure modes | 1 | contributors endpoint 500 |

---

## Happy path

### S-001: multi-maintainer healthy repo scores high

**Given** wiremock fixture for a repo with 5 active contributors over 365d, balanced commit distribution (Gini ≈ 0.3), CODEOWNERS present
**When** `MaintainersModule::run(&ctx)` is called
**Then** `result.score ≥ 80`; confidence = `High`; evidence includes Positive verdicts on `bus_factor_proxy` (≥4) and `commit_concentration` (Gini ≤0.4).

### S-002: solo-maintainer repo flagged but not "High Risk"

**Given** wiremock fixture for a repo with 1 contributor (90% of commits) over 365d, no governance docs
**When** `MaintainersModule::run(&ctx)` is called
**Then** `result.score` is in `[30, 60]`; confidence = `Medium`; evidence includes a `Concerning` (NOT `HighRisk`) item with code `solo_maintainer` and rationale "many excellent OSS projects are solo-maintained".

---

## Edge cases

### S-101: bot commits are excluded

**Given** fixture has 100 commits — 80 by `dependabot[bot]` and 20 by `alice`
**When** `MaintainersModule::run` is called
**Then** features layer counts 1 author (alice); evidence does NOT include dependabot in `top_authors`.

### S-102: new repo (<6 months old) gets Low confidence

**Given** repo `created_at` was 30 days ago
**When** `MaintainersModule::run` is called
**Then** confidence = `Low`; evidence includes a Neutral item explaining "repo too young for stable maintainer baseline".

### S-103: archived repo demotes to Low confidence with caveat

**Given** repo metadata `archived: true`
**When** `MaintainersModule::run` is called
**Then** confidence = `Low`; `missing_data` includes `"archived"`; module score still computed from real commits but caveat surfaces in the report.

---

## Failure modes

### S-201: contributors endpoint 500 partway through

**Given** wiremock returns `500` for `GET /repos/.../.../contributors`
**When** `MaintainersModule::run` is called
**Then** the run does not abort; the contributors-derived `active_maintainers` sub-score is dropped; `missing_data` includes `"contributors"`; confidence drops one band.
