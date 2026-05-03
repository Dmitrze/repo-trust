# 0006 — Five trust modules

## Status
Accepted (May 2026)

## Context

Any composite trust score has to be decomposed into intelligible parts. Too few modules and the score is opaque; too many and the report is overwhelming. We needed a small set of modules that:
- Are each independently meaningful to a user.
- Have minimal overlap.
- Can be enabled/disabled independently.
- Map cleanly onto questions a user actually asks.

## Decision

Five modules, in fixed order in the report:

1. **Star Authenticity** — "Are the popularity signals organic?"
2. **Activity Health** — "Is the repo alive?"
3. **Maintainer Health** — "Is stewardship sustainable?"
4. **Adoption Signals** — "Is it actually used in the wild?"
5. **Security & Readiness** — "Is it ready for production use?"

Default weights (v1.0): 20% / 25% / 20% / 20% / 15%. Configurable via `--weights` and `weights.toml`.

## Consequences

### Easier
- Each module maps to one user question.
- Module boundaries align with collector/feature/scorer pipelines, simplifying the codebase.
- Users can disable expensive modules (Star Authenticity in deep mode) for fast scans.
- Adding a sixth module requires an ADR — a high bar that prevents bloat.

### Harder
- Some signals span modules: `SECURITY.md` exists → affects both Maintainer Health (governance maturity) and Security & Readiness (security posture). We assign it to one module (Security) and document the choice in `docs/module-specs.md`.
- The 25% weight on Activity Health is a value judgment that not everyone will share; we document the rationale and make it configurable.

## Alternatives considered

### Three modules (Health / Trust / Security)
**Why considered:** Simpler.

**Why rejected:** "Health" is too broad to be actionable; users want to know separately whether a repo is alive vs whether one person owns it.

### Seven or eight modules (e.g. separating Releases, CI, Documentation)
**Why considered:** More granular.

**Why rejected:** Reports become harder to scan; many sub-signals are best aggregated. We expose them as `sub_scores` within modules instead.

### Modules per ecosystem (Rust crate, npm package, PyPI package, Maven artifact)
**Why considered:** Some signals are ecosystem-specific.

**Why rejected:** Modules should be questions, not ecosystems. Ecosystem-specific behavior is a feature of the collector, not a module boundary.
