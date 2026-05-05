# Contributing to Repo Trust

Thanks for thinking about contributing — this project lives or dies by community input. Read this whole file before opening your first PR.

---

## Ground rules

1. **Code of Conduct.** All interactions are governed by the [Contributor Covenant](CODE_OF_CONDUCT.md). No exceptions.
2. **Trust before taste.** This project is about epistemic honesty. If a change makes the tool look better but reduces transparency or determinism, it will be rejected.
3. **Conservative on negative claims.** Anything that increases the risk of false-positive "fake / suspicious" flags requires unusually strong justification.
4. **No surprise dependencies.** New runtime crates need a justification in the PR description. We prefer fewer, well-vetted dependencies.
5. **Apache-2.0, no DCO or CLA.** This project is licensed under [Apache-2.0](LICENSE). By opening a pull request you agree your contribution is licensed under the same terms — that's it. We do not require Developer Certificate of Origin sign-off and we do not ask contributors to sign a Contributor License Agreement.

---

## How to help

### If you have 5 minutes
- Star the repo, share the link.
- Run the tool against a repo you maintain and file an issue if the score feels wrong (please attach the JSON report).

### If you have 1 hour
- Read [`docs/methodology.md`](docs/methodology.md) and propose a refinement.
- Add a repo to the benchmark set in `examples/benchmark-set.csv` with a short justification.
- Improve documentation — typos, clarity, wrong claims, missing context.

### If you have a weekend
- Pick up an issue labeled `good-first-issue` or `help-wanted`.
- Add a missing test for a feature pipeline.
- Implement a missing collector (see the issue tracker for the current list).

### If you are doing serious work
- Open a discussion *before* writing code if your change spans multiple modules or touches scoring weights.
- Propose an Architecture Decision Record in `docs/adr/`.

---

## Development setup

Prerequisites:
- **Rust** stable 1.75 or newer (install via [rustup](https://rustup.rs/))
- **Git** 2.40+
- A **GitHub Personal Access Token** (read-only, no scopes needed for public repos) for testing real API calls

```bash
# Clone
git clone https://github.com/Dmitrze/repo-trust.git
cd repo-trust

# Install Rust toolchain components (rustfmt + clippy)
rustup component add rustfmt clippy

# Build everything
cargo build --all-features

# Run tests
cargo test

# Install the binary locally so you can test it
cargo install --path .

# Verify
repo-trust --help
```

Set your token (only needed for real-API tests):
```bash
export GITHUB_TOKEN=ghp_your_token_here
```

---

## Working on the code

### Branches
- `main` is always releasable.
- Feature branches: `feat/<short-description>`.
- Bugfix branches: `fix/<short-description>`.
- Doc-only branches: `docs/<short-description>`.

### Commits
We use [Conventional Commits](https://www.conventionalcommits.org/). Example:

```
feat(stars): add lockstep timing detection
fix(cache): prevent stale ETag on 304 response
docs(methodology): clarify sample size rationale
chore(deps): bump octocrab to 0.39
```

### Code style
- `cargo fmt --all -- --check` must pass. CI fails otherwise.
- `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic` must pass. Pedantic warnings are surfaced as warnings, not errors — but new code should not introduce any. If you legitimately need to silence one, prefer a scoped `#[allow(...)]` with a one-line comment over editing the crate-level allowlist in `src/lib.rs`.
- `cargo deny check` must pass. We forbid `openssl` and `native-tls` (we use `rustls`); we ban duplicate license-incompatible crates and prohibit known-vulnerable versions.
- `cargo audit` must report zero unaddressed advisories. Allowed only via documented exception in `.cargo/audit.toml` with rationale and a target review date.
- Public items need doc comments (`///`) and examples where reasonable.
- Tests for new behavior — no exceptions for non-trivial logic.

### Tests
```bash
# All tests
cargo test

# Just one crate or module
cargo test --package repo-trust scoring

# With output (useful when debugging)
cargo test -- --nocapture

# Coverage (requires cargo-tarpaulin)
cargo install cargo-tarpaulin
cargo tarpaulin --out Html --output-dir target/coverage

# Slow benchmark suite (requires network)
cargo test --features benchmark --release -- --ignored
```

We aim for:
- ≥ 80% overall line coverage
- ≥ 95% on `src/scoring/`
- ≥ 90% on `src/modules/`

### Determinism
The CLI must produce deterministic output: same inputs + same scoring version + same RNG seed = byte-identical JSON (modulo `snapshot_at` and `runtime_seconds`). Snapshot tests via [`insta`](https://insta.rs/) enforce this. If your change breaks them, either:
1. The change is wrong.
2. The change is right and you need to review the snapshots and explain in the PR.

To regenerate snapshots:
```bash
# Interactive review (recommended — review each diff before accepting):
cargo install cargo-insta
cargo insta review

# Or accept all in one shot (only when you're confident):
INSTA_UPDATE=always cargo test --all-features
```
Either way, commit the updated `*.snap` files alongside the code change.

---

## Pull request checklist

Before opening a PR:

- [ ] Branch is rebased onto current `main`.
- [ ] `cargo fmt --check` passes.
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes.
- [ ] `cargo test` passes.
- [ ] If you changed scoring: bumped the scoring version in `docs/scoring-model.md` and added a change-log entry.
- [ ] If you changed the JSON report schema: bumped `schema_version` and added migration notes.
- [ ] If you added a runtime crate: justified in the PR description.
- [ ] If you fixed a bug: added a regression test.
- [ ] If you added a public function or CLI flag: added documentation.

PR description must include:
- **What** changed.
- **Why** it changed.
- **How** to verify.
- **Risks** (if any).

---

## Adding a new module (advanced)

The five v1 modules are: Stars, Activity, Maintainers, Adoption, Security. Adding a sixth is a major change and requires:

1. An ADR in `docs/adr/` with the proposal.
2. A discussion thread or issue with positive maintainer signal.
3. A scoring-version bump (it changes weight semantics).
4. Implementation against the [`TrustModule` trait](docs/architecture.md#4-module-contract).
5. ≥ 90% test coverage on the new module.
6. Documentation in [`docs/module-specs.md`](docs/module-specs.md).

Before writing code, read the existing modules under [`src/scoring/`](src/scoring/) and their per-module specs in [`docs/module-specs.md`](docs/module-specs.md) — the file layout, the collector → features → scorer split, and the `TrustModule` impl shape are deliberate. Mirror them. Each existing module has a corresponding spec at `specs/<module-name>.md` and BDD-style scenarios at `tests/scenarios/<module-name>.md`; both are required for new modules too.

---

## Reporting issues

### Bug reports
Use the bug template. Include:
- Repo Trust version (`repo-trust --version`)
- Rust version (`rustc --version`)
- OS
- Repo you ran against
- The full JSON report (or as much as you can share)
- Expected vs actual behavior

### Methodology questions / score disputes
Use the methodology template. Include:
- The repo and the score it received.
- Why you disagree (with evidence).
- Whether you are the maintainer of the repo (no special weight, just for context).

We will not remove a score because someone disagrees with it — but we will absolutely refine our methodology if there's a credible critique.

---

## Communication

- **GitHub Issues** — bugs, feature requests, methodology disputes.
- **GitHub Discussions** — design conversations, "how would you score X" questions.
- **Pull Requests** — code, docs.

We do not currently have a Slack, Discord, or Matrix room. If we get one, it will be linked from the README.

---

## Recognition

All contributors are listed in `CONTRIBUTORS.md` (auto-generated from git history). Significant contributions are also called out in release notes.

---

## Repository administration

A handful of repository settings can't be expressed in CI workflows because they require admin-level GitHub API access. The ones we care about are scripted under `scripts/`:

- [`scripts/enable-branch-protection.sh`](scripts/enable-branch-protection.sh) — enables branch protection on `main` for the OpenSSF Scorecard Branch-Protection check (required CI status checks, no force-pushes, no deletions). Idempotent. Re-run it whenever the list of required CI checks changes (e.g. new job in `.github/workflows/ci.yml`) so the protection stays in sync. Solo-maintainer compromise: `enforce_admins: false` lets the maintainer keep direct pushes to `main` while still receiving Scorecard credit.

---

## Maintainers

Currently a single maintainer ([@Dmitrze](https://github.com/Dmitrze)). We are actively recruiting co-maintainers — see [`docs/governance.md`](docs/governance.md). The bus factor is currently 1, which is exactly the kind of risk we measure in our own Maintainer Health module. Help us improve it.

---

Thanks for reading this far. The OSS world is better when there are more people willing to dig into the substance of what makes a project trustworthy.
