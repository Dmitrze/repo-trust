---
feature: activity-health-module
status: accepted
spec: ../../specs/activity-health-module.md
dri: "@Dmitrze"
created: 2026-05-03
updated: 2026-05-03
---

# Activity Health module — Scenarios

Link: [`specs/activity-health-module.md`](../../specs/activity-health-module.md)

---

## Coverage at a glance

| Category | Count | Notes |
|---|---|---|
| Happy path | 2 | inactive baseline; healthy active |
| Edge cases | 3 | new repo; archived; issues disabled |
| Failure modes | 1 | partial collector failure |

---

## Happy path

### S-001: inactive repo (octocat/Hello-World) scores low

**Given** wiremock fixture for `octocat/Hello-World` (last commit > 365d ago, 0 commits last 90d, 0 active contributors last 90d, 0 releases ever)
**When** `ActivityModule::run(&ctx)` is called
**Then** the returned `ModuleResult.score ≤ 10`; confidence = `High` (data complete); evidence list contains ≥3 items including a Concerning verdict on `days_since_last_commit` and a Concerning verdict on `commits_last_90d`.

### S-002: high-activity repo scores high

**Given** wiremock fixture for `prometheus/prometheus` (last commit ≤ 7d ago, ≥100 commits in last 90d, ≥20 active contributors, recent release ≤30d)
**When** `ActivityModule::run(&ctx)` is called
**Then** `ModuleResult.score ≥ 80`; confidence = `High`; evidence list contains ≥3 Positive verdicts.

---

## Edge cases

### S-101: new repo (<6 months old) gets Low confidence

**Given** repo whose `created_at` was 14 days ago
**When** `ActivityModule::run` is called
**Then** confidence = `Low`; evidence includes a `Neutral` item explaining "repo too young for stable activity baseline".

### S-102: archived repo skipped with explicit caveat

**Given** repo metadata `archived: true`
**When** `ActivityModule::run` is called
**Then** `ModuleResult.score = 0` is *not* reported; instead, `missing_data: ["archived"]` and `confidence: Low`; the caller surfaces a top-level caveat in the report.

### S-103: issues disabled on the repo

**Given** repo metadata `has_issues: false`; commits/releases/PRs all present
**When** `ActivityModule::run` is called
**Then** `median_issue_first_response_hours: None`; that sub-score is dropped from the mean; confidence drops one band (`High → Medium`); evidence notes the missing input.

---

## Failure modes

### S-201: contributors endpoint returns 500 partway through collection

**Given** wiremock returns `500` for `GET /repos/.../.../contributors`
**When** `ActivityModule::run` is called
**Then** the run does not abort; `active_contributors_last_90d` is treated as `None`; that sub-score is dropped; `missing_data` includes `"contributors"`; confidence drops one band.
