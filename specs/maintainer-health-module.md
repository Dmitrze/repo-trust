---
feature: maintainer-health-module
status: accepted
dri: "@Dmitrze"
created: 2026-05-04
updated: 2026-05-04
related_agents: []
related_scenarios: ["tests/scenarios/maintainer-health-module.md"]
related_runbooks: []
related_docs: ["docs/methodology.md#module-3--maintainer-health", "docs/module-specs.md#maintainer-health"]
---

# Maintainer Health module

> Second end-to-end module. Pulls 365 days of commits + PR-review activity from the cached GitHub client, computes Gini coefficients on author and reviewer concentration, derives a bus-factor proxy and contributor retention rate, and emits ≥3 evidence items. Solo-maintainer projects are flagged in evidence — never penalised into "High Risk" alone (per `module-specs.md`).

---

## 1. Goal

`MaintainersModule::run(ctx)` returns `(ModuleResult, Vec<EvidenceItem>)` answering "is stewardship sustainable?" for the given repo. We know it worked when: prometheus/prometheus scores ≥80 (multi-maintainer + active reviewers); a solo-maintainer project scores 30-50 with the solo caveat surfaced as Concerning evidence (not HighRisk).

---

## 2. Non-functional requirements

- **API budget:** ≤30 calls in Standard mode (commits 365d windowed, contributors, presence checks for CODEOWNERS/MAINTAINERS.md/GOVERNANCE.md). Reuses the 18-month commit window from Activity if same scan via cache hits.
- **Determinism:** Gini computation uses sorted-key iteration; same author counts → same Gini to 6 decimals.
- **Bot filter:** GitHub `type == "Bot"` plus username regex (`*-bot$`, `dependabot[bot]`, `renovate[bot]`, `github-actions[bot]`) — applied at the features layer.

---

## 3. Boundaries

### In scope (Day 2)
- `src/collectors/maintainers.rs::collect` — fetches 365d commits, contributors, content presence (CODEOWNERS at 4 conventional paths, MAINTAINERS.md, GOVERNANCE.md).
- `src/features/maintainers.rs::compute` — bot filter, commits-by-author / reviewers-by-user maps, Gini, bus-factor proxy, retention rate (180d windows).
- `src/scoring/maintainers.rs::score` — sub-scores from `methodology.md` §Module 3 thresholds: bus-factor proxy, Gini bands, retention bands, presence of governance docs.
- `src/modules/maintainers.rs::run` — wires the three stages.
- proptest invariants on Gini (bounded `[0, 1]`, equal contributions ⇒ 0).
- ≥10 unit tests on the scorer + 1 wiremock integration test.
- Solo-maintainer caveat: evidence Concerning verdict, but module score is computed from real signals (not forced to 0). Per `module-specs.md`: "Solo-maintainer projects: flagged in evidence; not penalized into 'High Risk' alone. Many excellent OSS projects are solo-maintained."

### Out of scope (Day 2)
- PR-review concentration Gini using the **Reviews** API. The Reviews endpoint requires per-PR follow-up calls (pulls/{N}/reviews). For Day 2 we approximate "review activity" via PR `merged_by` + comment counts on the PRs already collected by the Activity collector; richer review concentration is a v1.1 follow-up.
- Maintainer responsiveness — separate spec, deferred to v1.1.
- Multi-organization governance signal (ecosyste.ms-style).

---

## 4. Probabilistic satisfaction threshold

N/A.

---

## 5. Happy-path scenario

1. `cli::scan::execute` builds `RepositoryContext`, picks `MaintainersModule`.
2. Collector pulls last-365d commits (cached if Activity already fetched 18m); contributors summary; presence check on CODEOWNERS / MAINTAINERS.md / GOVERNANCE.md via `Client::file_exists`.
3. Features apply bot filter, compute commits-by-author map, derive: `active_maintainers_last_year`, `commit_gini`, `bus_factor_proxy`, `contributor_retention_rate`, `has_codeowners`, `has_maintainers_md`, `has_governance_doc`.
4. Scorer maps to sub-scores per methodology table; final score = arithmetic mean.
5. Confidence: High when commit volume ≥30 and contributors endpoint succeeded; Medium when commit volume <30; Low when archived or repo <6mo old.
6. Evidence: ≥3 items including `bus_factor_proxy`, `commit_gini`, `contributor_retention`, plus presence-of-governance items where relevant.

---

## 6. Architecture sketch

```
[ Cached GH Client ] --> MaintainersCollector::collect → MaintainersRawData
                                                  |
                                                  v
                                  MaintainersFeatures::compute → MaintainersFeatures
                                  (bot filter, Gini, bus-factor proxy, retention)
                                                  |
                                                  v
                                  scoring::maintainers::score → (ModuleResult, Vec<EvidenceItem>)
                                                  |
                                                  v
                                       MaintainersModule::run returns
```

---

## 7. Closed loop

- **Goal metric:** ≥10 scorer unit tests + 1 wiremock integration test green; proptest invariants on Gini hold.
- **Where it lives:** CI; MemPalace `modules/maintainers`.
- **Read by:** Reviewer; Day 5 prometheus + rust-lang/cargo benchmark sweep.
- **Improvement path:** if benchmark mis-classifies multi-maintainer projects, tune the bus-factor proxy thresholds in v1.1.

---

## 8. Definition of Done

- [ ] Spec status `accepted`.
- [ ] `tests/scenarios/maintainer-health-module.md` lists ≥6 scenarios.
- [ ] `src/collectors/maintainers.rs::collect` implemented.
- [ ] `src/features/maintainers.rs::compute` implemented with bot filter.
- [ ] `src/modules/maintainers.rs::run` returns real data (no `bail!`).
- [ ] `src/scoring/maintainers.rs::score` exposes sub-scores: `bus_factor_proxy`, `commit_concentration` (Gini), `contributor_retention`, `governance_docs`.
- [ ] proptest invariants on Gini (bounded, equal-contrib ⇒ 0).
- [ ] ≥10 unit tests on scorer + ≥3 unit tests on features (bot filter, Gini math, retention).
- [ ] 1 wiremock integration test (multi-maintainer fixture).
- [ ] Solo-maintainer evidence verdict is `Concerning`, never `HighRisk` standalone.
- [ ] CHANGELOG entry.
- [ ] All quality gates green.

---

## 9. Open questions

- Should "active maintainer" require a commit OR a PR review OR an issue triage? Day 2 v1: commit only (commits 365d). Refine if benchmark surfaces under-counting.

---

## 10. Closed questions (history)

- 2026-05-04 — true PR-review concentration via /pulls/{N}/reviews per PR? — Deferred to v1.1; Day 2 uses `merged_by` + comment counts as proxy.

---

## 11. References

- `docs/methodology.md` §Module 3 — Maintainer Health.
- `docs/module-specs.md` §Maintainer Health.
- ADR-0011 — TrustModule trait shape.
- ADR-0012 — RepositoryContext runtime handles.
