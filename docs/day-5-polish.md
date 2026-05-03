# Day 5 polish — deferred items from Day 2 architect review

> **Status**: All Day 2 PRs (#22–#27) approved and merged. The items below were flagged as **non-blocking** during architect review and must be picked up during the Day 5 PM `chore/ci-strict-gates` work, before the public release.
>
> **DRI**: @Dmitrze + the Day 5 PM Claude Code session.
>
> **Acceptance for closing this doc**: every checkbox below ticked, then this file is deleted as part of pre-public cleanup.

---

## Critical (blocks strict CI gate)

### 1. `as u64` / `as u8` casts will fail `clippy::pedantic::cast_possible_truncation`

When `RUSTFLAGS="-D warnings"` + `#![warn(clippy::pedantic)]` are re-enabled, these will fail CI. Fix in a single sweep across the four modules + cli:

**`src/features/maintainers.rs`:**
- `let active_maintainers_last_year = by_author.len() as u64;`
- `let total_contributors = ...filter(...).count() as u64;`
- `let n = sorted.len() as i64;` (Gini formula)

**`src/scoring/maintainers.rs`:**
- `((sum + n / 2) / n) as u8` — final-score arithmetic mean
- evidence-display casts on `authors_sorted.len()`

**`src/modules/{activity,maintainers}.rs`:**
- `(ctx.snapshot_at - metadata.created_at).whole_days().max(0) as u64`

**`src/features/security.rs`:**
- `((now - r.date).whole_days().max(0)) as u64`
- `raw.osv_advisories.len() as u64`

**`src/scoring/security.rs`:**
- `score_to_u8`: `(score * 10.0).round().clamp(0.0, 100.0) as u8`
- `count_present_docs`: `.count() as u8`
- `(n * 20).min(100) as u8`
- `(weighted_sum / total_weight).round().clamp(0.0, 100.0) as u8`

**Activity module** (Day 1 carry-over) has the same pattern in `src/scoring/activity.rs` and `src/features/activity.rs`.

**Fix pattern**: prefer `u64::try_from(value).unwrap_or(0)` for runtime-bounded values; use scoped `#[allow(clippy::cast_possible_truncation)]` with a one-line rationale comment when the bound is mathematically guaranteed (e.g. score after `.clamp(0.0, 100.0)` cannot overflow `u8`). Do **not** add a crate-level `allow` — keep it local so future code doesn't get a free pass.

- [ ] All casts replaced with `try_from` or scoped allow
- [ ] `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic` passes clean

---

## Documentation

### 2. `semver_consistent` comment vs behavior mismatch

In `src/features/security.rs`:

```rust
/// True when every non-draft release tag matches `vX.Y.Z` or `X.Y.Z`.
/// Returns true for repos with zero releases (vacuously true).
fn semver_consistent(releases: &[ReleaseMeta]) -> bool {
    let mut any = false;
    for r in releases.iter().filter(|r| !r.draft) {
        any = true;
        if !is_semver_tag(&r.tag_name) { return false; }
    }
    any  // ← returns FALSE when no releases — contradicts comment
}
```

The behavior is **acceptable** (no releases → no full credit for "established semver practice"; scorer assigns Neutral 50/100), but the docstring lies. Two valid fixes:

- **Option A** (preferred — keep current behavior): update the docstring to *"Returns false for repos with zero releases — no track record of semver discipline yet."*
- **Option B** (vacuously-true semantics): change the trailing `any` to `true`.

Pick A unless real-repo benchmark in (5) below suggests B reads more naturally.

- [ ] Reconciled

---

## Test coverage gaps

Wiremock integration tests deferred from Day 2; currently covered only by unit tests or not at all.

### 3. Maintainer Health module
- [ ] **S-001** multi-maintainer healthy fixture (5 active contributors, Gini ≈ 0.3, CODEOWNERS present) → assert `score ≥ 80`, `confidence == High`
- [ ] **S-103** archived demotes to Low + caveat — currently only unit-tested
- [ ] **S-201** contributors endpoint 500 partway through → confidence drops one band, `missing_data["contributors"]`

### 4. Security & Readiness module
- [ ] **S-101** stale Scorecard (30-90 days old) lowers weight to 3.0, `confidence == Medium`
- [ ] **S-102** CODEOWNERS at non-default path detected (`/CODEOWNERS` works when `.github/CODEOWNERS` is 404)
- [ ] **S-201** Scorecard 5xx → real error, CLI exit code 7 (distinct from the 404 fallback path)

---

## Calibration (validate against real-repo benchmark)

### 5. Security federation policy weight semantics

`docs/methodology.md` §Module 5 says "Scorecard ≤30 days old: weight 0.40, confidence contribution High." The shipped implementation in `src/scoring/security.rs`:

```rust
let mut total_weight = 2.0 + 1.0 + 0.5 + 0.5;  // = 4.0 (docs + ci + semver + osv)
if let Some(s) = scorecard_subscore {
    weighted_sum += scorecard_weight * s as f64;  // 4.0 fresh / 3.0 stale
    total_weight += scorecard_weight;
}
```

This gives Scorecard fresh ≈ 50% share of final (4.0 / 8.0), stale ≈ 43% (3.0 / 7.0). The spec's "0.40" / "0.30" is ambiguous — could be absolute weights (current interpretation) or proportional shares.

**Day 5 action**: during the benchmark sweep against ≥10 real repos (prometheus, kubernetes, lodash, requests, axios, react, vue, rust-lang/cargo, fastapi, django) compare module scores against subjective expert-rated buckets. Decision tree:

- **If Security skews systematically away from expected category** → recalibrate the docs / ci / semver / osv absolute weights so Scorecard fresh contributes exactly 40% and stale exactly 30%.
- **If categories match the expected buckets** → leave as-is and update `methodology.md` to clarify that 0.40 / 0.30 are absolute weights, not proportional shares.

- [ ] Decision made + applied (recalibrate or doc-clarify)

---

## Out of scope — track for v1.1, not Day 5

These are **not** Day 5 work — flagged here only to prevent accidental scope-creep into Day 5:

- Maintainer Health: PR-review concentration via per-PR `/pulls/{N}/reviews` endpoint (currently uses `merged_by` + comment counts as proxy)
- Maintainer Health: maintainer responsiveness sub-score (separate spec)
- Security: branch-protection signals (requires admin token scope)

Stars Heuristic 2 (lockstep timing z-score) is **Day 4** work, not v1.1.

---

## Acceptance for closing this doc

When all six checkboxes above are ticked **and** `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic` is clean **and** the benchmark sweep is committed, this file is deleted as part of the pre-public-release cleanup PR.

---

*Created 2026-05-04 during Day 2 architect review. See AI_NATIVE_CONSTITUTION.md §Closed loop for the rationale on tracking deferred work as queryable artefacts rather than verbal hand-offs.*
