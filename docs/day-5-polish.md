# Day 5 polish — deferred items from Day 2 + Day 3 + Day 4 architect review

> **Status**: All Day 2 PRs (#22–#27), Day 3 PRs (#28–#32), and Day 4 PRs (#33–#40) approved and merged. The items below were flagged as **non-blocking** during architect review and must be picked up during the Day 5 PM `chore/ci-strict-gates` work, before the public release.
>
> **DRI**: @Dmitrze + the Day 5 PM Claude Code session.
>
> **Acceptance for closing this doc**: every checkbox below ticked, then this file is deleted as part of pre-public cleanup.

---

## Critical (blocks strict CI gate)

### 1. `as u64` / `as u8` casts will fail `clippy::pedantic::cast_possible_truncation`

When `RUSTFLAGS="-D warnings"` + `#![warn(clippy::pedantic)]` are re-enabled, these will fail CI. Fix in a single sweep across all five modules + cli:

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

**`src/features/stars.rs` (Day 3 + Day 4):**
- `(now - raw.repo_metadata.created_at).whole_days().max(0) as u64`
- `forks as f64 / total as f64` (cast OK — not truncating, but pedantic may want `f64::from(forks)`)
- `Some(matches as f64 / raw.sampled_profiles.len() as f64)`
- **Day 4 lockstep**: `Vec::with_capacity(span_days.unsigned_abs() as usize + 1)` — span_days is i64, abs as usize. Pedantic OK but may warn on platforms with 32-bit usize.

**`src/scoring/stars.rs` (Day 3 + Day 4):**
- `((u32::from(fork_score) + u32::from(watcher_score)) / 2) as u8` (clamped, mathematically safe)
- `(frac * 100.0).round().clamp(0.0, 100.0) as u8` (clamped, safe)
- `raw.round().clamp(0.0, 100.0) as u8` (clamped, safe — used 3× now: H1+H3, H1+H2+H3, and H2+H3 fallback formulas)

**Fix pattern**: prefer `u64::try_from(value).unwrap_or(0)` for runtime-bounded values; use scoped `#[allow(clippy::cast_possible_truncation)]` with a one-line rationale comment when the bound is mathematically guaranteed (e.g. score after `.clamp(0.0, 100.0)` cannot overflow `u8`). Do **not** add a crate-level `allow` — keep it local so future code doesn't get a free pass.

- [ ] All casts replaced with `try_from` or scoped allow
- [ ] `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic` passes clean

### 1b. CI matrix: add `cargo build --no-default-features` (Day 4 Q1)

Per Day 4 EOD Q1 — owner accepted the default option (yes, add Day 5 PM). The web-viewer feature (`src/cli/serve.rs`, `src/web/`) is opt-in via `[features] web`; if a user disables default features they should still get a working `repo-trust scan` binary. The Day 4 agent already verified `cargo build --no-default-features` succeeds locally; Day 5 PM should make this an explicit CI matrix entry.

- [ ] Extend CI matrix from `[ubuntu-latest, macos-latest] × [default-features]` to also include `[--no-default-features]`. Adds ~3× runner minutes for high signal-to-noise (catches feature-gating regressions).

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

Pick A unless real-repo benchmark in (8) below suggests B reads more naturally.

- [ ] Reconciled

### 2b. Hangover STUB comment in `src/models/repository.rs` (Day 3)

After PR #29 + #30 + #32 merged, the `deps_dev` field on `RepositoryContext` is no longer a stub — it's a real `DepsDevClient`. The doc comment still says:

```rust
/// STUB: scan-pipeline-day3 wires this real. Carries the deps.dev
/// federated client that the Adoption Signals module consumes.
pub deps_dev: DepsDevClient,
```

Fix: remove the `STUB:` lead-in. Update to: *"Carries the deps.dev federated client that the Adoption Signals module consumes (per ADR-0012)."*

- [ ] Reconciled

### 2c. Markdown snapshot fixture timestamp comment lies (Day 4)

In `tests/reports_markdown_snapshot.rs::pinned_snapshot_at()`:

```rust
fn pinned_snapshot_at() -> OffsetDateTime {
    // 2026-05-04T10:23:45Z — same value as the spec example.
    OffsetDateTime::from_unix_timestamp(1_777_796_625).unwrap()
}
```

The unix timestamp `1_777_796_625` is actually **2026-05-03T08:23:45Z**, not the comment's 2026-05-04T10:23:45Z. The committed snapshot file (`tests/snapshots/reports_markdown_snapshot__inactive_baseline_5_modules.snap`) is consistent with the actual timestamp — only the comment lies. Fix: either update the comment to match the actual unix timestamp, or update the timestamp + regenerate the snapshot to match the commented date.

- [ ] Reconciled

### 2d. `let _ = writeln!(...)` discard pattern (Day 4)

`src/reports/markdown_report.rs` uses `let _ = writeln!(out, ...)` ~50 times. Writing to a `&mut String` cannot fail (capacity grows), so this is idiomatic; but `clippy::pedantic::let_underscore_must_use` may warn under strict gates. Consider scoping `#[allow(clippy::let_underscore_must_use)]` to the module top, or replace with a small `wln!` macro that handles the discard internally.

- [ ] Reconciled

---

## Test coverage gaps

Wiremock integration tests deferred; currently covered only by unit tests or not at all.

### 3. Maintainer Health module (Day 2)
- [ ] **S-001** multi-maintainer healthy fixture (5 active contributors, Gini ≈ 0.3, CODEOWNERS present) → assert `score ≥ 80`, `confidence == High`
- [ ] **S-103** archived demotes to Low + caveat — currently only unit-tested
- [ ] **S-201** contributors endpoint 500 partway through → confidence drops one band, `missing_data["contributors"]`

### 4. Security & Readiness module (Day 2)
- [ ] **S-101** stale Scorecard (30-90 days old) lowers weight to 3.0, `confidence == Medium`
- [ ] **S-102** CODEOWNERS at non-default path detected (`/CODEOWNERS` works when `.github/CODEOWNERS` is 404)
- [ ] **S-201** Scorecard 5xx → real error, CLI exit code 7 (distinct from the 404 fallback path)

### 5. Adoption Signals module (Day 3)
- [ ] **Real S-001** popular-package wiremock fixture (deps.dev returns 1 GO package with `weeklyDownloads: "100000"`, README + docs/ + examples/ all present) → assert `score ≥ 75`, `confidence == High`. The current `s001_well_documented_repo_runs_end_to_end` test uses an empty deps.dev response and effectively covers the no-packages branch end-to-end. Real S-001 still missing.
- [ ] **S-002** minimal-but-documented (deps.dev 1k downloads + docs/ but no examples/) → assert `score ∈ [40, 70]`
- [ ] **S-102** explicit deps.dev outage scenario (5xx from deps.dev) → confidence Low, `missing_data["deps_dev_unavailable"]`
- [ ] **S-103** archived repo (currently unit-only) → integration test
- [ ] **S-201 (standalone)** README 404 standalone (with packages present) → Concerning `no_readme` evidence + downloads sub-score still computed

### 6. Star Authenticity module (Day 3 + Day 4)

The `tests/all_five_modules_integration.rs` end-to-end test (PR #32) covers Stars module wiring through the binary CLI. However, the fixture provides only **2 stargazers** with the **same date**, which is below `min_sample_for_medium_confidence (30)` AND below the 35-day window required for lockstep z-score — so Stars confidence is always Low and `lockstep_z_score = None` in that test. This validates structure, not score magnitude or H2 path.

- [ ] **S-001** organic-profile fixture with ≥100 stargazers (mostly healthy profiles) → assert `score ≥ 80`, `confidence == High`
- [ ] **S-002** suspicious-profile fixture with ≥100 stargazers (38%+ matching low-activity composite) → assert `score ≤ 30`, verdict `Concerning` (NOT `HighRisk`)
- [ ] **S-101** tiny repo (<50 stars) → score 0, confidence Low, `missing_data["below_sampling_floor"]`
- [ ] **S-102** new repo (<6 months) leniency applied → 5pp shift visible in evidence rationale
- [ ] **Lockstep H2** wiremock fixture: starred_at series spanning ≥35 days with both smooth (z<3) and bursty (z≥5) variants → asserts `lockstep_z_score` sub-score appears, formula uses 0.55/0.30/0.15 weights, `combined_low_activity_and_lockstep` evidence emitted when both thresholds met
- [ ] **S-401** determinism (same seed → byte-identical scores) — already enforced by `tests/aggregate_determinism.rs` for the broader scan; add Stars-specific seed-stability assertion if benchmarks show drift
- [ ] **S-501** language posture — already covered in unit test `rationale_uses_only_probabilistic_phrasing_no_fake_fraud_bot`; spot-check still required against real-repo output Day 5

### 7. Stars: `recency_biased_sample` Neutral evidence — ✅ **DONE in PR #34 (Day 4)**

Per the Day 3 EOD Q1 → architect option (a) decision, both items shipped:

- [x] `recency_biased_sample` Neutral evidence emitted on every non-below-floor run in `src/scoring/stars.rs` (with unit test `recency_biased_evidence_emitted_on_every_non_below_floor_run`)
- [x] `specs/star-authenticity-module-shallow.md` §9 amended with the 19-line caveat paragraph (PR #34)

### 8. Web viewer wiremock integration (Day 4)

`tests/web_viewer_integration.rs` covers the happy path (S-001/S-002/S-103) and the in-handler tests cover S-101 (empty cache) + S-102 (404) + S-502 (POST /scans → 405 без --allow-scan). Missing:

- [ ] **S-501** explicit test that `is_localhost_bind` warning fires on `0.0.0.0:8765` start (smoke test of the warn path)
- [ ] **POST /scans happy path** when `--allow-scan` is on — deferred because spec §3 marks the synchronous-blocking behavior as v1 (Day 4 Q3 default a). Day 5 only if the benchmark sweep surfaces issues.

### 9. CSV writer `\r` carriage return scenario (Day 4)

`escape_csv_handles_carriage_return` covers the unit case; missing an integration scenario where `\r` appears in a real evidence value. Low-priority because no current evidence emitter produces `\r` — purely defensive.

- [ ] Optional: add a contrived scenario or document that `\r` handling is unit-tested only.

---

## Calibration (validate against real-repo benchmark)

### 10. Security federation policy weight semantics

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

### 11. Adoption download bands logarithmic vs linear

`src/scoring/adoption.rs` uses logarithmic banding (1k → 25, 10k → 50, 100k → 75, 1M → 100). `specs/adoption-signals-module.md` §9 marks this as v1 with "tune in v1.1 if benchmark says".

**Day 5 action**: during the same benchmark sweep, look for mid-popularity packages (10k–100k downloads) that score 50 but feel intuitively like 60–70. If pattern reproduces, narrow the 50/75 band breakpoints. Decision goes in `methodology.md` Module 4 change log.

- [ ] Benchmark verdict captured

### 12. Stars lockstep z-score band calibration (Day 4)

`src/scoring/thresholds.rs::StarsThresholds::v1()::lockstep_score_bands` uses `[(3.0, 100), (5.0, 85), (8.0, 60), (12.0, 30), (∞, 10)]` per methodology v1. The bursty-pattern threshold (z ≥ 5 → 60) was set conservatively; the >12 band drops to 10 (almost zero). Real-world bursty-but-legitimate cases (HN front page, "Show HN", podcast mentions) can hit z ~10 naturally without being suspicious.

**Day 5 action**: during the benchmark sweep, identify any HN-bursty repos mis-classified by H2; if pattern is significant, soften the >12 band from 10 → 0 (or widen the 8-12 band) and document in methodology.md change log. The combined H1+H2 condition (both ≥ 20% AND z ≥ 5) is the more reliable signal than H2 alone.

- [ ] Benchmark verdict captured (calibrate or document)

---

## Out of scope — track for v1.1, not Day 5

These are **not** Day 5 work — flagged here only to prevent accidental scope-creep:

- Maintainer Health: PR-review concentration via per-PR `/pulls/{N}/reviews` endpoint (currently uses `merged_by` + comment counts as proxy)
- Maintainer Health: maintainer responsiveness sub-score (separate spec)
- Security: branch-protection signals (requires admin token scope)
- Adoption: GitHub `/dependents` HTML scrape — brittle, opt-in flag in v1.1 if user demand surfaces
- Adoption: Docker Hub pulls — Phase 2
- Stars: full uniform random sub-sampling from a larger pool (Phase 2 deep mode)
- Stars: deep-mode graph signal (co-starring overlap with known campaign clusters) — Phase 2+
- **Web viewer: async POST /scans** with job queue + `GET /scans/{id}` polling endpoint (Day 4 Q3 deferred per default option a). Synchronous behavior is fine for v1 "developer-laptop only" use case (architecture.md §12). Add only if user demand surfaces.
- **Web viewer: SARIF output format** — Day 4 PR #40 wires the `Format::Sarif` enum variant с `tracing::warn!("SARIF output deferred to v1.1; skipping")`. Implement когда user demand surfaces.
- **Web viewer: live re-scan progress via WebSockets** — Day 5+ if value is clear.

---

## Acceptance for closing this doc

When all checkboxes above are ticked **and** `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic` is clean **and** `cargo build --no-default-features` succeeds in CI matrix **and** the benchmark sweep is committed, this file is deleted as part of the pre-public-release cleanup PR.

---

*Created 2026-05-04 during Day 2 architect review. Updated 2026-05-05 with Day 3 items. Updated 2026-05-06 with Day 4 items + section 7 marked DONE. See AI_NATIVE_CONSTITUTION.md §Closed loop for the rationale on tracking deferred work as queryable artefacts rather than verbal hand-offs.*
