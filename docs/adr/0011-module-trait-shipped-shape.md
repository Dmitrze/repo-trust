# 0011 — TrustModule trait: shipped object-safe `run()` shape

## Status
Accepted (2026-05-03)

## Context

`docs/architecture.md` §4 originally described the `TrustModule` trait as a generic four-method contract — `collect`, `compute_features`, `score`, `explain` — with separate associated `RawData` and `Features` types. The doc noted that real code would split this into a GAT-style typed trait plus an object-safe `dyn DynTrustModule` for the registry.

The shipped skeleton (`src/modules/mod.rs`, May 2026) takes a simpler path: a single object-safe trait with one async `run()` method that returns `(ModuleResult, Vec<EvidenceItem>)`. Each module implementation owns its own `RawData` / `Features` types as private types — they never appear in the trait surface.

The five-day Phase 1+2+3 sprint forced the question: refactor to the GAT-split shape, or keep the simpler one and amend the doc?

## Decision

Keep the shipped object-safe trait. Amend `docs/architecture.md` §4 to match. The trait surface in `src/modules/mod.rs` is the v1 contract:

```rust
#[async_trait]
pub trait TrustModule: Send + Sync {
    fn name(&self) -> &'static str;
    fn version(&self) -> &'static str;
    async fn run(
        &self,
        ctx: &RepositoryContext,
    ) -> anyhow::Result<(ModuleResult, Vec<EvidenceItem>)>;
}
```

Each module implementation pipelines collector → features → scorer → explainer **internally**, exposing none of those intermediate types to callers. The pipeline stages remain real, testable units (in `src/collectors/<m>.rs`, `src/features/<m>.rs`, `src/modules/<m>.rs`); they are simply not part of the trait surface.

## Consequences

### Easier
- Object-safe trait → `Vec<Box<dyn TrustModule>>` registry works directly, no DynTrustModule erasure layer needed.
- Adding a new module is one `impl TrustModule` plus internal helpers — no associated types to wire through the registry.
- Plugin system reservation (ADR-0010) lands cleanly: a future plugin only needs to implement this single object-safe trait.
- `JoinSet<(ModuleResult, Vec<EvidenceItem>)>` for parallel module execution is straightforward, no GAT lifetime dance.

### Harder
- The trait surface gives no compile-time guarantee that a module implements all four pipeline stages — a module could in principle return hard-coded data from `run()`. We mitigate by convention: each module's `run()` body is a four-line pipeline, and each module's tests exercise all four stages independently.
- Less elegant for documentation purposes; we lose the "the trait *is* the pipeline" symmetry.

### Trade-offs explicitly accepted
- We accept weaker compile-time enforcement of pipeline structure in exchange for a vastly simpler trait surface and zero registry erasure overhead.
- We accept that the architecture document drifted from the code by ~2 weeks; the patch to §4 lands in the same commit as this ADR.

## Alternatives considered

### Refactor to GAT-split typed trait + object-safe erasure layer
**Why considered:** Matches original architecture.md §4 design; provides compile-time pipeline enforcement.

**Why rejected:** ~2 days of refactor work on a 5-day total budget for v1.0. Code churn ratio is poor — the existing object-safe trait works for every Phase 1+2+3 use case, and the GAT shape is purely for documentation aesthetics. Plugin system is deferred to v1.2 anyway; a future GAT migration is possible if plugin demand justifies it.

### Hybrid: keep object-safe trait at registry boundary; expose pipeline-as-trait for testing
**Why considered:** Best of both worlds.

**Why rejected:** Two trait hierarchies for the same concept doubles maintenance and confuses contributors. The pipeline stages are already independently testable as plain functions in `collectors::<m>::collect()`, `features::<m>::compute()`, `modules::<m>::score()`. No trait needed to enforce that.

## Migration note

`docs/architecture.md` §4 is patched in the same commit as this ADR. The text "this is split across two parts: a generic `TrustModule<RawData, Features>` GAT-style trait that each module implements with concrete types, and an object-safe `dyn DynTrustModule` registry trait that erases the types for the orchestrator" is replaced with a description of the shipped one-method trait and its internal pipeline convention.
