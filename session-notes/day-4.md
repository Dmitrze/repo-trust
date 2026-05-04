# Day 4 — Lockstep + 3 reports + web viewer + cache CLI (2026-05-04)

## What landed

8 atomic commits, ~5 200 net new lines, 206 → 266 tests:

1. `docs/day-4-specs` — 6 specs (star-authenticity-lockstep / reports-terminal / reports-markdown / reports-csv / web-viewer / cache-subcommands) + paired scenarios.
2. `feat/module-stars-lockstep` (main worktree) — Heuristic 2 z-score (28-day rolling baseline lagged 7 days, max daily z-score). Final formula reverts to methodology v1 weights (`0.55 × H1 + 0.30 × H2 + 0.15 × H3`); falls back to Day-3 redistribution when H2 unavailable. New `combined_low_activity_and_lockstep` Concerning evidence when both H1 ≥ 20% AND H2 ≥ 5. **Day-3 architect Q1 follow-through executed**: `recency_biased_sample` Neutral evidence on every non-below-floor run; `specs/star-authenticity-module-shallow.md` §9 amended with the recency-bias caveat. 6 features + 6 scorer unit tests.
3. `feat/reports-terminal` (parallel agent) — comfy-table + console color (Strong=green, …, HighRisk=red); ANSI suppression when piped. 11 unit tests + 1 insta snapshot.
4. `feat/reports-markdown` (parallel agent) — long-form GFM with module sections + evidence tables + methodology footer. Plain `writeln!` formatting; `pulldown-cmark` added to `[dev-dependencies]` only for round-trip validation. 7 unit tests + 1 insta snapshot.
5. `feat/reports-csv` (parallel agent) — fixed 21-column row per repo; RFC-4180-ish quoting; `csv` added to `[dev-dependencies]` only. 11 unit tests + 1 insta snapshot.
6. `feat/web-viewer` (parallel agent) — axum app on `127.0.0.1:8765`; askama templates + rust-embed for static assets; single-binary preserved. `POST /scans` gated behind `--allow-scan` (DNS-rebinding mitigation). New `Cache::list_all_reports`. 12 tests; `cargo build --no-default-features` still succeeds.
7. `feat/cli-cache-subcommands` — `cache info|clear|prune` replace the Day-0 stub. New `Cache::clear_api_cache` / `Cache::clear_all` / `Cache::prune_expired`. 4 unit + 5 binary integration tests.
8. `feat/scan-pipeline-day4` — `cli::scan::execute::resolve_formats()` precedence chain; `--format json,md,csv` produces all 3 files. New `tests/scan_format_dispatch.rs` integration test.

## Decisions

- **Lockstep verdict ceiling stays Concerning** even when combined H1+H2 evidence is emitted. Methodology requires both signals before lowering the module score band — but the FINAL score drops via the weighted formula, not the verdict.
- **Architect Q1 (cargo build --no-default-features in CI matrix)**: deferred to Day 5 PM `chore/ci-strict-gates`.
- **Architect Q2 (markdown empty-top_strengths placeholder)**: leave as-is — asymmetry is deliberate (`top_strengths` = stable anchor; `top_concerns` empty = positive default).
- **Architect Q3 (POST /scans synchronous)**: leave as v1 behavior — viewer is "developer-laptop only" per architecture §12.

## Blockers / Friction

- Stars test had to accommodate the new `lockstep_z_score: Option<f64>` field on `StarsFeatures`; existing `baseline()` test fixture extended.
- The `suspicious_profile_lowers_score_to_concerning_not_highrisk` test had its score expectation re-baselined from `≤30` to `≤35` because the Day-4 formula `0.55×20 + 0.30×30 + 0.15×~50` lands at ~28-32 (depending on H3 ratio scores).
- 4 parallel worktree subagents (terminal + markdown + csv + web) total 3 100s wall-clock; integration via stacked-rebase resolved 3 predictable Cargo.toml + CHANGELOG conflicts.

## Numbers

- 8 commits, +5 200 lines.
- 206 → 266 tests (+60).
- 8 PRs (#33-#40) opened end-of-day.
