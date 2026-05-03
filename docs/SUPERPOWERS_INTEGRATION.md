# Superpowers Integration Guide

> **What this is.** A mapping from the [obra/superpowers](https://github.com/obra/superpowers) Claude Code plugin's seven core skills onto the AI-native foundation flow (specs / scenarios / agents / runbooks / multi-agent roles).
>
> **Audience.** Claude Code, after Superpowers is installed.
>
> **Prerequisite.** Superpowers installed via `/plugin install superpowers@superpowers-marketplace` (one-time, per-machine).

---

## Why integrate at all

Superpowers and the AI-native foundation pack solve overlapping problems in compatible ways. Superpowers ships *behavior* (auto-triggered skills like `brainstorming`, `writing-plans`, `test-driven-development`); the AI-native pack ships *structure* (specs/, tests/scenarios/, agents/, runbooks/, constitution, playbook). They are complementary:

- Superpowers tells Claude Code *when* to brainstorm, *when* to write a plan, *when* to run TDD, *when* to do code review.
- The AI-native pack tells Claude Code *where the artifact goes*, *what shape it has*, *what the binding rules are*.

The integration: Superpowers' default outputs (a brainstorming session, a plan, a TDD cycle) land in the AI-native pack's directories (`specs/`, `tests/scenarios/`, `CHANGELOG.md`, `runbooks/`).

---

## The mapping

### `brainstorming` -> `/specs/<feature>.md`

**Superpowers behavior.** Activates before code. Refines a rough idea via Socratic questions. Explores alternatives. Presents the design in sections for validation. Saves a design document.

**AI-native binding.** The output of a `brainstorming` session is the input to a new spec. After the session ends, Claude Code:

1. Opens `specs/_TEMPLATE.md`.
2. Fills the Goal, Non-functional requirements, Boundaries, Happy-path scenario, and Architecture sketch sections from the brainstorming output.
3. Saves to `specs/<feature>.md` with frontmatter `status: draft` (until the user reviews) -> `proposed` -> `accepted`.
4. Surfaces the spec for human approval *before* moving to plans.

**Anti-pattern.** Treating the Superpowers brainstorming-session document as the spec itself. The spec lives in `/specs/`, follows the template, and is git-tracked.

### `writing-plans` -> `/tests/scenarios/<feature>.md` + implementation plan

**Superpowers behavior.** Activates with an approved design. Breaks work into 2-5-minute tasks. Each task has exact file paths, complete code, verification steps. Emphasizes RED-GREEN-REFACTOR TDD, YAGNI, DRY.

**AI-native binding.** A Superpowers plan has two natural homes in this repo:

1. **Scenarios** (the verification steps) -> `tests/scenarios/<feature>.md`. Each verification step in the plan becomes a Given/When/Then scenario with a stable `S-NNN` ID.
2. **Implementation plan** (the per-task breakdown) -> the Response Template's `Implementation plan` section in Claude Code's reply (per `CLAUDE.md` section 20.2). This stays in chat / commit history; it is not a separate file.

**Anti-pattern.** Letting the plan replace the scenarios file. The plan is per-engagement; the scenarios file is per-feature and lives forever.

### `subagent-driven-development` / `executing-plans` -> the multi-agent roles

**Superpowers behavior.** Dispatches fresh subagents per task with two-stage review (spec compliance, then code quality), or executes in batches with human checkpoints.

**AI-native binding.** This *is* the multi-agent template at runtime. The mapping:

| Superpowers concept | Multi-agent template role |
|---|---|
| Subagent dispatching tasks | Orchestrator |
| Subagent executing one task | Implementer |
| Two-stage review: spec compliance | Reviewer |
| Two-stage review: code quality | Reviewer (second pass) or Verifier |
| "Pre-flight" investigation before a complex task | Explorer |
| Decomposition before dispatching subagents | Planner |

Use Superpowers as the runtime mechanism; use the multi-agent template (`docs/MULTI_AGENT_TEMPLATE.md`) as the role definitions and reporting formats.

### `test-driven-development` -> implementation against scenarios

**Superpowers behavior.** Enforces RED-GREEN-REFACTOR per task: write a failing test, watch it fail, write minimal code, watch it pass, commit. Deletes any code written before its test.

**AI-native binding.** RED-GREEN-REFACTOR runs against the scenarios in `tests/scenarios/<feature>.md`. Each scenario translates into one or more concrete test cases (Vitest, Playwright, Braintrust eval). The TDD loop is per-test-case; the scenario is per-behavior. One scenario can spawn 1-N test cases; that's fine.

**Closed loop.** A scenario is "green" when its test cases pass. A feature is "green" when all scenarios are green. Both states are visible in CI and in PR descriptions.

### `requesting-code-review` -> Reviewer agent output

**Superpowers behavior.** Activates between tasks. Reviews against the plan; reports issues by severity. Critical issues block progress.

**AI-native binding.** The output of a Superpowers code review *is* the Reviewer agent's reporting format from `docs/MULTI_AGENT_TEMPLATE.md` section 9: overall verdict; critical problems; medium risks; small notes; what must be fixed before completion; what can be accepted now.

If the Superpowers review surfaces critical issues, the Implementer fixes; the Reviewer re-reviews; only then merge.

### `using-git-worktrees` + `finishing-a-development-branch` -> branch hygiene

**Superpowers behavior.** `using-git-worktrees` creates an isolated workspace on a new branch with a clean test baseline. `finishing-a-development-branch` verifies tests, presents merge / PR / keep / discard options, cleans up the worktree.

**AI-native binding.** This matches the project's branching convention from `REQUIREMENTS.md` section 10:

- `main` -> production.
- `develop` -> staging (if applicable).
- Feature branches: `feat/<short-name>` off `develop` (or `main`).

Use `using-git-worktrees` to create a `feat/<feature>` branch when starting a new spec implementation. Use `finishing-a-development-branch` to merge or PR when scenarios are green and the Reviewer is satisfied. Always update `CHANGELOG.md` as part of the finishing step.

### `systematic-debugging` + `verification-before-completion` -> Verifier role

**Superpowers behavior.** `systematic-debugging` runs a 4-phase root-cause process. `verification-before-completion` ensures the fix is *actually* fixed, not assumed-fixed.

**AI-native binding.** This is the Verifier agent's job from `docs/MULTI_AGENT_TEMPLATE.md` section 5.6.

The output of `verification-before-completion` lands in two places:

1. The PR description — the verification block in the Implementer's reply.
2. (For incidents) `runbooks/<scenario>.md` post-incident section — if the bug exposed a runbook gap, the runbook is updated.

---

## End-to-end flow on one feature

1. **User asks Claude Code for the feature.** (Trivial / non-trivial check applies.)
2. **`brainstorming` activates** (Superpowers). Socratic Q&A. Output: design document.
3. **Claude Code writes `specs/<feature>.md`** from the brainstorming output. Status: `draft`. Surfaces to user for approval.
4. **User approves the spec.** Status: `accepted`.
5. **`writing-plans` activates** (Superpowers). Output: a plan with per-task breakdown and verification steps.
6. **Claude Code writes `tests/scenarios/<feature>.md`** from the plan's verification steps. Stable `S-NNN` IDs.
7. **`using-git-worktrees` activates** (Superpowers). Branch `feat/<feature>` created. Clean baseline confirmed.
8. **`subagent-driven-development` activates** (Superpowers). Multi-agent template roles assumed.
9. **For each scenario: `test-driven-development` activates.** RED-GREEN-REFACTOR. Implementer writes code in narrow scope.
10. **`requesting-code-review` activates** between tasks. Reviewer output in the format from `MULTI_AGENT_TEMPLATE.md`.
11. **`verification-before-completion` activates** at the end. Verifier confirms real pass / fail with evidence.
12. **`finishing-a-development-branch` activates.** PR opened, references `specs/<feature>.md` in description.
13. **Claude Code adds `CHANGELOG.md` entry**, writes MemPalace diary entry in the relevant room.
14. **Merge after Reviewer green.**

The entire flow is one Claude Code session (or a small set of coordinated sessions per the multi-agent template). The user reviews at steps 4, 11, and 12. Everything else is automated.

---

## What to do if Superpowers is not installed

The AI-native flow still works without Superpowers. The cost: skills do not auto-trigger; the user must explicitly ask for brainstorming, planning, code review, etc. The structural rules (specs first, scenarios first, response template, artifacts mandatory) still apply because they live in `CLAUDE.md` section 20, which is binding regardless.

**Recommendation.** Install Superpowers. The ritual cost is one command per machine; the velocity gain is permanent.

---

## Updating Superpowers

```text
/plugin update superpowers
```

When you update, scan the new release notes (`obra/superpowers/RELEASE-NOTES.md`) for any changes that affect this mapping. If a skill is renamed, deprecated, or has a materially different output shape, update this guide accordingly and add a `CHANGELOG.md` entry.
