# Changelog

All notable changes to **Repo Trust** are documented here. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

The **scoring methodology** is versioned independently — see
[`docs/scoring-model.md`](docs/scoring-model.md) for the per-rule
change log.

---

## [0.1.0] — 2026-05-04

First public alpha. All five trust modules ship end-to-end with a
deterministic JSON output, CC-BY-4.0 methodology, and strict CI gates.

### Added
- **Five trust modules.** Star Authenticity, Activity Health,
  Maintainer Health, Adoption Signals, Security & Readiness, each
  contributing a sub-score, a confidence band, and an evidence list to
  the overall Trust Score.
- **`scan` command** with three modes (`quick`, `standard`, `deep`)
  and five output formats (terminal, JSON, Markdown, CSV, plus the
  local web viewer at `repo-trust serve`).
- **`batch`, `explain`, `cache`, `config`, `completions`** subcommands.
- **Federation clients** for OpenSSF Scorecard, deps.dev, OSV.dev, and
  the GitHub REST + GraphQL APIs, with ETag-aware caching to a local
  SQLite store.
- **Methodology document** ([`docs/methodology.md`](docs/methodology.md)),
  CC-BY-4.0 — citable in research, adaptable in derivative work.
- **Versioned scoring model** ([`docs/scoring-model.md`](docs/scoring-model.md)),
  shipped at scoring `1.1.1`.
- **12 Architecture Decision Records** under [`docs/adr/`](docs/adr/),
  covering deterministic output, federation strategy, and the
  GitLab-adapter scaffold.
- **313 tests** — unit + integration + snapshot + property — wired
  into a CI matrix (Ubuntu, macOS) with strict gates.

### Quality gates (all green at release)
- `cargo test --all-features` — 313 passed, 0 failed.
- `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic`.
- `cargo fmt --check`.
- `cargo deny check` (licenses + advisories + bans + sources).
- `cargo audit` (RUSTSEC advisories).
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features`.
- `cargo tarpaulin --fail-under 75` — line coverage **89.34%**.

### Notable hardening landed before the public flip
- **Tolerant `scorecard.dev` date parsing** — accepts every ISO-8601
  / RFC 3339 / date-only format scorecard.dev has emitted (the
  upstream API does not commit to a single sub-format).
- **deps.dev v3alpha `:packageversions` integration** — the older
  `:packages` endpoint and `weeklyDownloads` field were retired
  upstream; the client now consumes the new endpoint and filters
  the response by `relationProvenance` + owner-aware name-match
  to count only first-party publication relationships (avoiding
  transitive `SOURCE_REPO` false positives).
- **Adoption confidence 1.1.1** — re-tiered around what the new
  deps.dev contract actually exposes (ecosystem coverage) with a
  generous `is_well_documented` predicate that promotes the common
  short-README + `examples/` library-project layout (clap, serde,
  tower, axum) to High confidence when packages are present.

### Deliberately deferred
- OSV per-dependency walking — partial federation in `0.1.0`,
  full walk lands in `0.2.x`.
- Cross-ecosystem download volume signal — deps.dev v3 dropped
  `weeklyDownloads`. A replacement (PyPI / npm / crates.io) is
  scoped for `0.2.x`.
- `--exit-code-on-category` for CI policy gates — `0.2.0`.
- Pre-built binaries, Homebrew formula, Docker image,
  `cargo install repo-trust` from crates.io — `0.1.x` patch
  releases.
- GitLab adapter — scaffold-only in `0.1.0` per
  [ADR-0011](docs/adr/0011-gitlab-adapter.md), beta in `0.2.0`.

### Disclaimer
`repo-trust` produces a **probabilistic signal** designed to assist
human judgment. It is not a security audit, legal advice, or a
substitute for due diligence. Categories like `HighRisk` reflect
score thresholds against documented heuristics, not allegations of
misconduct. False positives can occur — please report calibration
concerns through the
[calibration template](.github/ISSUE_TEMPLATE/calibration.yml).

[0.1.0]: https://github.com/Dmitrze/repo-trust/releases/tag/v0.1.0
