---
feature: star-authenticity-module-shallow
status: accepted
dri: "@Dmitrze"
created: 2026-05-05
updated: 2026-05-05
related_agents: []
related_scenarios: ["tests/scenarios/star-authenticity-module-shallow.md"]
related_runbooks: []
related_docs: ["docs/methodology.md#module-1--star-authenticity", "docs/module-specs.md#star-authenticity"]
---

# Star Authenticity module — shallow (Day 3 cut)

> Fifth and final module. Day 3 ships **two of three** heuristics from `methodology.md` §Module 1: low-activity profile share (Heuristic 1) and fork/watcher ratios with ecosystem multipliers (Heuristic 3). Lockstep timing (Heuristic 2) is deferred to Day 4.
>
> Critical posture: the module **never** uses the words "fake", "fraud", or "bot" in evidence rationale (per `CLAUDE.md` §14 glossary). All language is probabilistic — "X% of sampled stargazers match a low-activity profile".

---

## 1. Goal

`StarsModule::run(ctx)` returns `(ModuleResult, Vec<EvidenceItem>)` answering "are popularity signals organic?" using two heuristics. Day 3 module formula: `0.55 × low_activity_subscore + 0.45 × ratio_subscore`. The 0.30 weight from the deferred Heuristic 2 is redistributed to ratios (was 0.15) until lockstep ships Day 4 (when weights revert to 0.55 / 0.30 / 0.15 per methodology).

We know it works when: a healthy repo (low low-activity-share + healthy ratios) scores ≥80 with High confidence; a low-quality popularity profile (high low-activity-share + thin ratios) scores ≤30 with at least one `Concerning` evidence item.

---

## 2. Non-functional requirements

- **Sample size:** 200 (Standard) / 0 (Quick — module skipped) per methodology and `default.toml`. Sample <100 → `Medium` confidence; <30 → `Low` confidence + caveat.
- **Determinism:** sampling uses `ChaCha20Rng::seed_from_u64(seed)` where seed defaults to `blake3(repo, scoring_version)` per ADR-0007. Already implemented in `utils::sampling::derive_seed + sample`.
- **Per-module rate budget:** 1 (stargazers list) + N (per-stargazer profile) ≤ 200 calls in Standard.
- **Conservative:** new repos (<6 months) get a 5pp leniency on the low-activity-share threshold.
- **Probabilistic language:** evidence rationale is `"X% of sampled stargazers match a low-activity profile"` — never `"this repo has fake stars"`. Verdict for the heuristic is `Concerning` at most, never `HighRisk` standalone.

---

## 3. Boundaries

### In scope (Day 3 — shallow)
- `src/collectors/stars.rs::collect` — fetch repo metadata (already cached if Activity ran first), stargazers (one page per `stars.sample_size_standard`), per-stargazer profile via new `github::Client::get_user(login)`.
- New `github::Client::get_user(login) -> Result<UserProfile>` method over `GET /users/{login}`. Cache key `github:users:{login}`, 24h TTL. UserProfile DTO carries the 9 signals + `created_at`.
- `src/features/stars.rs::compute` — implements the **9-signal composite** per `methodology.md` §Module 1:
  - `created_at > 2022-01-01` AND
  - `followers ≤ 1` AND `following ≤ 1` AND
  - `public_gists == 0` AND `public_repos ≤ 4` AND
  - `bio` empty AND `blog` empty AND `email` empty AND
  - `starred_at == created_at` (same UTC day) — only when `starred_at` available via the `vnd.github.star+json` Accept header (already implemented in `list_stargazers`).
- `src/scoring/stars.rs::score` — Heuristic 1 sub-score per the 6-band table from methodology (5/10/20/35/50% thresholds → 100/85/65/40/20/0). Heuristic 3 sub-score from fork/star + watcher/star ratios with ecosystem multipliers from `module-specs.md` §Star Authenticity. Final score = `0.55 × H1 + 0.45 × H3`.
- `src/modules/stars.rs::run` — wire collector → features → scorer.
- 5pp leniency on the low-activity-share threshold for repos with `created_at` younger than 6 months: e.g. the 20% concern-band shifts to 25% for these repos. Surfaced in evidence rationale.
- Skip module entirely (no result) when `total_stars < 50` per `module-specs.md` Edge cases — the sample is too small to be meaningful. Replace with a Neutral `stars_too_few_to_sample` caveat-evidence at the report level (the module returns score 0 + `missing_data: ["below_sampling_floor"]` + `Low` confidence; the orchestrator decides whether to drop it from the aggregate weight).
- Quick mode skips this module entirely (sample_size_quick = 0).

### Out of scope (Day 3, lands Day 4)
- **Heuristic 2: Lockstep timing z-score.** Daily star-count series + 28-day rolling mean/std + max daily z-score. Deferred Day 4; today's formula re-allocates the 0.30 weight to ratios (45% instead of 15%).
- Deep-mode graph signal (co-starring overlap with known campaign clusters) — Phase 2+.

---

## 4. Probabilistic satisfaction threshold

N/A — heuristic, not LLM.

---

## 5. Happy-path scenario

1. `cli::scan::execute` builds context, picks `StarsModule` (Standard mode, sample_size_standard=200).
2. Collector calls `github::Client::list_stargazers(owner, repo, max=200)` with `vnd.github.star+json` Accept header.
3. Collector seeds `ChaCha20Rng` from `blake3(full_name, scoring_version)`, samples up to 200 entries via `utils::sampling::sample`.
4. Collector calls `github::Client::get_user(login)` per sampled stargazer (concurrent via `try_join_all` with the rate-limit semaphore).
5. Features layer applies the 9-signal composite per stargazer, counts matches, derives `low_activity_share` (matches / sample_size).
6. Features layer pulls `forks_count` / `watchers_count` / `stargazers_count` from repo metadata, applies ecosystem multiplier from primary language, derives `fork_ratio_score` and `watcher_ratio_score` against `methodology.md` healthy-range thresholds.
7. Scorer maps each heuristic to sub-score per methodology bands; final = `0.55 × H1 + 0.45 × H3`.
8. Confidence: `High` if sample ≥ 100 + repo ≥ 6mo old; `Medium` if 30 ≤ sample < 100 OR repo < 6mo; `Low` if sample < 30.
9. Evidence: ≥3 items — `low_activity_stargazer_share`, `fork_to_star_ratio`, `watcher_to_star_ratio`, `sample_size`, plus a `lockstep_deferred_to_day_4` Neutral caveat-evidence so reports are explicit about what's missing.

For the `total_stars < 50` scenario:
- Module emits `ModuleResult { score: 0, confidence: Low, missing_data: ["below_sampling_floor"] }` plus a Neutral evidence item explaining the sample floor.
- Aggregator gives this 0 effective weight via the existing low-confidence × weight formula.

---

## 6. Architecture sketch

```
[ Cached GH Client ] -----► get_repo + list_stargazers (vnd.github.star+json) + get_user × N
                                                  |
                          utils::sampling::sample (ChaCha20Rng, seeded from blake3)
                                                  |
                                                  v
                                  StarsCollector::collect → StarsRawData
                                                  |
                                                  v
                                  StarsFeatures::compute (9-signal composite + ecosystem-adjusted ratios)
                                                  |
                                                  v
                                  scoring::stars::score (0.55 × H1 + 0.45 × H3 — Day 3 weights)
                                                  |
                                                  v
                                       StarsModule::run returns
```

---

## 7. Closed loop

- **Goal metric:** ≥10 scorer unit tests + 1 wiremock integration test (small fixture: 5 stargazers, mixed profiles).
- **Where it lives:** CI; MemPalace `modules/stars`.
- **Read by:** Reviewer; Day 4 lockstep work re-tightens the formula; Day 5 real-API validation against benchmark set.
- **Improvement path:** Day 4 adds Heuristic 2 → final formula reverts to 0.55 / 0.30 / 0.15 per `methodology.md`.

---

## 8. Definition of Done

- [ ] Spec status `accepted`.
- [ ] `tests/scenarios/star-authenticity-module-shallow.md` lists ≥6 scenarios.
- [ ] `src/collectors/stars.rs::collect` implemented; uses `utils::sampling`.
- [ ] `src/features/stars.rs` extended with `compute()` returning `StarsFeatures`.
- [ ] `src/scoring/stars.rs::score` implements Heuristic 1 + Heuristic 3 + Day-3 weighted formula + 5pp leniency for new repos.
- [ ] `src/scoring/thresholds.rs` exposes `StarsThresholds::v1()` (the 6 low-activity bands + ratio cutoffs + ecosystem multiplier table).
- [ ] `src/modules/stars.rs::run` returns real data (no `bail!`).
- [ ] New `github::Client::get_user(login)` + `UserProfile` DTO.
- [ ] ≥10 unit tests on scorer (band edges, ecosystem multipliers, 5pp leniency, sample-size confidence demotion).
- [ ] 1 wiremock integration test (small mixed-profile fixture).
- [ ] Evidence rationale uses **only probabilistic language** ("X% of sampled stargazers match a low-activity profile"); the words "fake", "fraud", "bot" do not appear in any new code under `src/scoring/stars.rs` or `src/features/stars.rs`.
- [ ] CHANGELOG entry.
- [ ] All quality gates green.

---

## 9. Open questions

- Should the 5pp leniency apply to ratio thresholds too? Day 3 v1: no — ratios are repo-level and should not depend on age. Tune if benchmark surfaces young-repo false positives.

### Caveat — recency-biased sample (Day-4 architect Q1 follow-through, 2026-05-04)

The Day-3 collector fetches the most-recent N stargazers via
`github::Client::list_stargazers(max=N)` and treats them as the sample
without further sub-sampling. This is **recency-biased**: for a 1000-star
repo asking for 200, we get the 200 most-recent stargazers (skewed toward
fresh accounts). The seeded `ChaCha20Rng` from `utils::sampling` is
therefore unused in the Day-3 / Day-4 Stars collector.

The methodology calls for "uniform random over GitHub's stargazer
pagination" — true uniform sampling is **deferred to Phase 2 deep mode**
(when the deep-mode budget allows fetching the full population, then
sampling N from it). Until then, the Stars module emits a Neutral
`recency_biased_sample` evidence item on every non-below-floor run so
reports are explicit about the sampling bias.

This caveat is co-located here for historical Day-3 record; see
`specs/star-authenticity-lockstep.md` §3 for the Day-4 evidence wiring.

---

## 10. Closed questions (history)

- 2026-05-05 — should the deferred Heuristic 2 be partially implemented? — No, full lockstep work lands Day 4 in one branch; partial today would be wasted work.
- 2026-05-05 — verdict ceiling for Heuristic 1? — `Concerning` is the maximum verdict. The module **never** emits `HighRisk` for the low-activity heuristic alone; methodology requires combined H1+H2 evidence to lower category to "Concerning band", which can only happen Day 4+.

---

## 11. References

- `docs/methodology.md` §Module 1 — Star Authenticity (Heuristic 1, Heuristic 3, citations).
- `docs/module-specs.md` §Star Authenticity (ecosystem multipliers, edge cases).
- `CLAUDE.md` §14 (Glossary — never use "fake / fraud / bot").
- ADR-0011, ADR-0012.
- `utils::sampling` (deterministic sampling helpers — already shipped Day 0).
