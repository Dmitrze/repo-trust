# Contributing to Repo Trust

Thanks for thinking about contributing — this project lives or dies by community input. Read this whole file before opening your first PR.

---

## Ground rules

1. **Code of Conduct.** All interactions are governed by the [Contributor Covenant](CODE_OF_CONDUCT.md). No exceptions.
2. **Trust before taste.** This project is about epistemic honesty. If a change makes the tool look better but reduces transparency or determinism, it will be rejected.
3. **Conservative on negative claims.** Anything that increases the risk of false-positive "fake / suspicious" flags requires unusually strong justification.
4. **No surprise dependencies.** New runtime crates need a justification in the PR description. We prefer fewer, well-vetted dependencies.

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
- `cargo fmt` formats. CI fails if `cargo fmt --check` fails.
- `cargo clippy --all-targets --all-features -- -D warnings` must pass.
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
2. The change is right and you need to review the snapshots (`cargo insta review`) and explain in the PR.

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
6. Documentation in `docs/module-specs.md`.

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

## Maintainers

Currently a single maintainer ([@Dmitrze](https://github.com/Dmitrze)). We are actively recruiting co-maintainers — see [`docs/governance.md`](docs/governance.md). The bus factor is currently 1, which is exactly the kind of risk we measure in our own Maintainer Health module. Help us improve it.

---

Thanks for reading this far. The OSS world is better when there are more people willing to dig into the substance of what makes a project trustworthy.
