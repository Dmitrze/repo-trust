# <PROJECT_NAME> — Claude Code Operating Manual

> **This file is the single source of truth for how Claude Code operates on this repository.** Read it before every working session. If anything here conflicts with older docs in `docs/`, this file wins.
>
> **And:** this file is itself downstream of `AI_NATIVE_CONSTITUTION.md` (repo root). If anything here conflicts with the constitution, the constitution wins and this file gets updated.
>
> **First-time use of this template:** sections 1-8, 14, and 18 contain placeholders in `<ANGLE_BRACKETS>`. Fill these in your first session against the new project. Sections 11-13, 15-17, 19-21 are generic and stay as-is unless the project genuinely needs to override them.

---

## 1. The product in one paragraph

<ONE-PARAGRAPH PRODUCT DESCRIPTION. Who is it for, what does it do, what is it not, what's the business shape (managed service / SaaS / open source / internal tool / etc.), what's the rough revenue model (if any), what's the target sales cycle and onboarding time.>

---

## 2. Non-negotiables (never violate)

<List of project-specific hard rules. Examples for a TypeScript-React-Supabase product would include items like: TypeScript strict no `any`; Tailwind only; every tenant-scoped table has RLS; no secrets in frontend; etc. Keep this list short — 5-12 items max. Each rule should be a binary check the reviewer agent can apply.>

The last item, regardless of project, is always:

```
13. **AI-native by constitution.** Every workflow is closed-loop. Every action produces an artifact. No code without a spec. No feature without scenarios. No outcome without a DRI. (Full rules: `AI_NATIVE_CONSTITUTION.md` and `docs/AI_NATIVE_PLAYBOOK.md`.)
```

Renumber to whatever the last index actually is.

---

## 3. Repository facts

- **Repo:** `<owner>/<n>` (private/public).
- **Default branch:** `main`.
- **Live URL:** `<production URL or N/A>`.
- **Deploy:** `<Render / Vercel / Cloudflare / Fly.io / N/A>`.
- **State:** `<frontend X% complete; backend Y% complete; etc.>`.
- **Top-level directories:** `brand/` (if any), `docs/`, `public/` (if web), `scripts/`, `src/`, `specs/`, `tests/`, `agents/` (if LLM agents are part of the product), `runbooks/`.

---

## 4. Tech stack (exact versions)

<Fill from the project's `package.json` / `pyproject.toml` / `Cargo.toml` / etc. List runtime + dev dependencies + approved additions + banned libraries. See `REQUIREMENTS.md` for fuller setup detail.>

---

## 5. The current focus / the pivot (if applicable)

<If the project is mid-pivot or has a strong constraint on what's in scope right now, write it here. Otherwise delete this section.>

---

## 6. Pricing & packaging (if applicable)

<If the project is a commercial product with tiers, list them with a one-line description per tier. Otherwise delete this section.>

---

## 7. Surfaces / audiences

<List the major user-facing surfaces (e.g., marketing site, client app, ops console, admin) with their auth role and a one-line purpose. Otherwise delete.>

---

## 8. Agents (if applicable)

If this project ships LLM agents as part of the product, list them here. Each gets a directory under `agents/<n>/` with prompt, policy, tools, threshold, eval-set, changelog. See `agents/README.md`.

If this project does not include LLM agents, delete this section and the `agents/` directory.

<List or "N/A".>

---

## 9. Architecture direction (if applicable)

<Short ASCII layer-cake diagram or bullet list. Don't duplicate `docs/Technical Blueprint.md` if it exists; link to it.>

---

## 10. Routes / surfaces inventory (if applicable)

<Keep / Transform / Kill / Add lists for major routes if the project is mid-restructure. Otherwise delete.>

---

## 11. Multi-agent orchestration for Claude Code

We use the **multi-agent master template** from `docs/MULTI_AGENT_TEMPLATE.md`. Critical-review that document at the start of any major engagement; do not apply it blindly.

### Default roles

- **Orchestrator** — the main Claude Code session. Holds the goal, sets plan, decides when to spawn other agents, composes the final result. Never implements big blocks directly if delegation is cleaner.
- **Explorer** — separate session. Investigates unknown parts of the codebase or third-party APIs before implementation. Returns a focused report on where to change and what risks exist.
- **Planner** — separate session. Decomposes a multi-step change into a sequence with dependencies. Returns an ordered step list with parallelizable branches flagged.
- **Implementer** — separate session, may be multiple in parallel on independent files. Writes the code in a narrow scope. Does not touch files outside the stated scope.
- **Reviewer** — separate session. Fresh-context review of the diff. Looks for logic errors, missed edge cases, security issues, accidental scope creep, spec mismatches.
- **Verifier** — separate session. Runs the code, checks typecheck, runs manual smoke tests. Reports real (not assumed) pass/fail with evidence.
- **Documenter** — *optional*. Used only when documentation, handoff, changelog, or decision write-ups are a real bottleneck. Otherwise documentation is part of the Implementer's Definition of Done.

### Mode selection

- **Light mode (small task / clear change):** Orchestrator + Implementer + Reviewer.
- **Standard mode (real feature):** Orchestrator + Explorer + Planner + Implementer + Reviewer + Verifier.
- **Heavy mode (migration / refactor / multi-epic):** standard + multiple Implementers in parallel + optional Documenter.

Do not default to heavy. Justify the upgrade.

### When to spawn parallel sessions

Yes if: independent features on different files; isolated experiments; independent review; pre-flight exploration of a third-party API; report generation vs. implementation.

No if: tightly coupled tasks; shared-file edits; anything that would create merge conflicts; the coordination cost outweighs the speed gain.

### Reporting format from sub-agents to Orchestrator

- **Explorer:** goal, what inspected, findings, risks, recommended next step.
- **Planner:** goal, decomposition, dependency order, parallelizable branches, Definition of Done.
- **Implementer:** what changed, files touched, out-of-scope items, decisions made that reviewer should double-check.
- **Reviewer:** overall verdict, critical issues, medium risks, minor nits, blockers to merge.
- **Verifier:** what ran, what passed, what failed, confidence level.

Full rationale and edge cases: `docs/MULTI_AGENT_TEMPLATE.md`.

---

## 12. Workflow cycle (Boris Cherny playbook)

For every non-trivial change, run the 11-step Boris cycle. Full version: `docs/BORIS_PLAYBOOK.md`. Compressed:

1. **Goal in one or two sentences** — who benefits, what pain solved, success signal.
2. **One concrete happy-path scenario** — step by step, who does what.
3. **3-7 modules** — architectural blocks, not individual functions.
4. **Constrain the stack and the iteration** — say what is *out* of scope.
5. **Skeleton first** — file structure and entry points before logic.
6. **Short iterations** — 5-20 minute loops; run after each; paste errors into the next prompt.
7. **Code as conversation** — always include current file contents and the exact error/goal.
8. **Definition of Done** — tests for the scenario, small functions, names that explain themselves.
9. **README / inline doc updated** — part of Implementer's done criteria.
10. **Verify hallucinations** — confirm library versions and API shapes against actual installed packages or docs.
11. **Checkpoint prompt every ~1 day or major feature** — "here's what exists, here's what's slow, here are the next 2-3 iterations".

---

## 13. Definition of Done (applies to every task)

A change is "done" only if all of these hold:

- Type-checks with zero errors (e.g., `pnpm build` passes the `tsc -b` step, or `mypy --strict` passes, or whatever the project's equivalent is).
- No `any` / no untyped escape hatches introduced.
- Tests pass (unit + scenario-driven E2E where relevant).
- The happy-path scenario from section 12 step 2 runs end-to-end against a real environment.
- No secrets in the diff.
- Accessible: keyboard navigation works, color contrast meets WCAG AA (for any UI work).
- Mobile works (320px width minimum) (for any UI work).
- README or relevant doc in `docs/` reflects the change.
- A short note written to MemPalace diary for the relevant wing.
- Committed with a Conventional Commit message (`feat:`, `fix:`, `refactor:`, `docs:`, `chore:`, `test:`).
- **Spec exists in `/specs/<feature>.md` and scenarios in `/tests/scenarios/<feature>.md`.**
- **For LLM features, the probabilistic satisfaction threshold is met (see `agents/<agent>/threshold.md`).**
- **A `CHANGELOG.md` entry is added (or generated by the post-merge agent).**

---

## 14. Glossary (use this vocabulary)

<Project-specific deprecated -> preferred terms. Otherwise delete.>

---

## 15. Quality standard — Boil the Ocean

The marginal cost of completeness is near zero with AI. Do the whole thing. Do it right. Do it with tests. Do it with documentation. Do it so well that the founder is genuinely impressed — not politely satisfied, actually impressed. Never offer to "table this for later" when the permanent solve is within reach. Never leave a dangling thread when tying it off takes five more minutes. Never present a workaround when the real fix exists. The standard isn't "good enough" — it's "holy shit, that's done." Search before building. Test before shipping. Ship the complete thing. When the founder asks for something, the answer is the finished product, not a plan to build it. Time is not an excuse. Fatigue is not an excuse. Complexity is not an excuse.

---

## 16. Review Gate

Every diff is read critically by a senior reviewer (human or Reviewer agent or Codex) who will not accept hand-waving. Leave a clean trail: commit messages explain why, code explains how, Definition of Done items are visibly satisfied.

---

## 17. MemPalace usage

This project uses MemPalace for cross-session memory. See `docs/MEMPALACE_INTEGRATION_GUIDE.md` for setup and `mempalace.yaml` for the wing/room layout.

- **At session start:** run `mempalace_list_agents` and search the relevant wing/room for prior context before implementing.
- **At session end:** write a diary entry for the wing you worked in. One to three sentences: what was done, what decisions were made, what's next.
- **For architectural decisions:** add a knowledge-graph triple in the `decisions` room with rationale.
- **For stale info:** invalidate graph entries when facts change — do not delete; MemPalace keeps history.
- **Do not dump everything into CLAUDE.md.** This file stays lean; long-tail context lives in the palace.

---

## 18. Development phases priority

<Project-specific roadmap summary. Reference `docs/Development Roadmap.md` if it exists. Otherwise list current sprint goal.>

---

## 19. Where to find everything

- **The constitution:** `AI_NATIVE_CONSTITUTION.md` (root) — 9 principles + decision checklist.
- **AI-native playbook:** `docs/AI_NATIVE_PLAYBOOK.md` — long-form reference.
- **Boris workflow cycle:** `docs/BORIS_PLAYBOOK.md` — 11-step working method.
- **Multi-agent template:** `docs/MULTI_AGENT_TEMPLATE.md` — master template for multi-agent setup.
- **Superpowers integration:** `docs/SUPERPOWERS_INTEGRATION.md` — how the Claude Code plugin maps to this stack.
- **MemPalace guide:** `docs/MEMPALACE_INTEGRATION_GUIDE.md`.
- **Specs (per feature):** `specs/<feature>.md` — see `specs/README.md` for format.
- **Scenarios (per feature):** `tests/scenarios/<feature>.md` — see `tests/scenarios/README.md`.
- **Agents (prompts, policies, tools, thresholds):** `agents/<agent-name>/` — see `agents/README.md`.
- **Runbooks:** `runbooks/<scenario>.md` — see `runbooks/README.md`.
- **Closed-loop inventory:** `docs/Closed Loops Inventory.md` — living state of every project process.
- **Token budget:** `docs/Token Budget.md` — weekly token-vs-outcomes ritual.
- **Founder discipline:** `docs/Founder Discipline.md` — weekly conviction-check ritual.
- **CHANGELOG:** `CHANGELOG.md` — agent-maintained record of shipped changes.
- **Project-specific docs:** `<list any docs/ files specific to this project>`.

If a doc in `docs/` contradicts this file, this file wins. If `AI_NATIVE_CONSTITUTION.md` contradicts this file, the constitution wins.

---

## 20. AI-Native Operating Principles — binding for Claude Code

This section translates `AI_NATIVE_CONSTITUTION.md` into operational rules Claude Code follows on every task.

### 20.1 Read order at session start

At the start of every session, in this order:

1. `AI_NATIVE_CONSTITUTION.md` (root) — the nine principles, the decision checklist.
2. This file (`CLAUDE.md`).
3. `docs/AI_NATIVE_PLAYBOOK.md` if the task spans more than one principle.
4. `docs/BORIS_PLAYBOOK.md` if launching a new feature or major engagement.
5. `docs/MULTI_AGENT_TEMPLATE.md` if planning anything that involves more than one Claude Code session.
6. `docs/SUPERPOWERS_INTEGRATION.md` to remember which Superpowers skill belongs to which phase.
7. The relevant `specs/<feature>.md` if the task touches an existing feature.
8. The relevant `agents/<agent-name>/` directory if the task touches an LLM agent.
9. MemPalace `decisions` and `sessions` rooms for prior context.

### 20.2 Mandatory Response Template (every non-trivial task)

Before writing implementation code, every substantive task is answered with this structure. Trivial tasks (typos, single-line CSS tweaks, comment edits) are exempt.

```
## Goal
<one sentence: who benefits, what changes, success signal>

## Context used
<what is in the agent's context for this task: files read, docs referenced,
MemPalace rooms searched. What is missing and how it was retrieved.>

## Spec
<link or path to /specs/<feature>.md — created if it didn't exist>

## Tests / Scenarios
<link or path to /tests/scenarios/<feature>.md — created if it didn't exist>

## Implementation plan
<ordered steps; named DRI; out-of-scope items called out;
file-level breakdown of changes>

## Closed loop
<what metric is measured by this change; where it appears
on a dashboard; how feedback returns to the next iteration>

## Artifacts produced
<list of every file/record this task creates or updates,
including spec, scenarios, code, CHANGELOG entry, MemPalace entries>
```

### 20.3 Spec-first / Test-first rule

For any feature touching backend, agent prompts, agent tools, or substantial frontend logic:

1. **Write the spec.** `/specs/<feature>.md` — use `/specs/_TEMPLATE.md` as the starting point.
2. **Write the scenarios.** `/tests/scenarios/<feature>.md` — use `/tests/scenarios/_TEMPLATE.md`.
3. **Then implement.** Code is downstream of spec and scenarios.
4. **Then judge against the spec.** PR review reads the spec first, the diff second.

If the user requests implementation without a spec, the agent first proposes the spec for approval, then implements. The agent does not silently proceed without a spec for non-trivial work.

### 20.4 Artifact rule

Every action that affects the project leaves a queryable artifact. Specifically:

- A code change leaves a PR with a description that references the spec.
- A decision leaves a MemPalace knowledge-graph triple.
- A merge leaves a `CHANGELOG.md` entry.
- A spec change leaves a commit on `/specs/<feature>.md`.
- A scenario change leaves a commit on `/tests/scenarios/<feature>.md`.
- An agent prompt change leaves a commit on `agents/<agent>/prompt.md`.
- A new internal process leaves an entry in `docs/Closed Loops Inventory.md`.
- A weekly token cycle leaves an entry in `docs/Token Budget.md`.

If the agent cannot point to the artifact a task produced, the task isn't done.

### 20.5 Context parity check (mandatory before non-trivial tasks)

Before starting any non-trivial task, the agent answers:

1. What does an agent need to know to do this well? (List.)
2. What is in my context now? (List.)
3. What is missing? (List.)
4. How do I close the gap? (Plan: project_knowledge_search, file reads, asking the user, etc.)

If the gap is non-zero and unclosed, the agent does not start. "Try and see" is forbidden.

### 20.6 No middleware proposals

The agent never proposes:
- A new manager or coordinator role.
- A weekly status meeting whose output isn't an artifact.
- A multi-step approval chain when one DRI would suffice.
- A "sync to align" call with no decision artifact.

The agent does propose:
- A new agent in `agents/`.
- A direct IC <-> DRI link.
- A scheduled job that produces an artifact.
- A dashboard that surfaces the state instead of meeting about it.

### 20.7 Token-max default

When the agent has a choice between (a) more agent capability + better artifact + higher token cost vs (b) less capability + thinner artifact + lower cost — it picks (a) by default and notes the spend in the response. Cost is justified by what shipped, not minimized for its own sake.

### 20.8 Founder-in-the-loop

For any decision that materially shapes strategy, AI tooling choice, or process design — the agent surfaces the decision to the founder for explicit confirmation, even if it has a strong recommendation.

### 20.9 DRI rule

Every substantive plan or proposal names exactly one DRI for each outcome. "DRI: TBD" is a flag; it blocks merge until resolved. "DRI: the team" is forbidden.

### 20.10 Closed-loop check (every new feature or process)

Every feature or process satisfies the Definition of Done from `docs/AI_NATIVE_PLAYBOOK.md` section 3.2:

1. Goal in writing.
2. Action produces a machine-readable artifact.
3. Artifact lands in the index.
4. An agent reads it.
5. A metric is measured automatically.

If any are missing, the design is incomplete and the agent says so.

---

## 21. Superpowers integration

This project assumes the [obra/superpowers](https://github.com/obra/superpowers) Claude Code plugin is installed on the developer's machine. Superpowers adds seven mandatory skills that map cleanly onto the AI-native flow. Full mapping in `docs/SUPERPOWERS_INTEGRATION.md`. Compressed:

| Phase in this stack | Superpowers skill | Output |
|---|---|---|
| Goal + scope discovery | `brainstorming` | Conversation that becomes `/specs/<feature>.md` |
| Plan decomposition | `writing-plans` | Plan that becomes `/tests/scenarios/<feature>.md` |
| Parallel sub-agent execution | `subagent-driven-development` or `executing-plans` | Implementer / Reviewer / Verifier sessions |
| Implementation | `test-driven-development` | RED-GREEN-REFACTOR per scenario |
| Code review | `requesting-code-review` | Reviewer agent output |
| Branch hygiene | `using-git-worktrees`, `finishing-a-development-branch` | Clean merges |
| Debugging | `systematic-debugging`, `verification-before-completion` | Real (not assumed) verification |

If Superpowers is not installed, the AI-native flow still works — it just runs without the auto-trigger benefits. Always-prefer install.

---

**Last updated:** when the template is adapted to a new project, replace this with the date and a one-sentence summary.
