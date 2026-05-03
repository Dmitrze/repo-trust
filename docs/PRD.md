# Product Requirements Document — Repo Trust

> **Version:** 1.0 (initial draft, May 2026)
> **Status:** Pre-build, Phase 0 (Research Foundation)
> **Owner:** @Dmitrze
> **License plan:** Apache-2.0 (project), CC-BY-4.0 (methodology docs)

---

## 1. Executive Summary

GitHub stars are the de facto popularity signal for open-source repositories, but they are an incomplete and increasingly distorted proxy for project quality. A 2024–2026 academic study (StarScout / ICSE 2026) identified approximately **6 million suspected fake stars across 18,617 repositories**, with fake-star activity surging in 2024 to the point where roughly **16% of repositories with 50+ stars showed fake-star campaign signals**. At the same time, real signals of project trustworthiness — maintainer concentration, release hygiene, downstream adoption, security posture — remain scattered across separate tools (OpenSSF Scorecard, deps.dev, Snyk Advisor, Socket.dev, libraries.io, ecosyste.ms).

**Repo Trust** is a developer-first, open-source command-line tool that produces a single, explainable, multi-dimensional **Trust Report** for any public GitHub repository. It is not a security scanner, not a star-fraud novelty detector, and not a SaaS dashboard. It is the missing **diligence layer** for engineers, analysts, scouts, and maintainers who need to answer one question quickly and defensibly:

> *Can this repository be trusted — for evaluation, for adoption as a dependency, for investment, or for inclusion in a curated list?*

The product calculates an overall Trust Score (0–100) and exposes five module-level scores with full evidence, confidence bands, and caveats. It runs locally, caches aggressively, respects API rate limits, produces machine-readable outputs (JSON, Markdown, CSV, SARIF), and is intentionally conservative when data is partial.

---

## 2. Problem Statement

### 2.1 The four problems we observe

1. **Popularity is gameable.** Fake-star marketplaces sell stars at $0.06–0.50 each; campaigns succeed in pushing repositories onto GitHub Trending. Discovery surfaces and VC-style "GitHub-as-traction" heuristics are systematically polluted.
2. **Evaluation is fragmented.** A serious evaluator currently consults at minimum: GitHub UI (commits, issues, releases, contributors), OpenSSF Scorecard (security), deps.dev or libraries.io (dependents), npm/PyPI download stats, OSV (vulnerabilities), and either Snyk Advisor or Socket.dev. There is no unified, scriptable, free, self-hostable view.
3. **Existing tools are point solutions.** OpenSSF Scorecard is excellent but security-only. Snyk Advisor and Socket.dev are SaaS-gated. deps.dev is API-only with no opinionated scoring or report. StarScout is a research artifact requiring BigQuery. Dagster's `fake-star-detector` is narrow and BigQuery-bound.
4. **Diligence is not reproducible.** Two evaluators looking at the same repo on different days, using ad-hoc heuristics, will reach different conclusions. There is no versioned scoring model that produces comparable outputs over time.

### 2.2 Who has this problem

| User segment | Current pain | What they need |
| --- | --- | --- |
| Application developers picking a dependency | Manual triage across 5+ tools, slow | One CLI invocation that gives "should I trust this?" |
| OSS maintainers benchmarking themselves or peers | Vanity metrics dominate, real strengths invisible | Module breakdown showing where they're strong/weak |
| Analysts at funds, accelerators, scouts | Need repeatable diligence at scale | Batch mode with CSV/JSON output for spreadsheets and CRMs |
| Security and platform engineers | Need supply-chain risk signal beyond CVEs | Trust signal that combines OSSF Scorecard + activity + adoption |
| Researchers and ecosystem curators | Need reproducible, versioned metrics | Scoring version pinning, snapshot mode, public methodology |
| Tech journalists, OSS directory builders | Need defensible, citable claims | Evidence-backed reports with caveats |

### 2.3 Out of scope

We are explicitly **not** building:

- A vulnerability scanner (use OSV, Trivy, Snyk, Socket).
- A code-quality static analyzer (use SonarQube, DeepSource, CodeQL).
- A license compliance tool (use FOSSA, ScanCode).
- A naming-and-shaming or fraud-adjudication system. Our language is probabilistic ("suspicious pattern", "weak readiness"), never definitive ("fraud").
- A SaaS dashboard. The CLI and local web viewer are the entire product surface in v1.

---

## 3. Competitive Landscape

| Tool | Type | Coverage | Open methodology | CLI-first | Free for individuals | Multi-dimensional trust |
| --- | --- | --- | --- | --- | --- | --- |
| **Repo Trust** *(this project)* | OSS CLI | GitHub public repos + ecosystem signals | ✅ Full | ✅ Yes | ✅ Yes | ✅ 5 modules |
| OpenSSF Scorecard | OSS CLI / GH Action | Security health (18 checks) | ✅ Full | ✅ Yes | ✅ Yes | ❌ Security only |
| Snyk Advisor | SaaS | Package + repo | ⚠️ Partial | ❌ Web-first | ⚠️ Limited tier | ⚠️ Health + popularity |
| deps.dev | Free API + Web | Packages, vulns, Scorecard | ✅ Yes | ❌ API-only | ✅ Yes | ❌ Aggregator, no opinion |
| Socket.dev | SaaS | npm/PyPI behavioral | ⚠️ Partial | ⚠️ CLI exists | ⚠️ Limited tier | ❌ Supply-chain only |
| StarScout (research) | Academic | GHArchive scale | ✅ Yes (paper) | ❌ Pipeline | ✅ Yes | ❌ Stars only |
| Dagster fake-star-detector | OSS | Single repo, BQ-bound | ✅ Yes | ⚠️ Dagster-bound | ⚠️ BQ free tier | ❌ Stars only |
| libraries.io | SaaS / OSS DB | Cross-package | ⚠️ Partial | ❌ Web | ✅ Yes | ⚠️ SourceRank only |

**Our positioning:** *The only locally-runnable, fully-open-source, CLI-first tool that combines fake-star signals, repo activity, maintainer concentration, ecosystem adoption, and security readiness into a single explainable Trust Report.*

We are complementary, not competitive, with OpenSSF Scorecard and deps.dev — we **consume their data** as inputs to our Adoption and Security modules.

---

## 4. Vision and Goals

### 4.1 Three-year vision

Repo Trust becomes the default `npm audit`-style command-line utility for repository diligence. Engineers run `repo-trust scan owner/repo` before adding a new dependency the same way they run `npm audit` after. Curators, scouts, and journalists cite Trust Reports the way they currently cite Scorecard scores.

### 4.2 Product goals (v1)

1. **One command, full report.** A single CLI invocation produces a complete, evidence-backed Trust Report for any public GitHub repository in under 30 seconds (Standard mode, warm cache).
2. **Explainable scoring.** Every module score is accompanied by ≥3 evidence items and an explicit confidence band (Low / Medium / High).
3. **Reproducibility.** Identical inputs and the same scoring version always produce identical outputs (modulo upstream API state). All scoring versions are pinned and migration-noted.
4. **Conservatism.** Where data is partial, the tool reports lower confidence rather than guessing. False positives in fake-star flagging are treated as worse than false negatives.
5. **Free and self-hostable.** No paid tier, no telemetry by default, no required server-side component.

### 4.3 Non-goals (v1)

- We will not provide an aggregate verdict ("safe" / "unsafe"). The closest we offer is a five-bucket category (Strong / Good / Mixed / Weak / High Risk).
- We will not analyze private repositories in v1. If users want this, they bring their own GitHub token with appropriate scopes; no special handling.
- We will not auto-publish reports anywhere. All outputs are local files until the user shares them.

---

## 5. Trust Model — Five Modules

We compute one **Repo Trust Score** (0–100) as a weighted aggregate of five module scores. The aggregate is useful for orientation; the module breakdown is the real product value.

### 5.1 Module weights (v1, illustrative)

| # | Module | Weight | Why this weight |
| --- | --- | --- | --- |
| 1 | Star Authenticity | 20% | Most-asked question; most novel value |
| 2 | Activity Health | 25% | Strongest single predictor of long-term project survival |
| 3 | Maintainer Health | 20% | Bus-factor risk is real and underweighted by popularity-only views |
| 4 | Adoption Signals | 20% | Real-world usage is the antidote to vanity metrics |
| 5 | Security & Readiness | 15% | Critical but well-served by OSSF Scorecard; we federate, not replicate |

Weights are configurable via `--weights` flag and a `weights.toml` file. Default weights are versioned (v1.0.0).

### 5.2 Module 1: Star Authenticity

**Question:** Are the popularity signals organic?

**Inputs (heuristic-driven, transparent):**
- Fork-to-star ratio (low ratio in a popular repo is suspicious).
- Watcher-to-star ratio.
- Median stargazer account age and account-creation distribution.
- Share of stargazer accounts matching the StarScout / Dagster "low-activity profile" (created recently, ≤1 follower, ≤4 public repos, default avatar, empty bio, star date == account creation date).
- Bursty / lockstep star timing (z-score of starring rate vs trailing baseline).
- Co-starring overlap with known campaign-cluster fingerprints (deep mode only, opt-in via `--deep`).

**Method:** A weighted evidence model in v1, not a black-box ML classifier. We follow the heuristic-first approach validated by Dagster (March 2023) and StarScout (ICSE 2026). Confidence drops sharply when stargazer sample size is below 100 or when GitHub API rate limits truncate the sample.

**Output:** Score 0–100, evidence list, confidence band, sample size disclosure.

**Anti-misuse:** Score language is always conditional ("X% of sampled stargazers match a low-activity profile"). We never publish a binary "fake / real" verdict.

### 5.3 Module 2: Activity Health

**Question:** Is the repository alive and operationally active?

**Inputs:**
- Commit frequency and recency (30 / 90 / 180 / 365-day windows).
- Release cadence and most-recent-release age.
- Issue first-response time (median, p90).
- Pull-request merge rate and review latency.
- Active contributors per 90-day window.
- Continuity score (variance of monthly commit count over 18 months).

**Method:** Threshold-based scoring with ecosystem-aware baselines (a Rust crate with monthly releases scores differently from a stable Python utility with quarterly releases).

### 5.4 Module 3: Maintainer Health

**Question:** Is stewardship sustainable, or does this project sit on one person's shoulders?

**Inputs:**
- Number of active maintainers in the last 365 days.
- Commit concentration (Gini coefficient of commits-per-author).
- Review concentration (Gini coefficient of PR-review actions).
- Bus factor proxy (minimum number of authors required to cover 50% of commits in last 365 days).
- Contributor retention (percent of contributors active in two consecutive 180-day windows).
- Maintainer responsiveness (median response time on issues / PRs by maintainer-flagged users).
- Ownership signals (CODEOWNERS file, MAINTAINERS.md, governance docs).

### 5.5 Module 4: Adoption Signals

**Question:** Is this repository actually used in the wild?

**Inputs (gracefully degrades when sources are unavailable):**
- GitHub `dependents` count and trend.
- Package-registry downloads (npm, PyPI, crates.io, RubyGems, Maven Central, NuGet) via deps.dev API.
- Docker Hub pulls (where applicable).
- Cited in well-known awesome-lists (configurable list).
- Documentation maturity score (presence and length of README, docs/ folder, examples/).
- Real-world reference signals (mentions in deps.dev's package-to-repo mapping graph).

This module **federates** existing public datasets rather than re-collecting them. We are explicit about this in the report.

### 5.6 Module 5: Security & Readiness

**Question:** Is this repository in a state that supports responsible adoption?

**Inputs:**
- OpenSSF Scorecard score (federated via the public API where available; we do not re-implement Scorecard's checks).
- OSV vulnerability count for the repository's published packages.
- Presence and recency of `SECURITY.md`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `LICENSE`, `CODEOWNERS`.
- CI workflow presence and basic shape.
- Release-tagging consistency (semver adherence).
- Branch-protection signals where observable via the API.

We do **not** replicate Scorecard. We import its output and weight it.

---

## 6. Score, Confidence, and Categories

### 6.1 Trust Score

A single integer 0–100, computed as the weighted average of module scores, weighted by both module weight and per-module confidence.

```
trust_score = Σ (module_weight_i × module_score_i × module_confidence_i) / Σ (module_weight_i × module_confidence_i)
```

This means a module with very low confidence contributes less to the overall score, preventing partial-data modules from dominating.

### 6.2 Confidence

Three bands: **Low / Medium / High**. Per-module confidence is determined by:

- **Data completeness** — what fraction of expected inputs were collected.
- **Sample size** — for sample-based modules (Star Authenticity), the size of the sampled population vs the configured target.
- **Cross-signal agreement** — when multiple sub-signals agree, confidence rises.
- **Staleness** — how old the cached data is relative to repo activity.

The overall report confidence is the minimum of any module that contributed >10% of the final score.

### 6.3 Categories

We bucket the score for human readability — but the bucket is always presented alongside the numeric score and the confidence band.

| Range | Category | Meaning |
| --- | --- | --- |
| 85–100 | Strong | Multiple modules score high, no significant warnings |
| 70–84 | Good | Generally healthy, watch any flagged module |
| 50–69 | Mixed | Notable strengths and notable weaknesses |
| 30–49 | Weak | Significant concerns across multiple modules |
| 0–29 | High Risk | Strong negative signals; treat as suspicious until reviewed by a human |

The category is **never** the only thing we display. A "Strong" report with low confidence is presented as `Strong (Low confidence)` and the user is told why.

---

## 7. Functional Requirements

### FR-1: Repository intake
The CLI shall accept a repository specifier in any of the following forms:
- `owner/repo` (e.g. `octocat/Hello-World`)
- Full GitHub URL (`https://github.com/octocat/Hello-World`, with or without trailing path or `.git`)
- Path to a newline-delimited file for batch mode

The CLI shall normalize, validate, and resolve renames (HTTP 301) before scanning.

### FR-2: Three execution modes
| Mode | Target latency (warm cache) | API calls (target) | Scope |
| --- | --- | --- | --- |
| `quick` | < 5s | < 30 | Repo metadata, latest activity, headline signals only |
| `standard` (default) | < 30s | < 200 | All modules at default sampling |
| `deep` | < 5min | < 2000 | Larger stargazer sample, longer historical windows, optional graph analysis |

Quick mode skips Star Authenticity (or runs it in headline-only mode) because it is the most expensive module.

### FR-3: Module selection
Users shall be able to enable / disable any module:

```
repo-trust scan owner/repo --modules activity,maintainers,security
repo-trust scan owner/repo --skip-modules stars
```

### FR-4: Output formats
- **Terminal** (default): colored summary with score, category, top-3 strengths, top-3 concerns, confidence band.
- **JSON**: full machine-readable report with stable schema.
- **Markdown**: human-friendly long-form report.
- **CSV**: tabular form, one row per repo (used for batch).
- **SARIF** (v1.1+): for security-tool integration.
- **HTML** (via `repo-trust serve`): localhost-only viewer.

### FR-5: Deterministic outputs
Same inputs (repo, mode, scoring version, weights, RNG seed) and same upstream API state shall produce byte-identical JSON output (modulo `snapshot_at` and `runtime_seconds` fields).

### FR-6: Caching and rate-limit awareness
- All API responses cached locally in SQLite at `~/.repo-trust/cache.db`.
- Default cache TTL: 24h for repo metadata, 1h for activity, 7d for stargazer pages.
- ETag-aware conditional fetching (`If-None-Match`) on every GitHub request.
- Token-bucket rate limiter coordinating concurrent requests, never exceeding 80% of remaining rate-limit budget.
- Cache management subcommands: `repo-trust cache info`, `repo-trust cache clear`, `repo-trust cache prune`.

### FR-7: Authentication
- Read `GITHUB_TOKEN` env var by default.
- Support `--token` flag (with warning that the value will be in shell history).
- Support `~/.repo-trust/config.toml` with token reference (`token_env = "GH_TOKEN"`).
- For unauthenticated runs (no token), gracefully degrade with clear warning.

### FR-8: Configuration files
Layered configuration loaded via `figment` in priority order:
1. Built-in defaults.
2. User config: `~/.repo-trust/config.toml`.
3. Project config: `./.repo-trust.toml` (in cwd).
4. Environment variables (`REPO_TRUST_*`).
5. CLI flags (highest priority).

### FR-9: Plugin reservation
Reserve a plugin interface for v1.2+. Do not expose it in v1.0 to avoid premature commitment to a stable plugin API.

---

## 8. Non-Functional Requirements

| Requirement | Target |
| --- | --- |
| **Cold-cache p95 latency (Standard mode)** | < 30s |
| **Warm-cache p95 latency (Standard mode)** | < 5s |
| **Memory** | < 200 MB for any single-repo scan; < 1 GB for batch of 100 repos |
| **Disk (cache)** | Default cap 500 MB; user-configurable; LRU eviction |
| **Cross-platform** | Linux (glibc 2.31+), macOS 13+, Windows 11 (via `cargo install`) |
| **Rust toolchain** | stable 1.75+ (MSRV); CI tests latest stable + 1.75 minimum |
| **Test coverage** | `cargo-tarpaulin` ≥ 85% on `src/scoring/` and `src/modules/`; ≥ 70% overall |
| **Documentation** | All public modules have rustdoc with examples; `cargo doc` builds clean |
| **License compliance** | `cargo-deny` allowlist enforced in CI |
| **No telemetry** | Zero outbound calls except to the configured API endpoints |

---

## 9. Distribution

| Channel | Status |
| --- | --- |
| **Cargo + crates.io** (primary): `cargo install repo-trust` | v1.0 launch |
| **Standalone binaries** (Linux x86_64, Linux arm64, macOS arm64, Windows x86_64) via `cross` cross-compilation | v1.0 launch |
| **Docker image** (`ghcr.io/dmitrze/repo-trust:latest`) | v1.0 launch |
| **Homebrew tap** (`brew install dmitrze/tap/repo-trust`) | v1.1 |
| **Winget / Scoop** (Windows) | v1.1 |
| **APT / DNF / Pacman repositories** | post-v1.0, community-maintained |

Releases follow SemVer. Pre-1.0 versions are subject to breaking changes; 1.0 stabilizes the JSON report schema.

---

## 10. CLI Surface (target)

```
repo-trust scan <repo> [options]
repo-trust batch <file> [options]
repo-trust explain <repo>
repo-trust serve [--port N] [--bind ADDR]
repo-trust cache info | clear | prune
repo-trust config show | set <key> <value>
repo-trust version
repo-trust completions <shell>
```

Sample `scan` flags:
```
--mode quick|standard|deep         (default: standard)
--modules <comma-separated>        (default: all)
--skip-modules <comma-separated>
--output <dir>                     (default: ./repo-trust-reports/)
--format <terminal|json|md|csv>... (multi-select; default: terminal)
--weights <path-to-toml>
--scoring-version <semver>         (pin scoring version)
--token <value>                    (or use $GITHUB_TOKEN)
--seed <u64>                       (RNG seed for sampling, default derived from repo)
--refresh                          (invalidate cache)
--refresh-module <name>            (invalidate specific module's cache)
--debug                            (verbose tracing logs)
--quiet                            (no progress output)
--no-color                         (disable terminal colors)
--json                             (alias for --format json --quiet)
```

---

## 11. Reporting Format (target JSON shape)

```json
{
  "schema_version": "1.0.0",
  "repository": {
    "full_name": "owner/repo",
    "url": "https://github.com/owner/repo",
    "default_branch": "main",
    "primary_language": "Rust",
    "stars": 12345,
    "snapshot_at": "2026-05-15T10:00:00Z"
  },
  "overall_score": 73,
  "overall_confidence": "Medium",
  "category": "Good",
  "modules": [
    {
      "module": "stars",
      "score": 81,
      "confidence": "High",
      "sub_scores": { "low_activity_share": 75, "lockstep_timing": 90 },
      "sample_size": 200,
      "missing_data": []
    }
  ],
  "evidence": [
    {
      "module": "stars",
      "code": "low_activity_stargazer_share",
      "label": "Share of low-activity stargazer accounts",
      "value": 0.082,
      "threshold": 0.20,
      "verdict": "Positive",
      "rationale": "8.2% of sampled stargazers match the low-activity profile, well below our 20% concern threshold."
    }
  ],
  "top_strengths": [],
  "top_concerns": [],
  "caveats": ["Stargazer sample limited to 200 due to API rate limit"],
  "scoring_version": "1.0.0",
  "weights_used": { "stars": 0.20, "activity": 0.25, "maintainers": 0.20, "adoption": 0.20, "security": 0.15 },
  "snapshot_at": "2026-05-15T10:00:00Z",
  "runtime_seconds": 12.3
}
```

The schema is **frozen** within a major version. Breaking schema changes require a major version bump and a documented migration path.

---

## 12. Roadmap

### Phase 0 — Research foundation (✅ this PRD and architecture)
Outputs: PRD, architecture, methodology plan, ADRs 0001-0010.

### Phase 1 — Core CLI MVP (target: 6–8 weeks)
- Cargo workspace, CLI skeleton (`scan` only), configuration loading, SQLite cache.
- Activity Health, Maintainer Health, Security & Readiness modules (the three least research-dependent).
- JSON and Markdown report writers.
- Quick + Standard modes.
- ≥ 70% test coverage.

### Phase 2 — Stars + Adoption (4–6 weeks)
- Star Authenticity module with StarScout-style heuristics.
- Adoption Signals module via deps.dev integration.
- Deep mode for stargazer sampling.
- Property tests on scoring functions.

### Phase 3 — Polish + viewer (4 weeks)
- `repo-trust serve` axum web viewer.
- Terminal report polish (`comfy-table`, color output).
- CSV + SARIF outputs.
- crates.io release of v1.0.0 + Homebrew tap + standalone binaries.

### Phase 4 — Adoption (ongoing)
- Apply to GitHub Secure Open Source Fund.
- Apply to Tidelift.
- GitHub Sponsors page activation.
- Conference talk submissions (FOSDEM, OSSummit, RustConf).
- Category-aware baselines (Rust crate vs Python lib vs JS framework).

---

## 13. Success Metrics

| Metric | 3-month target | 12-month target |
| --- | --- | --- |
| GitHub stars (this repo) | 200 | 2,500 |
| Weekly crates.io downloads | 500 | 5,000 |
| Active GitHub Sponsors | 5 | 50 |
| External contributors | 3 | 25 |
| Citations in blog posts / papers | 5 | 50 |
| Reports filed via the issue tracker | 10 | 100 |
| Mean time to triage a methodology question | < 7 days | < 3 days |

---

## 14. Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- |
| GitHub deprecates an API we depend on | Medium | High | Federate via deps.dev where possible; abstract API layer |
| GitHub tightens rate limits | Medium | Medium | ETag caching; deep mode is opt-in; clear docs on token use |
| False-positive "fake star" flag damages a real project | Low | High | Conservative thresholds; never use word "fraud"; allow disputes |
| Tool is misused for scoring contests / shaming | Medium | Medium | Methodology and confidence are foregrounded; we publish principles |
| Maintainer burnout (bus factor 1) | High | High | Transparent governance; recruit co-maintainers; sponsor revenue |
| Competing tool with VC funding launches | Medium | Low | Open source is durable; methodology rigor is the moat |

---

## 15. Open Questions

These are deliberately unresolved at the PRD stage; they will be tracked in `docs/adr/` as we make decisions:

1. Should we ship a default benchmark repo set in `examples/`, or only a methodology + curation guide?
2. Should Star Authenticity have a "headline" sub-mode that runs in `quick` without sampling?
3. How should we handle monorepos (`microsoft/vscode-extensions/something`)?
4. Should we publish a public scoreboard site? (Tentative answer: no, not in v1; risk of misuse.)
5. Should we accept anonymous methodology disputes? (Tentative answer: yes, but require evidence.)

---

*This document supersedes any prior README content about scope. Architecture details live in `docs/architecture.md`.*
