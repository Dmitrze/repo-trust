# 0002 — CLI framework: clap v4

## Status
Accepted (May 2026)

## Context

The CLI is the primary user surface. It must support:
- Subcommands (`scan`, `batch`, `explain`, `serve`, `cache`, `config`, `version`, `completions`).
- Many flags per subcommand, with environment-variable fallbacks.
- Excellent `--help` output (we lose users on bad help text).
- Shell completion generation for bash, zsh, fish, powershell.
- Stable binary size (clap is heavy by default).

## Decision

Use **`clap` v4 with the `derive` feature**, plus `clap_complete` for shell completions.

## Consequences

### Easier
- Type-safe argument parsing — args become `struct` fields with attributes.
- Auto-generated, well-formatted `--help` and `--version`.
- `clap_complete` produces completion scripts for all major shells.
- Excellent ergonomics for env-var fallbacks: `#[arg(long, env = "GITHUB_TOKEN")]`.
- The de facto standard — contributors recognize it instantly.

### Harder
- Adds ~200KB to the binary (acceptable; clap is essential UX).
- The derive API has a steep first hour for contributors unfamiliar with it.

## Alternatives considered

### `argh` (Google)
**Why considered:** Very small binary footprint; simple derive API.

**Why rejected:** Less mature; weaker `--help` rendering; no built-in completions story; smaller community.

### `pico-args`
**Why considered:** Minimal dependency footprint.

**Why rejected:** Manual parsing; no derive; no auto-generated help; not viable for the surface area we need.

### Hand-rolled
**Why considered:** Maximum control; minimum dependency.

**Why rejected:** We would re-implement clap badly. Not worth the time.
