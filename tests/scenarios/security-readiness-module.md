---
feature: security-readiness-module
status: accepted
spec: ../../specs/security-readiness-module.md
dri: "@Dmitrze"
created: 2026-05-04
updated: 2026-05-04
---

# Security & Readiness module — Scenarios

Link: [`specs/security-readiness-module.md`](../../specs/security-readiness-module.md)

---

## Coverage at a glance

| Category | Count | Notes |
|---|---|---|
| Happy path | 2 | Scorecard available; no Scorecard fallback |
| Edge cases | 3 | stale Scorecard (30-90d); CODEOWNERS at non-default path; missing LICENSE |
| Failure modes | 1 | Scorecard 5xx |

---

## Happy path

### S-001: repo with recent Scorecard scores high

**Given** wiremock returns Scorecard `score=8.7, date=12d ago` for the repo plus all docs present (SECURITY.md, CONTRIBUTING.md, CODE_OF_CONDUCT.md, LICENSE, CODEOWNERS) and a CI workflow file
**When** `SecurityModule::run(&ctx)` is called
**Then** `result.score ≥ 75`; confidence = `High`; evidence includes Positive verdicts on `scorecard_score`, `has_security_md`, `has_license`, `ci_workflow_present`.

### S-002: repo with no Scorecard falls back gracefully

**Given** wiremock returns `404` for the Scorecard endpoint; doc-presence checks succeed for LICENSE only
**When** `SecurityModule::run` is called
**Then** `result.score` is computed from doc + CI signals only; confidence = `Low`; evidence includes a Neutral item: "Scorecard has not yet scored this repository"; `missing_data` contains `"scorecard"`.

---

## Edge cases

### S-101: stale Scorecard (30-90 days old) lowers weight

**Given** Scorecard report exists with `date = 60d ago`
**When** `SecurityModule::run` is called
**Then** scorecard weight applied = 0.30 (vs 0.40 for fresh); module confidence = `Medium`; evidence includes a Neutral item noting the Scorecard age.

### S-102: CODEOWNERS at non-default path is detected

**Given** wiremock returns 404 for `/.github/CODEOWNERS` but 200 for `/CODEOWNERS`
**When** the doc-presence check runs
**Then** `has_codeowners = true`; evidence verdict on `has_codeowners` is `Positive`.

### S-103: missing LICENSE is Concerning

**Given** wiremock returns 404 for `/LICENSE`, `/LICENSE.md`, `/LICENSE.txt`, `/COPYING`
**When** `SecurityModule::run` is called
**Then** evidence includes an item with code `has_license`, value `false`, verdict `Concerning`.

---

## Failure modes

### S-201: Scorecard 5xx is a real error

**Given** wiremock returns `503` for the Scorecard endpoint
**When** `SecurityModule::run` is called
**Then** the run errors out; CLI maps to exit code 7 per architecture §8 (this differs from the 404 fallback path: 5xx is transient, 404 is "not yet scored").
