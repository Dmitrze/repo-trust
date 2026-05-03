# 0010 — Plugin system deferred to v1.2

## Status
Accepted (May 2026)

## Context

A plugin system would let third parties:
- Add new modules (a sixth, seventh, ...).
- Add new collectors (e.g. GitLab support).
- Add new report formats (e.g. Excel).

We could build this in v1.0. Doing so would:
- Force us to commit to a stable plugin ABI before we have one user of it.
- Introduce complexity in the registry, lifecycle, and security model.
- Risk locking us into a design that turns out to be wrong.

## Decision

The `TrustModule` trait is **defined in code** for v1.0, but we **do not advertise or stabilize a plugin API** until v1.2. The five built-in modules are statically linked.

When v1.2 ships a plugin API, we will:
1. Use the `inventory` crate or feature-flag-gated entry-point mechanism for module registration.
2. Document the trait, lifecycle, and security model in `docs/plugin-api.md`.
3. Provide a `repo-trust-plugin-template` repository.
4. Apply SemVer to the plugin API independently from the CLI version.

## Consequences

### Easier
- v1.0 ships sooner.
- We learn from real users before committing to a plugin contract.
- The internal `TrustModule` trait can evolve freely between v1.0 and v1.2.
- Security model is easier to reason about with no third-party code.

### Harder
- Users who want custom modules must fork the repo (acceptable for v1.0; no real demand yet).
- We will eventually need to refactor for plugin extraction; the current design anticipates this (modules are already independent under the trait).

## Alternatives considered

### Ship plugin system in v1.0
**Why considered:** "Open from the start" is a strong principle.

**Why rejected:** We have no plugin authors yet. Designing a plugin API without users always produces a wrong design.

### WebAssembly plugin sandbox
**Why considered:** Strongest security and portability story.

**Why rejected:** Premature; significant engineering cost. Reconsider in v2 if there's demand for sandboxed third-party plugins.

### Lua scripting embed
**Why considered:** Easier scripting story.

**Why rejected:** Doesn't fit our "compiled, fast, deterministic" posture. If we ever want a scripting layer, we'd revisit.
