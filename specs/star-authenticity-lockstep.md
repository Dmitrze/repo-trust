---
feature: star-authenticity-lockstep
status: accepted
dri: "@Dmitrze"
created: 2026-05-04
updated: 2026-05-04
related_agents: []
related_scenarios: ["tests/scenarios/star-authenticity-lockstep.md"]
related_runbooks: []
related_docs: ["docs/methodology.md#module-1--star-authenticity"]
---

# Star Authenticity — Heuristic 2 (lockstep timing) + Day-3 follow-through

> Day 4 completes the Star Authenticity module: ships Heuristic 2 (lockstep timing z-score), reverts the formula to the methodology weights `0.55 × H1 + 0.30 × H2 + 0.15 × H3`, and lands the two follow-through items the Day-3 architect approved (recency-biased-sample evidence + spec §9 caveat).
>
> Critical posture continues: probabilistic phrasing only — `"fake"` / `"fraud"` / `"bot"` forbidden in any new code. Verdict ceiling stays `Concerning` even when H1+H2 combine.

---

## 1. Goal

`StarsModule::run(ctx)` returns Heuristic 2 sub-score `lockstep_z_score` alongside the existing H1 + H3 sub-scores, computed from the daily star-event time series. Final formula reverts to methodology v1.0 weights. The module also emits a new Neutral `recency_biased_sample` evidence item on every non-below-floor run, and `specs/star-authenticity-module-shallow.md` §9 is amended with the same caveat.

We know it works when: a fixture with a clearly bursty star pattern (50 stars on day 1, 1 star/day for 20 days) yields a `lockstep_z_score ≥ 8`; a smooth distribution yields `lockstep_z_score < 3`. The combined H1 ≥ 20% AND H2 ≥ 5 condition (from methodology §Heuristic 2 caveats) is observable in evidence rationale but does NOT push verdict to `HighRisk` per `CLAUDE.md` §14.

---

## 2. Non-functional requirements

- **API budget:** 0 additional GitHub calls — the daily series is derived from the existing `list_stargazers` response (already requested with `vnd.github.star+json` for `starred_at`). For repos where the existing 200/2000 sample doesn't cover the full lifetime, the z-score is computed over the **observed window** with a `limited_window` evidence caveat.
- **Determinism:** rolling-window arithmetic uses sorted-by-date input + integer-bucket day counts; same inputs → same `lockstep_z_score` byte-for-byte.
- **Conservative posture:** verdict ceiling remains `Concerning`, even when H1+H2 combine. Methodology says "we require both signals before lowering the module score to the 'Concerning' band" — that's an additive depression on the FINAL score via the weighted formula, not a verdict escalation.

---

## 3. Boundaries

### In scope (Day 4)
- New `src/features/stars.rs::lockstep_z_score(starred_dates) -> Option<f64>`:
  - Build daily count series from the `Vec<OffsetDateTime>` starred-at values in the existing sample.
  - Compute 28-day rolling mean and standard deviation, **lagged by 7 days** (per methodology — the baseline window doesn't include the day being scored).
  - Compute z-score per day = `(count[d] - lagged_mean[d]) / max(lagged_std[d], 1.0)`.
  - Return `Option<f64>` = max z-score observed; `None` when the series is too short (<35 days = 28 baseline + 7 lag) or when the sample carries no `starred_at` timestamps (Plain `StargazerEntry` only).
- Update `src/features/stars.rs::compute()` to populate a new `StarsFeatures.lockstep_z_score: Option<f64>` field.
- Update `src/scoring/stars.rs::score()` to:
  - Add a 5th sub-score `lockstep_timing` mapped from the z-score per `methodology.md` §Heuristic 2 v1 bands: `<3 → 100, 3-5 → 85, 5-8 → 60, 8-12 → 30, >12 → 10`.
  - Final formula reverts to: `final = 0.55 × H1 + 0.30 × H2 + 0.15 × H3` when H2 is present; falls back to Day-3 redistribution `0.55 × H1 + 0.45 × H3` when H2 is `None`.
  - Emit `lockstep_z_score` evidence with verdict from `stars_verdict()` (still capped at `Concerning`).
  - When H1 ≥ 20% AND H2 ≥ 5 (combined-signal condition from methodology), emit a separate `combined_low_activity_and_lockstep` evidence item with a one-line rationale acknowledging the combination — verdict still `Concerning`, NEVER `HighRisk`.
  - **DROP** the `lockstep_deferred_to_day_4` evidence item (Day 3's caveat); it's no longer accurate.
  - **ADD** a `recency_biased_sample` Neutral evidence on every non-below-floor run with the rationale: "Day 3-4 sampling is recency-biased: the most-recent N stargazers are sampled directly. True uniform random sampling over the full stargazer history is deferred to Phase 2 deep mode." (Q1 follow-through.)
- Update `src/scoring/thresholds.rs::StarsThresholds::v1()` with new fields:
  - `lockstep_baseline_window_days: u64` (default 28),
  - `lockstep_baseline_lag_days: u64` (default 7),
  - `lockstep_score_bands: [(f64, u8); 5]` (the 5-tuple table above with 0 included as the open lower bound).
- Update `specs/star-authenticity-module-shallow.md` §9 to add the recency-bias caveat (Q1 follow-through). Keep the spec file alive — don't delete; it's now historical Day-3 record.
- ≥6 unit tests on `lockstep_z_score` (smooth → low z, bursty → high z, short series → None, no-starred-at → None, deterministic over identical inputs).
- ≥4 unit tests on the scorer covering: H2 sub-score bands, formula revert, combined-signal evidence, recency-bias evidence emission.
- Update `tests/scenarios/star-authenticity-module-shallow.md` to mark scenarios touching the deferred-lockstep evidence as superseded.

### Out of scope (Day 4)
- Deep-mode graph signal (co-starring overlap with known campaign clusters) — Phase 2+.
- True uniform-random sampling — Phase 2 deep mode (already deferred per Q1).
- Real-API benchmark calibration of the z-score thresholds — Day 5 PM benchmark sweep.

---

## 4. Probabilistic satisfaction threshold

N/A — heuristic.

---

## 5. Happy-path scenario

1. `StarsModule::run` invoked Standard mode; collector returns 200 stargazers with `starred_at` dates.
2. Features layer extracts the 200 dates, builds a daily count series.
3. `lockstep_z_score` computes the 28-day rolling baseline lagged 7 days; identifies max daily z-score = e.g. 2.4 (smooth distribution).
4. Scorer maps 2.4 → 100 (≤3 band); H2 sub-score 100.
5. Final score = `0.55 × H1 + 0.30 × H2 + 0.15 × H3` per methodology v1.0.
6. Evidence list includes `lockstep_z_score`, `low_activity_stargazer_share`, `fork_to_star_ratio`, `watcher_to_star_ratio`, `recency_biased_sample` (always), and `combined_low_activity_and_lockstep` (only when condition met).

For the suspicious-pattern scenario:
- Bursty fixture: 50 stars on day-N, sparse otherwise → daily z-score on day-N ≈ 8.
- H2 sub-score: 30. H1: 38% → 20. H3: low ratios → 30.
- Final: `0.55×20 + 0.30×30 + 0.15×30 = 11 + 9 + 4.5 = 24.5 → 25`.
- Combined-signal evidence emitted.
- Verdict on each evidence item: `Concerning` at most. Module result confidence drives whether the overall report category lands in "Weak" / "High Risk" — the module itself never bumps to HighRisk standalone.

---

## 6. Architecture sketch

```
[ StarsRawData.sampled_profiles ]
       |
       v   (already contains StargazerEntry::WithDate.starred_at when sample carried dates)
features::stars::compute()
       |
       v
StarsFeatures { ..., lockstep_z_score: Option<f64>, ... }
       |
       v
scoring::stars::score()
       |
       +-- H1 (low-activity share)        — bands from §Heuristic 1
       +-- H2 (lockstep_z_score)          — bands from §Heuristic 2
       +-- H3 (fork + watcher ratios)     — bands from §Heuristic 3
       v
final = 0.55×H1 + 0.30×H2 + 0.15×H3 (or 0.55×H1 + 0.45×H3 when H2 = None)
       |
       v
ModuleResult + evidence (incl. recency_biased_sample + maybe combined_low_activity_and_lockstep)
```

---

## 7. Closed loop

- **Goal metric:** ≥10 unit tests across feature + scorer; aggregate determinism integration test still passes; stars module's `lockstep_z_score` sub-score appears in the all-five-modules end-to-end report.
- **Where it lives:** CI; MemPalace `modules/stars` (when reconnected).
- **Read by:** Reviewer; Day 5 real-API benchmark sweep validates the z-score thresholds against known repos.
- **Improvement path:** Day 5 benchmark calibration may tune the z-score band edges if false-positive rate is high on legit-viral repos (HN front page → z-score ~10 naturally).

---

## 8. Definition of Done

- [ ] Spec status `accepted`.
- [ ] `tests/scenarios/star-authenticity-lockstep.md` lists ≥6 scenarios.
- [ ] `src/features/stars.rs::lockstep_z_score` implemented with the rolling-baseline algorithm.
- [ ] `StarsFeatures.lockstep_z_score: Option<f64>` field added; `compute()` populates it.
- [ ] `src/scoring/stars.rs::score()` adds H2 sub-score, reverts formula to 0.55/0.30/0.15, drops the deferred-caveat evidence, adds `recency_biased_sample` evidence on every non-below-floor run, emits `combined_low_activity_and_lockstep` when condition met.
- [ ] `src/scoring/thresholds.rs::StarsThresholds::v1()` extended with lockstep fields.
- [ ] `specs/star-authenticity-module-shallow.md` §9 amended with the recency-bias caveat (Q1 follow-through).
- [ ] ≥6 features unit tests + ≥4 scorer unit tests.
- [ ] Existing stars unit tests pass after formula change (some may need re-baselined expected scores).
- [ ] Aggregate determinism integration test still passes.
- [ ] Verdict on every evidence item stays at `Concerning` ceiling — test-enforced as before.
- [ ] CHANGELOG entry.
- [ ] All quality gates green.

---

## 9. Open questions

- Day 5 calibration may want to soften the >12 band from 10 to 0 to catch the most extreme cases more clearly. Defer to benchmark output.

---

## 10. Closed questions (history)

- 2026-05-04 — should "limited window" caveat fire when sample doesn't span 35+ days? — Yes, but defer to Day 4 implementation discretion. If the rolling window can't fill, return `None` from `lockstep_z_score` and emit a Neutral `lockstep_window_too_short` caveat-evidence (no z-score sub-score in that case).
- 2026-05-04 — verdict ceiling stays Concerning even with combined H1+H2 evidence? — Yes, per CLAUDE.md §14. The final SCORE drops sharply via the weighted formula; the verdict-ceiling property is what keeps probabilistic posture.

---

## 11. References

- `docs/methodology.md` §Module 1 Heuristic 2.
- `docs/module-specs.md` §Star Authenticity.
- `CLAUDE.md` §14 (Glossary — never use "fake / fraud / bot"; verdict ceiling).
- `specs/star-authenticity-module-shallow.md` (Day-3 historical spec; §9 amended in this commit).
