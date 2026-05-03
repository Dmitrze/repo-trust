# Repo Trust — Development Requirements

Everything you need to run, build, and contribute to Repo Trust.

---

## 1. System requirements

- **macOS 13+**, **Linux** (glibc 2.31+), or **Windows 11** (native or WSL2).
- **Rust** stable 1.75 or newer (install via [rustup](https://rustup.rs/)).
- **Git** 2.40+.
- **Python 3.9+** — only required if you use **MemPalace** as a developer tool. Not required to build, test, or run `repo-trust` itself.
- A **GitHub account** with read access to `Dmitrze/repo-trust`.

### Required third-party service accounts

| Service | Purpose | Where to get keys |
| --- | --- | --- |
| GitHub | Read public repo metadata, stargazer profiles, contributors, releases. **Required for any non-trivial scan.** | <https://github.com/settings/tokens> — a fine-grained PAT with no scopes works for public repos |

The following are **upstream public APIs we federate** — no account needed unless you exceed anonymous rate limits:
- deps.dev (no auth)
- scorecard.dev (no auth)
- osv.dev (no auth)

If you contribute to the `release` workflow you'll also interact with:
- crates.io (Trusted Publisher / OIDC, no API token in the repo)
- ghcr.io (uses `GITHUB_TOKEN` from the workflow)

---

## 2. Runtime dependencies

See `Cargo.toml` for the authoritative list. Highlights:

| Crate | Why |
| --- | --- |
| `clap` v4 | CLI parsing, `--help`, completions |
| `tokio` | Async runtime |
| `reqwest` (rustls) | HTTP client — **no openssl** |
| `octocrab` | GitHub REST + GraphQL |
| `serde` + `serde_json` | Serialization |
| `rusqlite` (bundled) | Local cache |
| `r2d2` + `r2d2_sqlite` | Connection pool for batch mode |
| `tracing` + `tracing-subscriber` | Structured logging |
| `figment` | Layered config (TOML + env + CLI) |
| `time` | Timestamps with serde |
| `rand_chacha` | Deterministic RNG for sampling |
| `blake3` | Stable hashing for cache keys + RNG seeds |
| `axum` (`web` feature) | Optional localhost viewer |
| `petgraph` (`deep` feature) | Graph analysis in deep mode |

## 3. Dev dependencies

| Crate | Use |
| --- | --- |
| `insta` | Snapshot tests for deterministic output |
| `proptest` | Property tests for scoring functions |
| `wiremock` | HTTP mocking for collector integration tests |
| `tempfile` | Temporary directories in tests |
| `assert_cmd` + `predicates` | CLI integration tests |
| `criterion` | Benchmarks |

Quality tools (run in CI):
- `cargo fmt`
- `cargo clippy`
- `cargo-tarpaulin`
- `cargo-deny`
- `cargo-audit`

---

## 4. Approved additions

Libraries pre-approved to add as needed:

| Crate | Why |
| --- | --- |
| `tower` | If we need middleware composition beyond `tower-http` |
| `metrics` | If we expose Prometheus-style metrics from `serve` |
| `notify` | If we add file-watch on cache for the web viewer |

Approval criterion: small footprint, actively maintained, solves a real problem the standard library doesn't.

---

## 5. Explicitly banned libraries

| Banned | Use this instead |
| --- | --- |
| `openssl` / `openssl-sys` | `rustls` (already in `reqwest`) |
| `native-tls` | `rustls` |
| `chrono` | `time` (we standardized on `time` for serde + features) |
| `lazy_static` | `std::sync::OnceLock` (or `once_cell::sync::Lazy` if MSRV blocks) |
| `error-chain` | `thiserror` for libraries, `anyhow` for the binary |
| Any unmaintained crate | Pick an alternative |

Enforcement: `deny.toml` blocks `openssl*` and `native-tls`.

---

## 6. MemPalace (optional, developer tool)

MemPalace gives Claude Code persistent memory across sessions. It is **not** a runtime dependency of Repo Trust — only used by humans + Claude Code working on the repo.

```bash
pipx install mempalace
mempalace init <path-to-project>
mempalace mine . --mode convos
```

MemPalace data lives in `./.mempalace` (gitignored). Structure is in `mempalace.yaml`. See `docs/MEMPALACE_INTEGRATION_GUIDE.md`.

---

## 7. Superpowers (Claude Code plugin)

Superpowers is a one-time install per developer machine, not per-project.

```text
# In Claude Code:
/plugin marketplace add obra/superpowers-marketplace
/plugin install superpowers@superpowers-marketplace
```

Full mapping of Superpowers skills onto this project's workflow: `docs/SUPERPOWERS_INTEGRATION.md`.

---

## 8. Installation

```bash
# 1. Clone
git clone https://github.com/Dmitrze/repo-trust.git
cd repo-trust

# 2. Install Rust toolchain (rustup will pick up rust-toolchain.toml)
rustup component add rustfmt clippy

# 3. Build
cargo build --all-features

# 4. Set up env
cp .env.example .env
# ...edit .env and put your GitHub PAT in GITHUB_TOKEN

# 5. Run tests
cargo test --all-features

# 6. Install the binary locally (optional)
cargo install --path .

# 7. Verify
repo-trust --help
repo-trust scan octocat/Hello-World --mode quick
```

---

## 9. Cargo commands cheat sheet

| Command | What it does |
| --- | --- |
| `cargo build` | Debug build |
| `cargo build --release` | Release build (LTO, stripped) |
| `cargo test` | All tests |
| `cargo test --doc` | Doctests only |
| `cargo test scoring::aggregate` | Single module |
| `cargo fmt --all` | Format |
| `cargo fmt --all -- --check` | CI-style check |
| `cargo clippy --all-targets --all-features -- -D warnings` | Lint |
| `cargo doc --all-features --no-deps --open` | Build and view rustdoc |
| `cargo bench` | Run benchmarks |
| `cargo tarpaulin --all-features --workspace` | Coverage |
| `cargo deny check` | License + advisory + source check |
| `cargo audit` | RustSec advisory check |
| `cargo install --path .` | Install local build to `~/.cargo/bin/` |

---

## 10. Branching & commit conventions

- `main` → always releasable.
- Feature branches: `feat/<short-name>` off `main`.
- Fix branches: `fix/<short-name>`.
- Doc-only branches: `docs/<short-name>`.
- **Conventional Commits required:** `feat:`, `fix:`, `refactor:`, `docs:`, `chore:`, `test:`, `ci:`, `perf:`.
- **Never** commit `.env*` files (other than `.env.example`).
- **Never** commit real API keys or PATs.

---

## 11. Questions / blockers

If a service account, API key, or credential is blocking work, log it in MemPalace under `ops` room and notify via your preferred channel. Do not fabricate values or hardcode placeholders that will leak into commits.
