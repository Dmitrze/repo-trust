---
feature: security-readiness-module
status: accepted
dri: "@Dmitrze"
created: 2026-05-04
updated: 2026-05-04
related_agents: []
related_scenarios: ["tests/scenarios/security-readiness-module.md"]
related_runbooks: []
related_docs: ["docs/methodology.md#module-5--security--readiness", "docs/module-specs.md#security--readiness"]
---

# Security & Readiness module

> Third end-to-end module. Federates OpenSSF Scorecard via `api::scorecard` and OSV via `api::osv`, plus its own doc-presence + CI-workflow + semver-consistency signals. When Scorecard has no score for the repo, the module degrades to Low confidence + doc-presence-only sub-scores rather than failing.

---

## 1. Goal

`SecurityModule::run(ctx)` returns `(ModuleResult, Vec<EvidenceItem>)` answering "is this repository in a state that supports responsible adoption?". We know it worked when: prometheus/prometheus (Scorecard score available) returns ≥75 with High confidence; octocat/Hello-World (no Scorecard) returns 30-60 with Low confidence + Neutral evidence ("Scorecard has not yet scored this repository").

---

## 2. Non-functional requirements

- **API budget:** ≤15 calls in Standard mode (1 Scorecard + 1 OSV per package + ≤8 doc-presence + 1 workflows listing + 1 tags listing). Day 2 Phase 1 OSV is wired but not actually called (no package list yet — comes Day 3 from Adoption); module collects 0 OSV advisories with a `defer_to_phase_3` caveat.
- **Federation policy:** per `methodology.md` §Module 5
  - Scorecard ≤30 days old: weight 0.40, confidence contribution High.
  - Scorecard 30-90 days old: weight 0.30, confidence contribution Medium.
  - Scorecard >90 days old or absent: ignored, module relies on doc + CI signals only with Low confidence.

---

## 3. Boundaries

### In scope (Day 2)
- `src/collectors/security.rs::collect` — Scorecard via `api::scorecard::Client`, doc presence (`SECURITY.md`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `LICENSE` — also `LICENSE.md`, `LICENSE.txt`, `COPYING` —, CODEOWNERS at 4 conventional paths), `.github/workflows/` listing via `Client::file_exists` on a probe path, semver-consistency from the existing release-tag list.
- `src/features/security.rs::compute` — `scorecard_score: Option<f64>`, `scorecard_age_days: Option<u64>`, `scorecard_checks_failed: Vec<String>`, `osv_open_advisories: u64` (always 0 in Day 2), `has_security_md: bool`, etc.
- `src/scoring/security.rs::score` — sub-scores: `scorecard_aggregate` (when available), `documentation_presence`, `ci_workflow_presence`, `osv_advisories` (0-count → 100). Final = weighted average per federation policy above.
- `src/modules/security.rs::run` — wires the three stages.
- ≥10 unit tests on scorer + 1 wiremock integration test (Scorecard 200 path) + 1 wiremock test (Scorecard 404 fallback).

### Out of scope (Day 2)
- OSV per-package query — wired but not invoked until Day 3 supplies package coordinates from Adoption.
- Branch-protection signals — requires the GitHub admin token scope; Phase 2.
- Re-running Scorecard checks ourselves (we federate per ADR-0005).
- Active CVE advisory triage UI — surface count + IDs only.

---

## 4. Probabilistic satisfaction threshold

N/A.

---

## 5. Happy-path scenario

1. `cli::scan::execute` picks `SecurityModule` from registry.
2. Collector calls `scorecard::Client::get(owner, repo)`.
3. Scorecard returns 200 + report (score = 8.7, ~18 checks, date = 12 days ago).
4. Collector calls `Client::file_exists` for each doc + a probe `.github/workflows/ci.yml`-like path; tags listed via existing releases call.
5. OSV intentionally not called in Day 2 (deferred); `osv_open_advisories = 0`, missing_data includes `osv_deferred_to_phase_3`.
6. Features computed; scorer applies federation policy weights; final score reflects Scorecard heavily.
7. Evidence: ≥3 items including `scorecard_score`, `has_security_md`, `has_license`, `ci_workflow_present`, plus a Neutral `osv_deferred_to_phase_3` caveat-evidence.

For 404 Scorecard scenario:
- Collector receives `Ok(None)` from `scorecard::Client::get`.
- Features `scorecard_score = None`.
- Scorer applies "no Scorecard" branch: sub-score weights renormalize to doc + CI only.
- Confidence forced to Low.
- Evidence includes a Neutral item: "Scorecard has not yet scored this repository".

---

## 6. Architecture sketch

```
[ Cached GH Client ] -----► doc presence + workflow probe + tags
                                                  |
[ Scorecard Client ] -----► get(owner, repo) → Some|None ScorecardReport
                                                  |
[ OSV Client ]      ----X (Day 2: not called; Day 3 wires per-package)
                                                  |
                                                  v
                                      SecurityCollector::collect → SecurityRawData
                                                  |
                                                  v
                                  SecurityFeatures::compute → SecurityFeatures
                                                  |
                                                  v
                                  scoring::security::score → (ModuleResult, Vec<EvidenceItem>)
                                                  |
                                                  v
                                       SecurityModule::run returns
```

---

## 7. Closed loop

- **Goal metric:** ≥10 scorer unit tests + 2 wiremock integration tests (Scorecard 200 + 404) green.
- **Where it lives:** CI; MemPalace `modules/security`.
- **Read by:** Reviewer; Day 5 rust-lang/cargo benchmark.
- **Improvement path:** Day 3 wires real OSV invocation once Adoption supplies `(repo → packages)` map.

---

## 8. Definition of Done

- [ ] Spec status `accepted`.
- [ ] `tests/scenarios/security-readiness-module.md` lists ≥6 scenarios.
- [ ] `src/collectors/security.rs::collect` implemented (federates Scorecard; doc-presence checks; CI workflow probe; semver consistency from tags).
- [ ] `src/features/security.rs::compute` + `src/modules/security.rs::run` implemented.
- [ ] `src/scoring/security.rs::score` implements federation-weighted aggregate.
- [ ] ≥10 unit tests on scorer.
- [ ] Wiremock tests cover Scorecard 200 + Scorecard 404 fallback + missing-LICENSE evidence.
- [ ] CHANGELOG entry.
- [ ] All quality gates green.

---

## 9. Open questions

- Should "semver consistent" require all tags to follow vX.Y.Z, or allow X.Y.Z without v? Day 2 v1: accept both.
- Should the "no SECURITY.md" verdict be Concerning or Neutral for tiny repos (<5 contributors)? Day 2 v1: always Neutral; weight is small enough that it doesn't dominate.

---

## 10. Closed questions (history)

- 2026-05-04 — invoke OSV by source repo URL via `affected.repo`? — No, not a queryable index in the OSV API. Defer to Day 3 with deps.dev mapping.
- 2026-05-04 — should we treat 5xx Scorecard responses as "no score"? — No; 5xx is a transient failure and the module errors out (CLI exit 7 per architecture §8). Only 404 = "not yet scored".

---

## 11. References

- `docs/methodology.md` §Module 5 — Security & Readiness.
- `docs/module-specs.md` §Security & Readiness.
- `docs/api-notes.md` §scorecard.dev, §OSV.dev.
- ADR-0005 — Federate, don't replicate.
- ADR-0011, ADR-0012.
