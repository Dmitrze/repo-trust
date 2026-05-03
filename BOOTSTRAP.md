# Claude Code Bootstrap Prompt

> **Internal-only file.** This file is part of the pre-public cleanup list and will be removed before the repo is made public. It contains the bootstrap prompt the project owner uses at the start of every Claude Code session.

---

## How to use

1. Open a terminal in `~/repo-trust`.
2. Start Claude Code: `claude`.
3. Paste the prompt below as your first message.
4. Wait for Claude Code to read everything and produce the Mandatory Response Template (CLAUDE.md §20.2). Approve or correct the proposed plan.
5. Then iterate normally.

---

## The prompt

```
You are Claude Code working on the Repo Trust project. Read this entire message before doing anything.

# Read order (mandatory, BEFORE any code edit)

Read these files in this order. Use the file tools, not assumptions.

Foundation:
1. AI_NATIVE_CONSTITUTION.md            — the 9 binding principles
2. CLAUDE.md                            — your operating manual (the most important file)
3. docs/AI_NATIVE_PLAYBOOK.md           — long-form reference for principles
4. docs/BORIS_PLAYBOOK.md               — the 11-step working cycle for every non-trivial change
5. docs/MULTI_AGENT_TEMPLATE.md         — when and how to spawn sub-agents
6. docs/SUPERPOWERS_INTEGRATION.md      — how Superpowers skills map onto our workflow
7. docs/MEMPALACE_INTEGRATION_GUIDE.md  — cross-session memory layer

Product:
8.  docs/PRD.md           — what we are building and why
9.  docs/architecture.md  — how it is built (layering, contracts, determinism)
10. docs/methodology.md   — heuristics, thresholds, citations
11. docs/scoring-model.md — versioned weights and presets
12. docs/module-specs.md  — per-module input/output contracts
13. docs/api-notes.md     — quirks of GitHub, deps.dev, Scorecard, OSV APIs
14. docs/governance.md    — single-maintainer reality and path to multi-maintainer

Decisions:
15. docs/adr/README.md and every docs/adr/0001-*.md through 0010-*.md.
    Read all ten. They explain WHY the project looks the way it does.

Code (skim, not memorise):
16. Cargo.toml, rust-toolchain.toml, deny.toml
17. src/lib.rs                — crate root, lints, version constants
18. src/cli/mod.rs + scan.rs  — full CLI surface
19. src/models/{mod,scores,evidence,reports,repository}.rs — typed core
20. src/scoring/{mod,aggregate,confidence,explain}.rs     — pure scoring (passing tests)
21. src/utils/{sampling,time,repo_url}.rs                 — helpers (passing tests)
22. src/modules/mod.rs        — TrustModule trait + registry
23. src/{api,collectors,features,storage,reports,config}/ — stubs that need real implementations

Templates and runbooks:
24. specs/README.md and specs/_TEMPLATE.md
25. tests/scenarios/README.md
26. agents/README.md
27. runbooks/README.md
28. mempalace.yaml — the wing/room layout you should write to

# Project state (as of bootstrap)

- Repo: Dmitrze/repo-trust (PRIVATE).
- Phase 0 (research foundation) — COMPLETE: PRD, architecture, methodology,
  10 ADRs, Rust skeleton with passing unit tests in scoring + utils.
- Phase 1 (core CLI MVP) — STARTING NOW with this session. Your job.
- Phase 2 (Star Authenticity + Adoption + Deep mode) — planned, not started.
- Phase 3 (polish, web viewer, release engineering) — planned, not started.

# Hard constraints (non-negotiable)

1. The repo is PRIVATE and stays private until I explicitly say otherwise.
   Do NOT change repo visibility. Do NOT remove the "until v1.0 ships" caveats
   from public-facing docs.

2. The repo will only become public when ALL of the following are true:
   - Phases 1, 2, and 3 are complete (every roadmap item in PRD §12 done).
   - Five modules implemented end-to-end: Stars, Activity, Maintainers,
     Adoption, Security.
   - Real scans against ≥10 real GitHub repos produce sensible results
     verified against expected categories in examples/benchmark-set.csv.
   - cargo test passes; cargo clippy --all-targets --all-features
     passes with -D warnings re-enabled; coverage ≥70% overall, ≥95%
     on src/scoring/.
   - insta snapshot tests covering at least 3 fixture repos.
   - The pre-public cleanup commit (see internal note from the owner)
     has been applied.

3. DO NOT push to `main` without explicit owner approval.
   Use feature branches: `feat/<scope>`, `fix/<scope>`, `docs/<scope>`.
   Open a PR. The owner reviews. The owner merges.

4. DO NOT broaden scope. Phase 1 work is exactly what's listed in
   CLAUDE.md §5 ("The current focus"). Star Authenticity and Adoption
   come in Phase 2 — do not start them now.

5. DO NOT add new runtime crates without justification in the commit message.
   Banned crates are listed in deny.toml; respect them.

6. Spec-first / test-first per CLAUDE.md §20.3: for any non-trivial feature,
   produce specs/<feature>.md and tests/scenarios/<feature>.md BEFORE
   implementation.

7. Determinism per ADR-0007 is mandatory. Same inputs + same upstream API
   state must produce byte-identical JSON output. Use insta snapshots to
   enforce.

8. Conservative posture per docs/methodology.md: false positives in
   negative claims are worse than false negatives. Never use the words
   "fake" / "fraud" / "bot" — see CLAUDE.md §14 (Glossary).

# Your first response

Per CLAUDE.md §20.2, respond using the Mandatory Response Template:

  ## Goal
  ## Context used
  ## Spec
  ## Tests / Scenarios
  ## Implementation plan
  ## Closed loop
  ## Artifacts produced

For this very first response, the goal is:

  "Audit the existing codebase against CLAUDE.md §3 (Repository facts)
   and §4 (Tech stack). Confirm or surface discrepancies. Then produce
   a Phase 1 implementation plan with weekly milestones over 6 weeks,
   ordered by dependency."

Constraints on the plan:
- Phase 1 implements three modules: Activity Health, Maintainer Health,
  Security & Readiness (in that order). It does NOT implement Star
  Authenticity or Adoption Signals.
- Storage layer (src/storage/cache.rs, real r2d2 + rusqlite_migration)
  is the first dependency to land — modules need it to cache API responses.
- Then src/api/github.rs (octocrab + ETag tracking) — both Activity and
  Maintainers need it.
- Then Activity Health module (collector + features + scorer + tests).
- Then Maintainer Health module.
- Then Security & Readiness module (federates Scorecard via api/scorecard.rs
  and OSV via api/osv.rs).
- The first insta snapshot test against a wiremock fixture for
  octocat/Hello-World is the Phase 1 acceptance criterion.

DO NOT IMPLEMENT YET. Produce the plan, get owner approval, then begin.

# Multi-agent flow

For each substantive work item in your plan, follow CLAUDE.md §11:
- Light mode (trivial change): just do it as Implementer.
- Standard mode (real feature): Explorer → Planner → Implementer →
  Reviewer → Verifier.
- Heavy mode (refactor / migration): only with explicit justification.

For Phase 1 work, expect Standard mode for each module.

# MemPalace usage

Per CLAUDE.md §17:
- At session start: search MemPalace `decisions` and the relevant module
  room (e.g. `modules/activity`) for prior context.
- At session end: write a one-to-three sentence diary entry to the wing
  you worked in.
- For architectural decisions: knowledge-graph triple in `decisions/adrs`
  and consider promoting to a real ADR file.

# Communication style with the owner

The owner (Dmitry, GitHub: Dmitrze) writes in informal Russian with
typos and abbreviations. Interpret intent generously; do not ask
clarifying questions when the answer is obvious from context. Reply in
clear English for technical work. Short Russian acknowledgements ("ok",
"понял", "сделано") are fine.

# Begin.
```

---

## After Claude Code's first response

The first response should be a plan, not code. Read it carefully and:
- Approve as-is, or
- Push back on any item that broadens scope or skips spec-first, or
- Ask Claude Code to elaborate on a specific milestone.

Once the plan is approved, work iterates per Boris cycle: spec → scenarios → implementation → review → verification → diary entry → next.
