# 0001 — Language: Rust

## Status
Accepted (May 2026)

## Context

Repo Trust ships as a single CLI binary that:
- Performs concurrent HTTP requests to GitHub, deps.dev, OpenSSF Scorecard, and OSV.
- Parses untrusted JSON from third-party APIs.
- Caches data locally in SQLite.
- Optionally serves a localhost web UI.
- Will run in CI pipelines belonging to other organizations.
- Aspires to qualify for the GitHub Secure Open Source Fund.

Language choice affects performance, distribution ergonomics, security posture, contributor pool, and the project's perception in the modern OSS dev-tool category.

## Decision

Use **Rust 2021 edition** with MSRV `1.75`.

## Consequences

### Easier
- Single static binary distribution (no runtime needed on user machines).
- `cargo install repo-trust` is a one-liner that works across Linux/macOS/Windows.
- Cross-compilation via `cross` for arm64 Linux and Apple Silicon.
- Memory-safe by default — a meaningful posture for a security-adjacent tool.
- Sum types (`enum`) + exhaustive matching make the `TrustModule` contract genuinely safe to extend.
- `serde` provides zero-cost JSON serialization with compile-time schema enforcement.
- `tokio` async runtime is mature and well-suited to coordinated concurrent HTTP.
- Strong fit with the 2026 "modern dev-tool" category (uv, ruff, biome, bun, ripgrep, fd, bat, gitui, hyperfine, tokei).

### Harder
- Smaller contributor pool than Go or Python.
- Compile times longer than Go for clean builds (mitigated by incremental builds and CI caching).
- Dependency on stable async-fn-in-traits (since Rust 1.75) raises our MSRV.

### Trade-offs explicitly accepted
- We accept a smaller contributor pool in exchange for stronger compile-time guarantees.
- We accept slower clean builds in exchange for runtime performance and binary size.
- We accept higher MSRV (1.75) in exchange for clean trait design.

## Alternatives considered

### Go
**Why considered:** OpenSSF Scorecard and deps.dev are written in Go. `go install` is excellent. Build times are fast. Larger pool of cloud-native contributors.

**Why rejected:** Go interfaces lack the exhaustiveness guarantees of Rust enums, which matters for our module registry. Go's lack of sum types makes the explainability layer messier. Go's GC introduces tail latencies that complicate the deterministic-runtime goal. We also evaluated the perception axis: Rust is the lingua franca of the modern dev-tools category in 2026; Go retained more of the cloud-native infra category.

### Python
**Why considered:** Largest contributor pool. Rich data-analysis ecosystem (pandas, numpy). Fast for prototyping.

**Why rejected:** Slower per request, requires a runtime on user machines, weaker static guarantees, worse first-run UX (`pip install` vs `cargo install` or downloaded native binary). For a tool whose value proposition includes "runs in CI in seconds," the runtime overhead is meaningful. We are happy to consume Python OSS infrastructure (e.g. MemPalace as a developer tool) but the shipped binary is not Python.

### Zig
**Why considered:** Smallest binaries; manual memory management with no GC; growing reputation.

**Why rejected:** Pre-1.0 instability; thin async ecosystem; small contributor pool.

### TypeScript / Node.js
**Why considered:** Familiar to many web developers; npm package distribution.

**Why rejected:** Heavy runtime dependency; package-management nightmare; weaker static guarantees than Rust; the "native CLI tool" category has decisively moved away from Node since ~2024.
