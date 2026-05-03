---
feature: adoption-signals-module
status: accepted
spec: ../../specs/adoption-signals-module.md
dri: "@Dmitrze"
created: 2026-05-05
updated: 2026-05-05
---

# Adoption Signals module — Scenarios

Link: [`specs/adoption-signals-module.md`](../../specs/adoption-signals-module.md)

---

## Coverage at a glance

| Category | Count | Notes |
|---|---|---|
| Happy path | 2 | popular package; minimal but documented |
| Edge cases | 3 | no published package; deps.dev outage; archived repo |
| Failure modes | 1 | README missing |

---

## Happy path

### S-001: popular package scores high

**Given** wiremock returns one Go package with `weekly_downloads = 100_000`, README present (>500 words), `docs/` and `examples/` directories present
**When** `AdoptionModule::run(&ctx)` is called
**Then** `result.score >= 75`; confidence = `High`; evidence includes Positive verdicts on `weekly_downloads` and `documentation_maturity`.

### S-002: minimal package but well-documented

**Given** weekly downloads ~1k, README + docs/ present (no examples/)
**When** `AdoptionModule::run` is called
**Then** `result.score` lands in `[40, 70]`; confidence = `High`; evidence is mixed (Neutral on downloads, Positive on docs).

---

## Edge cases

### S-101: no published package falls back gracefully

**Given** `deps_dev::Client::project_packages` returns empty Vec (research repo with no published package)
**When** `AdoptionModule::run` is called
**Then** confidence = `Medium`; `missing_data` includes `"no_packages"`; evidence has a Neutral `no_packages` item explaining this is a caveat, not a concern; module score is computed from doc maturity alone.

### S-102: deps.dev outage drops to Low confidence

**Given** `deps_dev::Client::project_packages` returns `Err` (5xx)
**When** `AdoptionModule::run` is called
**Then** the run does not abort; confidence = `Low`; `missing_data` includes `"deps_dev_unavailable"`; evidence has a Neutral caveat-item.

### S-103: archived repo demotes to Low + caveat

**Given** repo metadata `archived: true`
**When** `AdoptionModule::run` is called
**Then** confidence = `Low`; `missing_data` includes `"archived"`; evidence surfaces the archive as Neutral.

---

## Failure modes

### S-201: README endpoint 404 → degraded doc maturity

**Given** `GET /repos/.../.../readme` returns 404 (no README in default branch)
**When** `AdoptionModule::run` is called
**Then** `documentation_maturity` sub-score drops; evidence has a Concerning `no_readme` item.
