# Day 3 — Stars + Adoption + 5-module registry (2026-05-05)

## What landed

5 atomic commits, ~3 425 net new lines, 147 → 206 tests:

1. `docs/day-3-specs` — 3 specs (deps-dev-client / adoption-signals-module / star-authenticity-module-shallow) + paired scenarios.
2. `feat/api-deps-dev` (parallel agent) — federated client over `api.deps.dev/v3` (project_packages + package endpoints), 24h TTL, custom string→u64 deserializer for `weeklyDownloads`. 5 wiremock + 6 in-module unit tests.
3. `feat/module-adoption` (parallel agent) — federates deps.dev for downloads + GitHub README maturity (presence + word count) + docs/ + examples/ probes. Logarithmic download bands. Conservative posture: no published package = Medium + Neutral caveat. New `github::Client::get_readme` with in-tree base64 decoder. **Adoption agent stubbed `src/api/deps_dev.rs` and added the `deps_dev: DepsDevClient` field to `RepositoryContext` ahead of the deps-dev branch landing — coordination via a documented STUB marker that gets discarded on rebase.** 16 scorer + 6 features + 4 base64 + 2 wiremock integration tests.
4. `feat/module-stars-shallow` (main worktree) — Heuristic 1 (9-signal low-activity composite, 6-band table) + Heuristic 3 (fork/watcher ratios + ecosystem multipliers per `module-specs.md`). Day-3 formula: `0.55 × H1 + 0.45 × H3` (lockstep H2 deferred Day 4). Below-floor short-circuit (<50 stars). New `github::Client::get_user`. Verdict ceiling Concerning; probabilistic phrasing only. 11 scorer + 8 features tests.
5. `feat/scan-pipeline-day3` — `cli::scan::execute::select_modules()` expanded to all 5 modules. New `tests/all_five_modules_integration.rs` end-to-end test.

## Decisions

- **Stars shallow**: Heuristics 1 + 3 ship Day 3 with a redistributed weight (`0.55 × H1 + 0.45 × H3`) until lockstep H2 lands Day 4. The weight redistribution is a Day-3 specific choice documented in the spec; Day 4 reverts to `0.55 / 0.30 / 0.15`.
- **Architect Q1 follow-through deferred to Day 4**: `recency_biased_sample` Neutral evidence + `specs/star-authenticity-module-shallow.md` §9 caveat ride the lockstep branch.
- **Architect Q2**: `bail!()` cleanup limited to modules; CLI subcommand stubs (`serve` / `cache` / `batch` / `config` / `explain`) are Day 4 / v1.1+ scope.
- **Architect Q3 MemPalace**: still disconnected; falls back to plain markdown session-notes per the agreed plan (this file).

## Blockers / Friction

- The deps-dev/adoption coordination via STUB marker worked but required careful conflict resolution on rebase (real `deps_dev.rs` from sibling branch, drop the stub).
- Aggregate determinism integration test broke briefly when the new modules added evidence — fixed by sorting evidence by `(module, code)` before serialization (already in place; the test caught a regression in earlier diff).

## Numbers

- 5 commits, +3 425 lines.
- 147 → 206 tests (+59).
- 5 PRs (#28-#32) opened end-of-day.
