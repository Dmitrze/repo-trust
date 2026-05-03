---
feature: activity-health-module
status: accepted
dri: "@Dmitrze"
created: 2026-05-03
updated: 2026-05-03
related_agents: []
related_scenarios: ["tests/scenarios/activity-health-module.md"]
related_runbooks: []
related_docs: ["docs/methodology.md#module-2--activity-health", "docs/module-specs.md#activity-health"]
---

# Activity Health module

> First end-to-end module. Collector pulls commits / releases / issues / PRs through the cached GitHub client; features compute the eight signals in `module-specs.md` §Activity; scorer applies thresholds from `default.toml` and emits ≥3 evidence items.

---

## 1. Goal

`ActivityModule::run(ctx)` returns `(ModuleResult, Vec<EvidenceItem>)` with a 0–100 score, a confidence band, ≥3 evidence items, and the sub-scores documented in `methodology.md` §Module 2.

We know it worked when: the integration test against the wiremock fixture for `octocat/Hello-World` returns score 0–10 (correctly recognizing the inactive baseline), and `prometheus/prometheus` returns score 80–100 (correctly recognizing high activity).

---

## 2. Non-functional requirements

- **Quick mode:** ≤5 GitHub API calls (repo metadata + 30d commits only).
- **Standard mode:** ≤30 API calls (windows: 30d/90d/365d commits, 18-month monthly, releases, last-90d issues, last-90d PRs, contributors).
- **Determinism:** same fixture → same `(score, sub_scores, evidence)` byte-for-byte (after `snapshot_at` redaction).
- **Conservative:** missing endpoint (e.g. issues disabled on the repo) → that sub-score is omitted, confidence drops one band.

---

## 3. Boundaries

### In scope (Phase 1, Day 1)
- `src/collectors/activity.rs::collect(ctx, client) -> ActivityRawData`.
- `src/features/activity.rs::compute(raw) -> ActivityFeatures` (struct already exists; this adds the producer).
- `src/modules/activity.rs::ActivityModule::run` wired to the above.
- Threshold constants in `src/scoring/thresholds.rs::ACTIVITY` loaded from `default.toml`.
- Sub-scores per `methodology.md`: days_since_last_commit, commits_last_90d, active_contributors_last_90d, median_issue_response_hours, days_since_last_release.
- Final module score: arithmetic mean of sub-scores (per `methodology.md` §Module 2 final).
- Evidence: ≥3 items always, structured by `EvidenceItem` type.
- "Long-stable utility" down-weight gated behind a config flag, default off in Phase 1, surfaced as caveat when active.

### Out of scope (explicit)
- Archived-repo skip logic (one-line guard in `run()` that bails early with a caveat — not feature work).
- Ecosystem-aware multipliers — defer to Phase 4.
- Variance-of-monthly-commits sub-score weighting — collected but not in the v1.0 scoring weight (logged in evidence only).

---

## 4. Probabilistic satisfaction threshold

N/A.

---

## 5. Happy-path scenario

1. `cli::scan::execute(args)` builds `RepositoryContext`, instantiates registry, picks `ActivityModule`.
2. `ActivityModule::run(&ctx)` called.
3. Collector hits cached GitHub client for: repo metadata, commits (3 windows + 18-month monthly), releases, issues 90d, PRs 90d, contributors.
4. Features computed (8 fields in `ActivityFeatures`).
5. Scorer applies thresholds → 5 sub-scores → arithmetic mean → integer score 0–100.
6. Confidence: `High` if all data present + repo age ≥6 months; `Medium` if data partial; `Low` if <30 days of activity history available.
7. Evidence list: ≥3 items, mix of Positive/Neutral/Concerning verdicts based on sub-score bands.
8. Returned to caller; aggregator includes in overall trust score.

---

## 6. Architecture sketch

```
[ Cached GH Client ] --> ActivityCollector::collect(ctx, client) -> ActivityRawData
                                                  |
                                                  v
                                  ActivityFeatures::compute(raw)
                                                  |
                                                  v
                                  scoring::activity::score(features) -> (ModuleResult, Vec<EvidenceItem>)
                                                  |
                                                  v
                                       ActivityModule::run returns
```

---

## 7. Closed loop

- **Goal metric:** `tests/modules/activity_test.rs` passes ≥10 unit cases + 1 wiremock integration test.
- **Where it lives:** CI; MemPalace `modules/activity`.
- **Read by:** Reviewer; benchmark sweep on Day 5 surfaces real-repo accuracy.
- **Improvement path:** if benchmark shows category mis-classifications, tune `default.toml` thresholds (versioned via SCORING_VERSION bump).

---

## 8. Definition of Done

- [ ] Spec status `accepted`.
- [ ] `tests/scenarios/activity-health-module.md` lists ≥6 scenarios.
- [ ] `src/collectors/activity.rs::collect` implemented.
- [ ] `src/features/activity.rs::compute` implemented.
- [ ] `src/modules/activity.rs::ActivityModule::run` returns real data (no `bail!`).
- [ ] `src/scoring/thresholds.rs` exposes activity threshold table.
- [ ] ≥10 unit tests in `src/scoring/thresholds.rs::tests` covering each band of each sub-score.
- [ ] ≥3 evidence items emitted per `EvidenceItem` type.
- [ ] Wiremock integration test using `tests/fixtures/octocat-Hello-World/` minimal set.
- [ ] CHANGELOG entry.
- [ ] All quality gates green.

---

## 9. Open questions

- None.

---

## 10. Closed questions (history)

- 2026-05-03 — should variance-of-monthly-commits be in v1 score? — No, collected but evidence-only; weighting is empirically uncalibrated.

---

## 11. References

- `docs/methodology.md` §Module 2.
- `docs/module-specs.md` §Activity Health.
