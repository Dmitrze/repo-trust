# Repo Trust — Claude Code Operating Manual

> **This file is the single source of truth for how Claude Code operates on this repository.** Read it before every working session. If anything here conflicts with older docs in `docs/`, this file wins.
>
> **And:** this file is itself downstream of `AI_NATIVE_CONSTITUTION.md` (repo root). If anything here conflicts with the constitution, the constitution wins and this file gets updated.

---

## 1. The product in one paragraph

Repo Trust is an open-source command-line tool that produces an explainable, multi-dimensional **Trust Report** for any public GitHub repository. It is built for engineers, OSS maintainers, scouts, and security teams who need to answer the question "can this repo be trusted?" — beyond the star count. The product surface is a single Rust binary (`repo-trust`) plus an optional localhost web viewer; there is no SaaS dashboard, no required server-side component, and no telemetry by default. **Business shape:** open-source community project (Apache-2.0 + CC-BY-4.0 for methodology) funded via GitHub Sponsors, GitHub Secure Open Source Fund, and similar grants. There is no paid tier and no plan for one. **Target audience:** developers picking dependencies, OSS maintainers benchmarking themselves, analysts at scouts/funds doing diligence at scale, security/platform engineers integrating supply-chain signals into CI.

---

## 2. Non-negotiables (never violate)

1. **No `unsafe` code.** Crate-level `#![deny(unsafe_code)]`. Justified exceptions only via ADR.
2. **Pedantic clippy is law.** `cargo clippy --all-targets --all-features -- -D warnings` is a CI gate.
3. **Deterministic JSON output.** Same inputs + same upstream API state ⇒ byte-identical JSON (modulo `snapshot_at` and `runtime_seconds`). Enforced by `insta` snapshot tests. See [ADR-0007](docs/adr/0007-deterministic-output.md).
4. **No telemetry, ever.** Zero outbound calls except to the configured upstream APIs (GitHub, deps.dev, scorecard.dev, osv.dev).
5. **No black-box scoring.** No ML classifiers in v1. Every score is a function of named features against named thresholds. See [ADR-0004](docs/adr/0004-no-ml-in-v1.md).
6. **Conservative on negative claims.** False positives in fake-star detection harm real maintainers. Bias toward Low confidence over confident accusation.
7. **Federate, don't replicate.** OpenSSF Scorecard, deps.dev, OSV are inputs to our modules — we do not duplicate them. See [ADR-0005](docs/adr/0005-federate-not-replicate.md).
8. **Confidence is reported separately from score.** A two-axis output is mandatory. See [ADR-0008](docs/adr/0008-confidence-separate-from-score.md).
9. **Spec-first / test-first.** No new module or scoring change without a spec in `specs/<feature>.md` and scenarios in `tests/scenarios/<feature>.md`.
10. **Versioned scoring model.** Scoring changes bump `SCORING_VERSION` in `src/lib.rs` and add an entry to `docs/scoring-model.md`.
11. **Frozen JSON schema per major.** `REPORT_SCHEMA_VERSION` only changes on major bumps.
12. **No banned dependencies.** OpenSSL and native-tls are forbidden via `deny.toml` (we use `rustls`).
13. **AI-native by constitution.** Every workflow is closed-loop. Every action produces an artifact. No code without a spec. No feature without scenarios. No outcome without a DRI. (Full rules: `AI_NATIVE_CONSTITUTION.md` and `docs/AI_NATIVE_PLAYBOOK.md`.)

---

## 3. Repository facts

- **Repo:** `Dmitrze/repo-trust` (private until v0.1 ships; public thereafter).
- **Default branch:** `main`.
- **Live URL:** N/A — this is a CLI tool, not a hosted service.
- **Distribution targets:** crates.io, GitHub Releases (binaries), ghcr.io (container), Homebrew tap (v1.1).
- **State:** Phase 0 complete (research foundation); Phase 1 in progress (CLI skeleton + first three modules).
- **Top-level directories:**
  - `src/` — Rust source
  - `tests/` — integration tests + `tests/scenarios/` (BDD-style)
  - `benches/` — criterion benchmarks
  - `examples/` — sample reports, benchmark CSV
  - `docs/` — PRD, architecture, methodology, ADRs (`docs/adr/`)
  - `specs/` — feature specs (one per feature)
  - `agents/` — N/A for this project (we don't ship LLM agents)
  - `runbooks/` — incident response and operations
  - `.github/` — CI workflows, issue/PR templates

---

## 4. Tech stack (exact versions)

See `Cargo.toml` for the authoritative list. Highlights:

- **Edition:** Rust 2021. **MSRV:** 1.75 (pinned in `rust-toolchain.toml` and CI matrix).
- **CLI:** `clap` v4 (derive) + `clap_complete` + `indicatif` + `console` + `comfy-table`.
- **Async:** `tokio` (full features) + `async-trait` + `futures`.
- **HTTP:** `reqwest` with `rustls-tls`, `gzip`, `http2` features. **No `openssl` or `native-tls` — enforced by `deny.toml`.**
- **GitHub API:** `octocrab`.
- **Serialization:** `serde` + `serde_json` + `serde_with` + `schemars` + `toml`.
- **Storage:** `rusqlite` (bundled) + `r2d2` + `r2d2_sqlite` + `rusqlite_migration`.
- **Logging:** `tracing` + `tracing-subscriber` (env-filter + json).
- **Time:** `time` (with serde features).
- **Config:** `figment` (TOML + env layered).
- **Web (optional, `web` feature):** `axum` + `tower-http` + `askama` + `rust-embed`.
- **Graph (optional, `deep` feature):** `petgraph`.
- **RNG (deterministic):** `rand` + `rand_chacha` (ChaCha20).
- **Hashing:** `blake3`.
- **Errors:** `anyhow` (binaries) + `thiserror` (libraries).

**Dev dependencies:** `insta` (snapshots), `proptest`, `wiremock`, `tempfile`, `assert_cmd`, `predicates`, `criterion`.

**Quality tools (CI gates):** `rustfmt`, `clippy`, `cargo-tarpaulin` (coverage ≥ 85% on scoring + modules), `cargo-deny`, `cargo-audit`.

See `REQUIREMENTS.md` for the full developer setup.

---

## 5. The current focus

**Phase 1 — Core CLI MVP.** The three least research-heavy modules first: Activity Health, Maintainer Health, Security & Readiness (federating OSSF Scorecard). Star Authenticity and Adoption Signals come in Phase 2.

Specific in-scope items right now:
- Wire `cli::scan::execute()` end-to-end with the module registry.
- Implement `src/api/github.rs` (octocrab wrappers + ETag tracking).
- Implement `src/storage/cache.rs` (r2d2 SQLite pool + `rusqlite_migration`).
- Implement Activity Health collector + features + scorer with full unit tests.
- Land first `insta` snapshot test against a wiremock-served fixture.

Out of scope until Phase 2:
- Star Authenticity sampling.
- Adoption Signals (deps.dev integration).
- `serve` localhost web viewer.

---

## 6. Pricing & packaging

N/A. This is open source, Apache-2.0 + CC-BY-4.0 (methodology). There is **no paid tier** and there will never be a "Repo Trust Pro." Funding is via GitHub Sponsors, GitHub Secure Open Source Fund, Tidelift, Open Source Collective.

---

## 7. Surfaces / audiences

| Surface | Audience | One-line purpose |
| --- | --- | --- |
| `repo-trust` CLI | Developers, analysts, maintainers | Local trust diligence on a GitHub repo |
| `repo-trust serve` localhost viewer | Same users, prefer GUI | Browse cached reports in a web UI |
| GitHub repo (`Dmitrze/repo-trust`) | Contributors, sponsors | Source of truth, issue tracker |
| crates.io page | Rust developers | Discoverability, version history |
| `docs.rs/repo-trust` | Library consumers (future) | Generated rustdoc |
| GitHub Releases | All users | Pre-built binaries with SLSA provenance |
| `ghcr.io/dmitrze/repo-trust` | CI users | Multi-arch container image with SBOM |

There is no auth model — all surfaces are public-read after v0.1.

---

## 8. Agents

**N/A.** Repo Trust does not ship LLM agents as part of the product. The `agents/` directory is retained from the AI-Native Foundation template but is empty and may be removed in a future commit. We do *use* Claude Code as a development tool (this manual is for Claude Code), but we do not ship LLM-driven scoring or report generation — see [ADR-0004](docs/adr/0004-no-ml-in-v1.md) for the full rationale.

---

## 9. Architecture direction

Layered, as documented in `docs/architecture.md`:

```
  cli/
    ↓
  modules/  (TrustModule trait + registry)
    ↓
  collectors/    features/    scoring/   (pure)
    ↓                          ↑
  api/  →  storage/cache  →  reports/
```

Layering rules:
- `cli` may import any layer.
- `modules` may import `collectors`, `features`, `models`, `scoring`, `utils`.
- `collectors` may import `api`, `storage`, `models`, `utils` — never `modules` or `scoring`.
- `scoring` is pure: no I/O, no async, deterministic.
- `reports` reads from `models` only — never re-runs collectors.

---

## 10. Routes / surfaces inventory

N/A — not a web product. CLI subcommands are: `scan`, `batch`, `explain`, `serve`, `cache`, `config`, `completions`, `version`. See `src/cli/`.

---

## 11. Multi-agent orchestration for Claude Code

We use the **multi-agent master template** from `docs/MULTI_AGENT_TEMPLATE.md`. Critical-review that document at the start of any major engagement; do not apply it blindly.

### Default roles

- **Orchestrator** — the main Claude Code session.
- **Explorer** — separate session for codebase / API investigation.
- **Planner** — separate session for multi-step decomposition.
- **Implementer** — separate session for narrow-scope code writing.
- **Reviewer** — fresh-context review of the diff.
- **Verifier** — runs the code, runs tests, reports real pass/fail.
- **Documenter** — optional; usually documentation is part of Implementer's DoD.

### Mode selection

- **Light:** Orchestrator + Implementer + Reviewer.
- **Standard:** + Explorer + Planner + Verifier.
- **Heavy:** + parallel Implementers + optional Documenter.

Do not default to heavy. Justify the upgrade.

Full rationale and reporting formats: `docs/MULTI_AGENT_TEMPLATE.md`.

---

## 12. Workflow cycle (Boris Cherny playbook)

For every non-trivial change, run the 11-step Boris cycle. Full version: `docs/BORIS_PLAYBOOK.md`. Compressed:

1. Goal in one or two sentences.
2. One concrete happy-path scenario.
3. 3–7 modules.
4. Constrain stack and iteration; declare what's out of scope.
5. Skeleton first (file structure before logic).
6. Short iterations; run after each.
7. Code as conversation — always include current file + exact error.
8. Definition of Done (tests, names, small functions).
9. README / inline doc updated.
10. Verify hallucinations (real crate versions, real API shapes).
11. Checkpoint prompt every ≈1 day or major feature.

---

## 13. Definition of Done

A change is "done" only if all of these hold:

- `cargo build --all-features --all-targets` passes.
- `cargo fmt --all -- --check` passes.
- `cargo clippy --all-targets --all-features -- -D warnings` passes.
- `cargo test --all-features` passes (including `--doc`).
- For new/changed scoring: `insta` snapshot tests updated and reviewed.
- For new public items: rustdoc with at least one example.
- For new runtime crates: justified in PR description.
- No secrets in the diff.
- `CHANGELOG.md` `[Unreleased]` updated.
- For scoring changes: `docs/scoring-model.md` change log entry + scoring version bump.
- For schema changes: `REPORT_SCHEMA_VERSION` bump + migration notes.
- Conventional Commit message (`feat:`, `fix:`, `refactor:`, `docs:`, `chore:`, `test:`, `ci:`).
- A short note written to MemPalace diary for the relevant wing.
- **Spec exists in `/specs/<feature>.md`** for non-trivial features.
- **Scenarios in `/tests/scenarios/<feature>.md`** for non-trivial features.

---

## 14. Glossary

| Term | Use this | Don't use |
| --- | --- | --- |
| The composite output for a repo | "Trust Report" | "trust score document", "repo card" |
| The 0–100 number | "Trust Score" | "score", "rating", "grade" |
| One of the five scoring components | "module" | "check", "factor", "axis" |
| The 0–100 number from one module | "module score" | "module rating" |
| The Low/Medium/High band | "confidence" | "reliability", "certainty" |
| The five-bucket label | "category" | "verdict", "grade" |
| A piece of evidence | "evidence item" | "finding", "check result" |
| Stargazer accounts matching the 9-signal profile | "low-activity profile" | "fake account", "bot account" |
| A repo's score is suspicious-looking | say so probabilistically: "X% of sampled stargazers match a low-activity profile" | NEVER "this repo is fake", "fraudulent", "bot-driven" |
| Federating Scorecard / deps.dev / OSV | "federate" | "integrate", "reuse" (those are weaker) |

---

## 15. Quality standard — Boil the Ocean

The marginal cost of completeness is near zero with AI. Do the whole thing. Do it right. Do it with tests. Do it with documentation. Do it so well that the founder is genuinely impressed — not politely satisfied, actually impressed. Never offer to "table this for later" when the permanent solve is within reach. Never leave a dangling thread when tying it off takes five more minutes. Never present a workaround when the real fix exists. The standard isn't "good enough" — it's "holy shit, that's done." Search before building. Test before shipping. Ship the complete thing. When the founder asks for something, the answer is the finished product, not a plan to build it. Time is not an excuse. Fatigue is not an excuse. Complexity is not an excuse.

---

## 16. Review Gate

Every diff is read critically by a senior reviewer (human or Reviewer agent or Codex) who will not accept hand-waving. Leave a clean trail: commit messages explain why, code explains how, Definition of Done items are visibly satisfied. For methodology / scoring changes, the review *also* checks the benchmark report and ADR.

---

## 17. MemPalace usage

This project uses MemPalace for cross-session memory. See `docs/MEMPALACE_INTEGRATION_GUIDE.md` for setup and `mempalace.yaml` for the wing/room layout.

- **At session start:** run `mempalace_list_agents` and search the relevant wing/room for prior context before implementing.
- **At session end:** write a diary entry for the wing you worked in. One to three sentences.
- **For architectural decisions:** add a knowledge-graph triple in the `decisions` room and consider promoting to an ADR.
- **For methodology refinements:** record in the `methodology` wing.
- **Do not dump everything into CLAUDE.md.** This file stays lean; long-tail context lives in the palace.

---

## 18. Development phases priority

### Phase 0 — Research foundation ✅ (this PRD, architecture, ADRs 0001-0010)

### Phase 1 — Core CLI MVP (target: 6–8 weeks)
In-scope:
- Cargo workspace, CLI skeleton (`scan` only end-to-end), config loading, SQLite cache.
- Three modules: Activity Health, Maintainer Health, Security & Readiness.
- JSON and Markdown report writers.
- Quick + Standard execution modes.
- ≥ 70% test coverage; ≥ 95% on `src/scoring/`.

### Phase 2 — Stars + Adoption (4–6 weeks)
- Star Authenticity module with StarScout-style heuristics.
- Adoption Signals module via deps.dev integration.
- Deep mode for stargazer sampling.
- Property tests on scoring functions.

### Phase 3 — Polish + viewer (4 weeks)
- `repo-trust serve` axum web viewer.
- Terminal report polish (`comfy-table`, color output).
- CSV + SARIF outputs.
- crates.io v1.0.0 release + Homebrew tap + standalone binaries.

### Phase 4 — Adoption (ongoing)
- Apply to GitHub Secure Open Source Fund.
- Apply to Tidelift.
- GitHub Sponsors page activation.
- Conference talk submissions (FOSDEM, RustConf, OSSummit).
- Recruit co-maintainers (governance bus-factor).

---

## 19. Where to find everything

- **The constitution:** `AI_NATIVE_CONSTITUTION.md` (root).
- **AI-native playbook:** `docs/AI_NATIVE_PLAYBOOK.md`.
- **Boris workflow cycle:** `docs/BORIS_PLAYBOOK.md`.
- **Multi-agent template:** `docs/MULTI_AGENT_TEMPLATE.md`.
- **Superpowers integration:** `docs/SUPERPOWERS_INTEGRATION.md`.
- **MemPalace guide:** `docs/MEMPALACE_INTEGRATION_GUIDE.md`.
- **Product Requirements Document:** `docs/PRD.md`.
- **Architecture:** `docs/architecture.md`.
- **Methodology:** `docs/methodology.md`.
- **Versioned scoring model:** `docs/scoring-model.md`.
- **Module specs:** `docs/module-specs.md`.
- **Benchmark plan:** `docs/benchmark-plan.md`.
- **API notes:** `docs/api-notes.md`.
- **Governance:** `docs/governance.md`.
- **Architecture Decision Records:** `docs/adr/0001-*.md` through `0010-*.md`.
- **Specs (per feature):** `specs/<feature>.md`.
- **Scenarios (per feature):** `tests/scenarios/<feature>.md`.
- **Runbooks:** `runbooks/<scenario>.md`.
- **Closed-loop inventory:** `docs/Closed Loops Inventory.md`.
- **Token budget:** `docs/Token Budget.md`.
- **Founder discipline:** `docs/Founder Discipline.md`.
- **Changelog:** `CHANGELOG.md`.
- **Setup:** `REQUIREMENTS.md`.
- **Contributing:** `CONTRIBUTING.md`.
- **Code of Conduct:** `CODE_OF_CONDUCT.md`.
- **Security:** `SECURITY.md`.

If a doc in `docs/` contradicts this file, this file wins. If `AI_NATIVE_CONSTITUTION.md` contradicts this file, the constitution wins.

---

## 20. AI-Native Operating Principles — binding for Claude Code

Unchanged from the AI-Native Foundation template. See sections 20.1–20.10 in the constitution and the previous template revision — the operating principles (read order, mandatory response template, spec-first rule, artifact rule, context parity check, no middleware, token-max default, founder-in-the-loop, DRI rule, closed-loop check) apply to Repo Trust without modification.

---

## 21. Superpowers integration

Unchanged from template. See `docs/SUPERPOWERS_INTEGRATION.md`. The seven Superpowers skills (`brainstorming`, `writing-plans`, `subagent-driven-development`, `test-driven-development`, `requesting-code-review`, `using-git-worktrees`, `systematic-debugging`) map onto the Repo Trust workflow without modification.

---

**Last updated:** 2026-05-03 — adapted from AI-Native Foundation template for Repo Trust (Rust CLI for repository diligence).
