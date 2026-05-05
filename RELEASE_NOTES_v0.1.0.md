# v0.1.0 — First public alpha 🚀

**Repo Trust** tells you whether a GitHub repository deserves your trust — beyond the star count.

This is the first public release. The five trust modules ship end-to-end, the methodology is documented and citable (CC-BY-4.0), and the output is deterministic. APIs may evolve before `v1.0.0` — pin a version when integrating.

---

## What's in the box

For any public GitHub repository, `repo-trust scan owner/name` produces:

- A **Trust Score (0–100)** broken into 5 modules with sub-scores, evidence, and a confidence band.
- Five output formats: human-friendly **terminal**, **JSON** (deterministic, suitable for diffing in CI), **Markdown** (suitable for PRs), **CSV** (suitable for spreadsheets), and a local **web viewer** (`repo-trust serve`).
- Three modes: `--mode quick` (~5s, ~30 API calls), `--mode standard` (~30s, default), `--mode deep` (~5min, full stargazer sampling).

### The 5 modules

1. **⭐ Star Authenticity** — Detects fake-star patterns using the StarScout 9-signal low-activity profile composite + lockstep timing z-score + ecosystem-aware fork/watcher ratios. Always probabilistic phrasing — never says "fraud" or "fake."
2. **📈 Activity Health** — Commit cadence over 30/90/365-day windows, release rhythm, issue & PR latency, contributor activity windows.
3. **👥 Maintainer Health** — Bus-factor proxy, Gini coefficient on commits-by-author, contributor retention, governance docs.
4. **🌍 Adoption Signals** — Federates [deps.dev](https://deps.dev) for first-party package presence across CARGO / NPM / PYPI / GO / MAVEN. Documentation maturity.
5. **🔒 Security & Readiness** — Federates [OpenSSF Scorecard](https://scorecard.dev) (freshness-weighted) + [OSV.dev](https://osv.dev) for vulnerabilities + doc presence + CI workflows + semver discipline.

### Federation, not replication

We **consume** OpenSSF Scorecard, deps.dev, and OSV through thin ETag-aware clients with local SQLite caching. We don't duplicate what they do — we add the trust dimensions they don't cover (Star Authenticity, Maintainer Health, Adoption Signals).

### Determinism

Same inputs + same scoring version ⇒ byte-identical JSON (modulo `snapshot_at` and `runtime_seconds`). Enforced by snapshot tests + property tests. See [ADR-0007](docs/adr/0007-deterministic-output.md).

### Methodology

The scoring methodology is fully documented in [`docs/methodology.md`](docs/methodology.md), licensed **CC-BY-4.0** so you can cite it in research or adapt it. Per-rule scoring weights and thresholds are versioned in [`docs/scoring-model.md`](docs/scoring-model.md). This release ships scoring **1.1.1**.

---

## Install

```bash
cargo install --git https://github.com/Dmitrze/repo-trust --tag v0.1.0
```

Requires Rust 1.75+. Get it via [rustup.rs](https://rustup.rs/).

```bash
# Recommended — set a GitHub token for the higher rate limit (5000/hr vs 60/hr).
# Create one at https://github.com/settings/tokens with scope: public_repo
export GITHUB_TOKEN=ghp_...

# Scan a repo
repo-trust scan tokio-rs/tokio

# Or run the local web viewer
repo-trust serve
# → open http://localhost:8765
```

---

## Quality gates passed

- ✅ **313 tests** (unit + integration + snapshot + property) — all green on Ubuntu and macOS.
- ✅ **clippy::pedantic** with `-D warnings` — clean.
- ✅ **cargo-deny** (licenses, advisories, bans, sources) — clean.
- ✅ **cargo-audit** — no known RUSTSEC advisories.
- ✅ **rustdoc** with `-D warnings` — clean.
- ✅ **Code coverage** — 89.34% (gate: ≥75%).
- ✅ **OpenSSF Scorecard** integration live — [view our own score](https://scorecard.dev/viewer/?uri=github.com/Dmitrze/repo-trust).

---

## What's deliberately deferred

This is an alpha — these gaps are known, documented, and on the roadmap:

- **OSV deep-walk per-dependency** — partial federation in `0.1.0`, full walk in `0.2.x`.
- **Cross-ecosystem download volume signal** — deps.dev v3 dropped `weeklyDownloads`; a replacement source (PyPI / npm / crates.io APIs) is scoped for `0.2.x`.
- **`--exit-code-on-category` for CI policy gates** — `0.2.0`.
- **Pre-built binaries / Homebrew / `cargo install repo-trust` from crates.io / Docker image** — `0.1.x` patch releases.
- **GitLab adapter** — scaffold-only here, beta in `0.2.0`. See [ADR-0011](docs/adr/0011-gitlab-adapter.md).

---

## Disclaimer

`repo-trust` produces a **probabilistic signal** designed to assist human judgment. It is **not** a security audit, legal advice, or a substitute for due diligence. Categories like `HighRisk` reflect score thresholds against documented heuristics, not allegations of misconduct. False positives can occur — please report calibration concerns via the [calibration template](https://github.com/Dmitrze/repo-trust/issues/new?template=calibration.yml).

---

## Acknowledgements

Repo Trust stands on the shoulders of:

- The **OpenSSF Scorecard** team for setting the standard on open security health metrics.
- **Google Open Source Insights / deps.dev** for the public package metadata graph.
- **OSV.dev** for the open vulnerability database.
- The **StarScout** authors (He et al., ICSE 2026) for the rigorous fake-star detection methodology that informs our Star Authenticity module.
- The **Dagster** team for [the original 2023 fake-star investigation](https://dagster.io/blog/fake-stars) and for open-sourcing [`fake-star-detector`](https://github.com/dagster-io/fake-star-detector).

---

## Get involved

- ⭐ **Star the repo** — discoverability genuinely depends on it.
- 🐛 **Try it on your favorite repos** and file [calibration feedback](https://github.com/Dmitrze/repo-trust/issues/new?template=calibration.yml) if a score feels wrong. Real-world calibration is the most valuable thing you can contribute.
- 💬 **Discuss** — open a [Discussion](https://github.com/Dmitrze/repo-trust/discussions) for ideas, methodology questions, or feedback.
- 💖 **[Sponsor on GitHub](https://github.com/sponsors/Dmitrze)** — Repo Trust is and remains free, Apache-2.0, and self-hostable forever. No paid tier. No SaaS gating.

**Built and maintained by [Dmitry Melnik](https://dmitrymelnik.ai).**

---

**Trust over hype. Explanations over scores. Free and open, forever.**
