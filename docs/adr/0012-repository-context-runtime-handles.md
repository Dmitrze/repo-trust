# 0012 — `RepositoryContext` carries runtime handles for v1

## Status
Accepted (2026-05-04)

## Context

`docs/architecture.md` §1 (Layering rules) states:

> - `collectors` may import from `api`, `storage`, `models`, `utils` — never from `modules` or `scoring`.
> - `scoring` is pure (no I/O).
> - `models` is the type vocabulary; …

Implicit in that ordering is that `models` does not depend on `storage` or `api` — it is a leaf type vocabulary.

The shipped Day 1 `RepositoryContext` (in `src/models/repository.rs`) violates that: it now carries `cache: storage::Cache` and `github: api::github::Client` fields. `TrustModule::run(ctx)` is the single entry point for every module, and modules need both handles.

Two alternatives were available:

1. **Wrapper type in a higher layer.** Define `runtime::RunContext { repo: RepositoryContext, cache, github, … }` and pass that into `TrustModule::run` instead of the bare `RepositoryContext`. Models stays pure; runtime composes I/O handles.
2. **Trait-object indirection.** Define `trait CacheLike` and `trait GithubLike` in `models` with the methods modules consume; storage/api implement them. `RepositoryContext` carries `Arc<dyn CacheLike + Send + Sync>` etc. Models stays leaf-pure.

Both add ceremony. For a 5-day Phase 1+2+3 sprint with one reviewer and one binary consumer, the ceremony has no payoff.

## Decision

`RepositoryContext` carries concrete runtime handles for v1:

```rust
pub struct RepositoryContext {
    pub full_name: String,
    pub canonical_url: url::Url,
    pub mode: CliMode,
    pub scoring_version: semver::Version,
    pub weights: ModuleWeights,
    pub rng_seed: u64,
    pub snapshot_at: OffsetDateTime,

    // Runtime handles — v1 layering exception, see this ADR.
    pub cache: crate::storage::Cache,
    pub github: crate::api::github::Client,
    // Day 2: + scorecard
    // Day 2: + osv
    // Day 3: + deps_dev
}
```

The cache and clients are all `Clone + Send + Sync` (Arc-internal), so passing the context around is cheap.

`docs/architecture.md` §1 layering rule is amended to read: *"models may import from storage and api solely for the runtime-handle fields on `RepositoryContext`. All other model types remain leaf types."* The amendment lands in this commit.

## Consequences

### Easier
- Single struct flows through the trait surface. No wrapper type to maintain.
- Modules access cache + GitHub client through `ctx.cache` / `ctx.github` with zero boilerplate — matches `module-specs.md` examples.
- New runtime handles (Scorecard, OSV, deps.dev) are one-line additions on Day 2/3.
- No `Arc<dyn …>` indirection; concrete types preserve compile-time guarantees and inlining.
- Tests construct contexts with real or fake clients via the same struct literal.

### Harder
- The `models` crate now transitively depends on `reqwest`, `r2d2`, `rusqlite`. A future "publish a tiny `repo-trust-models` crate" plan would have to split these fields out (this is the post-v1.0 forward path; see below).
- Documentation drift risk between architecture.md §1 (now amended) and contributors' first reading of the trait surface.

### Trade-offs explicitly accepted
- We accept the layering exception for the duration of v1.x in exchange for zero wrapper-type ceremony.
- We accept that publishing a separate `repo-trust-models` crate (for third-party report consumers) would require splitting runtime handles out of `RepositoryContext`. There is no such consumer in the v1 roadmap.

## Forward path (post-v1.0)

If any of the following materialise, refactor into `runtime::RunContext`:

1. **Plugin system (v1.2 per ADR-0010).** Third-party modules will need a stable `RepositoryContext` ABI that does not bundle storage + reqwest implementation choices.
2. **Separate `repo-trust-models` crate.** If we split out a tiny dependency-light crate so external dashboards can deserialize `TrustReport` without pulling rusqlite + reqwest, runtime handles must move out of `models`.
3. **Library use case.** If a host application embeds `repo_trust` as a library and wants to provide its own cache implementation, the trait-object indirection (alternative 2 above) becomes worth the cost.

Until at least one of these lands, the shipped shape stays.

## Alternatives considered

### Refactor to `runtime::RunContext` wrapper before v1.0 ship
**Why considered:** Cleaner architecture; preserves models leaf-purity.

**Why rejected:** Pure refactor work on a 5-day budget, no observable user-facing benefit, no second consumer of `models` exists. Worth it only if (1)-(3) above appear.

### Trait-object indirection (`Arc<dyn CacheLike>`)
**Why considered:** Decouples models from concrete I/O implementations.

**Why rejected:** Adds dyn dispatch on every cache call, requires defining trait surfaces that mirror `Cache` exactly, and the only consumer is `TrustModule::run`. Over-engineering for the v1 use case.

### Pass cache + clients as separate parameters to `run()`
**Why considered:** Keeps `RepositoryContext` pure.

**Why rejected:** Breaks the `TrustModule::run(ctx)` trait surface; either trait grows or each method takes a different parameter set. Both worse than the layering exception.

## Migration note

`docs/architecture.md` §1 is amended in the same commit as this ADR. Future ADRs that touch `RepositoryContext` should reference this one.
