# Repo Trust — Public Methodology

> **Version:** 1.0 (May 2026). Versioned with the scoring model. See [`scoring-model.md`](scoring-model.md) for the change log.
>
> **License:** This document is dual-licensed under Apache-2.0 (the project license) and [CC-BY-4.0](https://creativecommons.org/licenses/by/4.0/) so it can be cited and adapted in academic and industry work.

---

## Why publish a methodology?

A score that you can't audit is a score you can't trust. Repo Trust takes the opposite stance from closed-source SaaS competitors: every threshold, every weight, every heuristic is documented here, in plain English, with citations.

If you disagree with a number, that disagreement is welcome — open a [methodology question issue](https://github.com/Dmitrze/repo-trust/issues/new?template=methodology_question.yml) and we'll engage with the substance.

---

## Guiding principles

1. **Heuristic, not black-box.** No ML classifiers in v1 (see [ADR-0004](adr/0004-no-ml-in-v1.md)). Every score is a function of named features against named thresholds.
2. **Conservative on negative claims.** A false positive in fake-star detection harms a real maintainer. We bias toward saying "low confidence" rather than "suspicious."
3. **Federate, don't replicate.** When OpenSSF Scorecard, deps.dev, or OSV already answer a question well, we consume their output (see [ADR-0005](adr/0005-federate-not-replicate.md)).
4. **Confidence is reported separately from score** (see [ADR-0008](adr/0008-confidence-separate-from-score.md)).
5. **Probabilistic language only.** We say "X% of sampled stargazers match a low-activity profile" — never "this repo has fake stars."

---

## Module 1 — Star Authenticity

### The question
*Are the popularity signals organic, or do they show patterns consistent with paid star campaigns?*

### Inputs

For a sample of stargazers (size depends on mode — see §6):
- Account creation date
- Followers / following counts
- Public repo count
- Public gist count
- Bio, blog URL, email presence
- Star date (when available via GraphQL `starredAt` field)
- For the lockstep signal: full timestamp series of star events

For the repo itself:
- Total stars, forks, watchers
- Stargazer-time-series shape

### Heuristic 1: Low-activity profile share

A stargazer account is flagged as "low-activity" if **all** of the following are true:

| Signal | Threshold |
| --- | --- |
| `created_at` | After 2022-01-01 |
| `followers` | ≤ 1 |
| `following` | ≤ 1 |
| `public_gists` | == 0 |
| `public_repos` | ≤ 4 |
| `bio` | empty |
| `blog` | empty |
| `email` | empty |
| `starred_at` | == `created_at` (same day, when available) |

**Rationale:** This composite signature is from Dagster's [fake-star-detector (2023)](https://dagster.io/blog/fake-stars) and was further validated by StarScout (He et al., ICSE 2026, [arxiv 2412.13459](https://arxiv.org/pdf/2412.13459)). Each individual signal is weak, but their conjunction is statistically anomalous — real users almost always have at least one non-default field.

**Score thresholds (v1.0):**

| Low-activity share | Sub-score |
| --- | --- |
| < 5% | 100 |
| 5–10% | 85 |
| 10–20% | 65 |
| 20–35% | 40 |
| 35–50% | 20 |
| > 50% | 0 |

**Important caveats:**
- Below 100 sample size, confidence drops to Medium.
- Below 30 sample size, confidence drops to Low and we surface a caveat.
- New repos (< 6 months old) often have a higher share of new accounts naturally; we apply a 5pp leniency to repos under 6 months.

### Heuristic 2: Lockstep timing

**Idea:** Real stars arrive in a relatively smooth distribution shaped by traffic events (HN, Twitter, conference talks). Paid stars often arrive in tight bursts.

**Method (v1.0):**
1. Build a daily count series of stars over the repo's lifetime.
2. Compute the rolling 28-day mean and standard deviation, lagged by 7 days.
3. Compute the z-score of each day's star count vs the lagged baseline.
4. Identify "burst" days where z-score ≥ 5.
5. Compute `lockstep_z_score` = max z-score observed.

**Score thresholds (v1.0):**

| Max daily z-score | Sub-score |
| --- | --- |
| < 3 | 100 |
| 3–5 | 85 |
| 5–8 | 60 |
| 8–12 | 30 |
| > 12 | 10 |

**Caveats:**
- A genuine viral moment (HN front page) can produce z-scores in the 8–15 range. We are explicit that high z-scores are not proof of campaigns, only a signal that *combined with* the low-activity share suggests a campaign.
- We require **both** signals (low-activity share ≥ 20% AND z-score ≥ 5) before lowering the module score to the "Concerning" band.

### Heuristic 3: Fork / watcher ratios

**Idea:** Healthy projects have stars correlated with forks (people who clone the code) and watchers (people who track it). Star-only popularity is suspicious.

| Signal | Healthy range |
| --- | --- |
| `forks / stars` | ≥ 0.04 |
| `watchers / stars` | ≥ 0.005 |

These ratios shift by ecosystem (TypeScript libraries fork less than Python frameworks). We apply ecosystem-aware multipliers documented in `module-specs.md`.

### Final module score

`module_score = 0.55 × low_activity_subscore + 0.30 × lockstep_subscore + 0.15 × ratio_subscore`

This weighting reflects the rigor of each signal: low-activity profile share is the strongest, lockstep is corroborating, ratios are rough.

---

## Module 2 — Activity Health

### The question
*Is the repository alive and operationally active?*

### Inputs
- Commits in last 30 / 90 / 365 days
- Days since last commit
- Days since last release (if any)
- Median issue first-response time (last 90 days)
- Median PR review time (last 90 days)
- Active contributors in last 90 days
- Variance of monthly commit count over 18 months (continuity)

### Sub-scores

| Sub-signal | Threshold for full credit | Threshold for zero |
| --- | --- | --- |
| Days since last commit | ≤ 14 | ≥ 365 |
| Commits last 90d | ≥ 30 | == 0 |
| Active contributors last 90d | ≥ 4 | ≤ 1 |
| Median issue response (hours) | ≤ 48 | ≥ 720 (30d) |
| Days since last release | ≤ 90 | ≥ 730 (2y) |

Each sub-score is interpolated linearly between the two thresholds.

**Final:** `module_score = mean of sub-scores`.

### Ecosystem awareness

Long-stable utilities (e.g. a UUID library) legitimately go quiet. We mitigate by:
- Down-weighting "days since last release" if `total_releases >= 3` AND `versioned_consistently == true`.
- Down-weighting "commits last 90d" if `dependents_count >= 1000` (a heavily-depended-on stable library).

These adjustments are documented per ecosystem in `module-specs.md`.

---

## Module 3 — Maintainer Health

### The question
*Is stewardship sustainable, or does this project sit on one person's shoulders?*

### Inputs
- Active maintainers in last 365 days (any commit)
- Gini coefficient of commits-per-author (last 365 days)
- Gini coefficient of PR-review actions (last 365 days)
- Bus factor proxy: minimum number of authors covering 50% of last 365d commits
- Contributor retention rate: % of contributors active in two consecutive 180-day windows
- Median maintainer response time (issues + PRs)
- Presence of `CODEOWNERS`, `MAINTAINERS.md`, governance docs

### Why Gini?

A project where one author makes 95% of commits has Gini ≈ 0.95; a healthy project where five maintainers each contribute 20% has Gini ≈ 0. Gini captures concentration in a single number that is comparable across project sizes.

### Bus factor proxy

We explicitly avoid calling this "bus factor" because true bus factor is unmeasurable from outside. Our proxy: how few people would need to leave to drop below 50% of recent commits? It correlates with bus factor but isn't identical.

### Score table (illustrative)

| Bus-factor proxy | Sub-score |
| --- | --- |
| ≥ 5 | 100 |
| 4 | 85 |
| 3 | 70 |
| 2 | 50 |
| 1 | 25 |

Full tables in `scoring-model.md`.

---

## Module 4 — Adoption Signals

### The question
*Is this repository actually used in the wild?*

### Inputs (federated where possible)
- GitHub `dependents` count and trend (last 90 days)
- Package-registry weekly downloads (npm, PyPI, crates.io, RubyGems, Maven Central, NuGet) via [deps.dev](https://deps.dev)
- Mentions in well-known awesome-lists (configurable list maintained in `examples/awesome-lists.txt`)
- Documentation maturity: presence and length of `README.md`, `docs/` folder, `examples/` folder

### Federation

We do not collect download statistics ourselves — deps.dev does this at industrial scale and we'd be replicating their work badly. We pull `weekly_downloads` for whichever package system the repo publishes to and report it as one signal in the module.

### Graceful degradation

Not every repo publishes a package. A research project on GitHub may have 10,000 stars and zero packages. We don't penalize this — we simply lower the module's confidence to Medium and surface the absence as a caveat, not a concern.

---

## Module 5 — Security & Readiness

### The question
*Is this repository in a state that supports responsible adoption?*

### Inputs
- OpenSSF Scorecard score (federated from `api.scorecard.dev`)
- OSV vulnerability count for the repo's published packages (federated from `api.osv.dev`)
- Presence and recency of: `SECURITY.md`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `LICENSE`, `CODEOWNERS`
- CI workflow files in `.github/workflows/`
- Release-tagging consistency (semver adherence)
- Branch-protection signals (where observable via the API)

### Federation: Scorecard

If Scorecard has a recent score for the repo (≤ 30 days old), we use it directly with weight 0.40 of this module's score. We do **not** re-run Scorecard's 18 checks ourselves.

If Scorecard has no score, we report `Low confidence` for this module and rely on the document-presence and CI sub-signals.

### Federation: OSV

We map `repo → published packages` via deps.dev, then query OSV for each package's published versions. The count of unfixed advisories in the latest published version becomes a sub-signal.

---

## Aggregation

The overall Trust Score is a confidence-weighted average:

```
trust_score = Σ (w_i × score_i × conf_i) / Σ (w_i × conf_i)
```

where `conf_i` is `0.5` for Low, `0.75` for Medium, `1.0` for High. This means a partially-collected module contributes less to the final number, preventing thin-data modules from dominating.

Default weights (v1.0):

| Module | Weight |
| --- | --- |
| Star Authenticity | 0.20 |
| Activity Health | 0.25 |
| Maintainer Health | 0.20 |
| Adoption Signals | 0.20 |
| Security & Readiness | 0.15 |

Users can override weights via `--weights weights.toml`.

## Confidence aggregation

The report's overall confidence is the **minimum** confidence across modules whose weight contributes more than 10% of the total. This means one low-confidence module on a meaningful axis lowers the headline confidence — we don't average it away.

## Determinism

See [ADR-0007](adr/0007-deterministic-output.md). Same inputs (`repo`, `mode`, `scoring_version`, `weights`, `rng_seed`) + same upstream API state = byte-identical JSON output (modulo `snapshot_at` and `runtime_seconds`).

Default RNG seed is derived from `(repo, scoring_version)` via blake3 hash, so users get the same sample for the same repo across runs without specifying `--seed`.

## Citations

- He, H. et al. **"4.5 Million (Suspected) Fake Stars in GitHub: A Growing Spiral of Popularity Contests, Scams, and Malware."** *ICSE 2026*. [arxiv:2412.13459](https://arxiv.org/pdf/2412.13459).
- Dagster Labs. **"Fake stars on GitHub."** Blog post, March 2023. <https://dagster.io/blog/fake-stars>
- Open Source Security Foundation. **OpenSSF Scorecard.** <https://scorecard.dev>
- Google Open Source Insights. **deps.dev.** <https://deps.dev>
- Open Source Vulnerabilities. **OSV.dev.** <https://osv.dev>
