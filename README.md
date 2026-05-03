<div align="center">

# 🛡️ Repo Trust

**A command-line tool that tells you whether an open-source repository deserves your trust — beyond the star count.**

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)
[![Rust stable (1.75+)](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://www.rust-lang.org/)
[![Status: Pre-alpha](https://img.shields.io/badge/status-pre--alpha-orange.svg)](#project-status)
[![PRs welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](CONTRIBUTING.md)
[![OpenSSF Scorecard](https://api.scorecard.dev/projects/github.com/Dmitrze/repo-trust/badge)](https://scorecard.dev/viewer/?uri=github.com/Dmitrze/repo-trust)

```
$ repo-trust scan octocat/Hello-World

Repo Trust Report  ·  octocat/Hello-World
Trust Score: 73 / 100   ·   Category: Good   ·   Confidence: Medium

  Module                     Score   Confidence
  Star Authenticity            81    High
  Activity Health              68    High
  Maintainer Health            54    Medium
  Adoption Signals             88    High
  Security & Readiness         71    Medium
```

</div>

---

## Why this exists

In 2024, researchers identified [**6 million suspected fake stars across 18,617 GitHub repositories**](https://arxiv.org/pdf/2412.13459). At one point, **16% of repositories with 50+ stars** showed signs of fake-star campaigns. GitHub stars are a popularity signal, not a trust signal — yet developers, scouts, and analysts use them as a shortcut for credibility every day.

Existing tools fix part of the problem:
- **OpenSSF Scorecard** is excellent — but it scores *security*, not trust.
- **deps.dev** aggregates rich data — but is API-only, no opinion.
- **Snyk Advisor** and **Socket.dev** are SaaS-gated.
- **StarScout** is a research artifact, not a tool you can install.

**Repo Trust** is the missing diligence layer: a free, fully open-source, locally-runnable command-line tool that combines five trust dimensions into one explainable report.

---

## What it does

For any public GitHub repository, `repo-trust scan` computes a **Trust Score (0–100)** broken into five modules:

| Module | What it measures |
| --- | --- |
| 🌟 **Star Authenticity** | Are the popularity signals organic? Detects fake-star campaign patterns using StarScout-style heuristics (low-activity stargazer share, lockstep timing). |
| 🩺 **Activity Health** | Is the repo alive? Commit cadence, release rhythm, issue/PR latency, contributor activity over multiple windows. |
| 👥 **Maintainer Health** | Is stewardship sustainable? Bus-factor proxy, commit/review concentration, contributor retention. |
| 📈 **Adoption Signals** | Is it actually used? Package-registry downloads, GitHub dependents, ecosystem citations. |
| 🛡️ **Security & Readiness** | Is it ready for production use? Federates OpenSSF Scorecard, OSV vulnerabilities, presence of `SECURITY.md` / CI / branch protection. |

Every score comes with **evidence**, a **confidence band** (Low / Medium / High), and **caveats** when data is partial. We never say "this repo is fraud." We say "X% of sampled stargazers match a low-activity profile" and let you draw the conclusion.

---

## Install (target — not yet shipped)

```bash
# Cargo (Rust) — primary
cargo install repo-trust

# Homebrew (planned, v1.1)
brew install dmitrze/tap/repo-trust

# Docker
docker run --rm ghcr.io/dmitrze/repo-trust scan octocat/Hello-World

# Standalone binaries (planned, v1.1)
# Linux x86_64, Linux arm64, macOS arm64, Windows x86_64
# Download from: https://github.com/Dmitrze/repo-trust/releases
```

> **⚠️ Project status:** Pre-alpha. APIs and outputs will change before `v1.0.0`. Do not depend on this in production yet. See [the roadmap](docs/PRD.md#12-roadmap).

---

## Use it

### Single repo
```bash
# Default standard mode, writes to ./repo-trust-reports/
repo-trust scan octocat/Hello-World

# Quick mode (< 5 seconds, headline signals only)
repo-trust scan octocat/Hello-World --mode quick

# Deep mode (full stargazer sampling, graph analysis, requires GITHUB_TOKEN)
export GITHUB_TOKEN=ghp_...
repo-trust scan octocat/Hello-World --mode deep
```

### Specific modules
```bash
repo-trust scan octocat/Hello-World --modules activity,maintainers,security
repo-trust scan octocat/Hello-World --skip-modules stars
```

### Batch mode
```bash
echo "facebook/react" >> repos.txt
echo "vercel/next.js" >> repos.txt
echo "django/django" >> repos.txt

repo-trust batch repos.txt --format table --output ./reports/
repo-trust batch repos.txt --json > batch.json
```

### Dig deeper
```bash
repo-trust explain octocat/Hello-World        # full evidence walkthrough
repo-trust serve                              # localhost:8765 web viewer
repo-trust export octocat/Hello-World --md --json --csv
```

---

## How is it different from OpenSSF Scorecard?

OpenSSF Scorecard answers: **"Does this project follow security best practices?"**

Repo Trust answers: **"Should I trust this repository?"**

| Question | OpenSSF Scorecard | Repo Trust |
| --- | --- | --- |
| Is the repo actively maintained? | ✅ (maintained check) | ✅ (Activity Health module) |
| Are there CI workflows and signed releases? | ✅ | ✅ (federates Scorecard) |
| Does it have unfixed vulnerabilities? | ✅ | ✅ (federates OSV via Scorecard) |
| **Are the stars organic?** | ❌ | ✅ (Star Authenticity) |
| **Is one maintainer doing 90% of the work?** | ❌ | ✅ (Maintainer Health) |
| **Is the project actually adopted in the wild?** | ❌ | ✅ (Adoption Signals) |
| **What's the overall trust signal?** | ❌ (security only) | ✅ (weighted composite) |

We **federate** Scorecard's security score rather than replicating it. If you only want security, run Scorecard. If you want trust, run us.

---

## How is it different from Snyk Advisor / Socket.dev?

- **Open source.** Apache-2.0. No paid tier.
- **CLI-first.** No SaaS account required.
- **Local-first.** No telemetry by default.
- **Explainable.** Every score has evidence and a confidence band.
- **Reproducible.** Same inputs + scoring version → byte-identical JSON.
- **Versioned methodology.** Scoring changes are SemVer-tracked.

---

## Methodology

Read the full methodology in [`docs/methodology.md`](docs/methodology.md). Highlights:

- **No black-box ML in v1.** Scoring is a transparent weighted-evidence model with documented thresholds.
- **Confidence is independent of score.** A high-score-low-confidence repo is presented differently than a high-score-high-confidence repo.
- **Federate, don't replicate.** We import OpenSSF Scorecard, deps.dev, and OSV outputs as inputs to our modules.
- **Conservative by design.** When data is partial, we report lower confidence. False positives on the fake-star flag are treated as worse than false negatives.

---

## Documentation

| Document | What it covers |
| --- | --- |
| [`docs/PRD.md`](docs/PRD.md) | Product Requirements — scope, goals, modules, roadmap |
| [`docs/architecture.md`](docs/architecture.md) | Architecture — modules, data flow, technology choices |
| [`docs/methodology.md`](docs/methodology.md) | Public methodology — what we measure and how |
| [`docs/scoring-model.md`](docs/scoring-model.md) | Versioned scoring weights, thresholds, change log |
| [`docs/module-specs.md`](docs/module-specs.md) | Per-module input/output contracts |
| [`docs/benchmark-plan.md`](docs/benchmark-plan.md) | How we benchmark and validate our scoring |
| [`docs/api-notes.md`](docs/api-notes.md) | GitHub API quirks, rate-limit notes |
| [`docs/governance.md`](docs/governance.md) | Project governance |
| [`docs/adr/`](docs/adr/) | Architecture Decision Records |

---

## Project status

**Phase 0 — Research foundation** ✅ (this PRD and architecture)
**Phase 1 — Core CLI MVP** 🛠️ in progress
**Phase 2 — Star Authenticity & Adoption** ⏳
**Phase 3 — Deep mode & local viewer** ⏳

See the [full roadmap in the PRD](docs/PRD.md#12-roadmap).

---

## Contributing

We need help. The fastest ways to make a difference:

1. **Try the tool** (once Phase 1 ships) and file issues with real-world repos where the score feels off.
2. **Add module specs** — extend `docs/module-specs.md` with edge cases.
3. **Curate the benchmark set** — propose repos for the benchmark categories.
4. **Add a new module** — see [the plugin design](docs/architecture.md#4-module-contract) (planned for v1.2; we welcome design input now).

Read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a PR. We follow [Conventional Commits](https://www.conventionalcommits.org/) and the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md).

---

## Sponsorship

Repo Trust is built and maintained as a community project. If your team uses it in CI or for diligence at scale, please consider sponsoring its maintenance.

[![GitHub Sponsors](https://img.shields.io/github/sponsors/Dmitrze?logo=GitHub-Sponsors&style=for-the-badge)](https://github.com/sponsors/Dmitrze)

Sponsorship goes directly to maintainer time. We have **no paid tier and no plans for one**. There will never be a "Repo Trust Pro." Funding lets the existing OSS project survive.

We are also applying to:
- [GitHub Secure Open Source Fund](https://github.com/open-source/github-secure-open-source-fund)
- [Tidelift](https://tidelift.com/)
- [Open Source Collective](https://opencollective.com/opensource)

If you represent a company that benefits from supply-chain security tooling and you'd like to discuss enterprise sponsorship, open a [discussion](https://github.com/Dmitrze/repo-trust/discussions).

---

## Acknowledgements

Repo Trust stands on the shoulders of:

- The **OpenSSF Scorecard** team for setting the standard on open security health metrics.
- **Google Open Source Insights / deps.dev** for the public package-and-repo metadata graph.
- **OSV.dev** for the open vulnerability database.
- The **StarScout** authors (He et al., ICSE 2026) for the rigorous fake-star detection methodology that informs our Star Authenticity module.
- The **Dagster** team for [the original 2023 fake-star investigation](https://dagster.io/blog/fake-stars) and for open-sourcing [`fake-star-detector`](https://github.com/dagster-io/fake-star-detector).

If you publish research on repository trust, signal, or supply-chain integrity and we cite or build on your work, please tell us — we'll add proper attribution.

---

## License

[Apache 2.0](LICENSE) © 2026 Repo Trust contributors

The methodology documents in `docs/` are additionally licensed under [CC-BY-4.0](https://creativecommons.org/licenses/by/4.0/) so that the methodology can be cited and adapted in academic and industry work.

---

<div align="center">

**Trust over hype.** **Explanations over scores.** **Free and open, forever.**

</div>
