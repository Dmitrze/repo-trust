# Gstack Integration Guide

> **What this is.** A mapping from [garrytan/gstack](https://github.com/garrytan/gstack) — Garry Tan's (CEO YC) 23 slash-command skills for Claude Code — onto the AI-native foundation flow (specs / scenarios / agents / runbooks / multi-agent roles).
>
> **Audience.** Claude Code, after gstack is installed.
>
> **Prerequisite.** gstack installed via the one-line bootstrap (see below). One-time install per developer machine.

---

## What gstack is in one paragraph

gstack turns Claude Code into a virtual engineering team via 23 specialized slash-commands. Each command is a specialist: a CEO, an eng manager, a designer, a reviewer, a QA lead, a security officer, a release engineer. They run in the order a sprint runs: **Think -> Plan -> Build -> Review -> Test -> Ship -> Reflect**. Each skill feeds into the next: `/office-hours` writes a design doc that `/plan-ceo-review` reads; `/plan-eng-review` writes a test plan that `/qa` picks up; `/review` catches bugs that `/ship` verifies are fixed.

MIT-licensed, free, open source. The provenance is real: gstack is what Garry Tan personally uses to ship a reported ~810x his 2013 LOC pace solo.

---

## Why integrate at all

Three layers of tooling, three different jobs:

| Layer | What it provides | Examples |
|---|---|---|
| **Superpowers** | *Behavior* — auto-triggered skills | `brainstorming`, `writing-plans`, `test-driven-development`, `requesting-code-review` |
| **gstack** | *Specialist roles* — slash-commands | `/office-hours`, `/plan-ceo-review`, `/review`, `/qa`, `/cso`, `/ship` |
| **AI-native foundation** | *Structure* — artifacts and binding rules | `specs/`, `tests/scenarios/`, `agents/`, `runbooks/`, constitution |

They are complementary, not competing:

- **Superpowers** says *when* to plan / brainstorm / TDD.
- **gstack** says *who in the virtual team is doing it* (CEO vs eng manager vs designer vs QA lead).
- **AI-native foundation** says *where the artifact lives and what shape it has*.

The full chain: Superpowers fires `brainstorming` -> gstack `/office-hours` runs the role with structure -> the AI-native pack saves the result to `specs/<feature>.md`.

---

## The mapping

### `/office-hours` -> `/specs/<feature>.md` (the entry point)

**gstack behavior.** YC Office Hours role: six forcing questions that reframe the product. Pushes back on framing, challenges premises, generates implementation alternatives. Writes a design doc that downstream skills read.

**AI-native binding.** The output of `/office-hours` is the source for a new spec. After the session, Claude Code:

1. Opens `specs/_TEMPLATE.md`.
2. Fills the Goal, Non-functional requirements, Boundaries, Happy-path scenario from the office-hours design doc.
3. Saves to `specs/<feature>.md` with frontmatter `status: draft`.
4. Surfaces for human approval before proceeding to plans.

### `/plan-ceo-review` + `/plan-eng-review` + `/plan-design-review` + `/plan-devex-review` -> spec hardening + scenarios

**gstack behavior.** Four review skills run on the design before code. CEO challenges scope; eng manager locks architecture; designer rates UX 0-10; DX lead audits developer experience.

**AI-native binding.** All four feed back into the spec and produce the scenarios:

- CEO output (scope expansion / hold / reduction) -> updates `specs/<feature>.md` boundaries section.
- Eng review (architecture, edge cases, tests) -> populates `tests/scenarios/<feature>.md` happy-path + edge-cases + failure-modes.
- Design review (UX dimensions) -> populates `tests/scenarios/<feature>.md` accessibility section.
- DX review (TTHW, friction points) -> populates `tests/scenarios/<feature>.md` if the feature has a developer-facing surface.

### `/autoplan` -> the standard plan-and-review pipeline

**gstack behavior.** One command runs CEO -> design -> eng review automatically with encoded decision principles. Surfaces only taste decisions for approval. Saves the plan; doesn't implement.

**AI-native binding.** This is the canonical pre-implementation flow for non-trivial features. After `/autoplan`:

1. Spec is updated with all four reviews' decisions.
2. Scenarios file is fully populated.
3. Spec status is `proposed` -> human approves -> `accepted`.
4. Only after `accepted` does `/ship` run.

### `/review` -> Reviewer agent role from MULTI_AGENT_TEMPLATE

**gstack behavior.** Staff Engineer reviews diffs. Auto-fixes obvious issues. Flags completeness gaps. Two-stage: spec compliance, then code quality.

**AI-native binding.** This is the Reviewer role from `docs/MULTI_AGENT_TEMPLATE.md` section 5.5 made executable. The output format matches MULTI_AGENT_TEMPLATE section 9: overall verdict, critical problems, medium risks, small notes, what must be fixed before completion, what can be accepted now.

### `/qa` -> Verifier role + scenario coverage

**gstack behavior.** Real Chromium browser. Real clicks. Real screenshots. Tests staging URL, finds bugs, fixes them with atomic commits, generates regression tests, re-verifies.

**AI-native binding.** This is the Verifier role from MULTI_AGENT_TEMPLATE section 5.6. Every regression test it generates becomes a new scenario in `tests/scenarios/<feature>.md` with a stable `S-NNN` ID. The bug becomes a permanent test.

### `/cso` -> security audit gate before merge

**gstack behavior.** Chief Security Officer role. OWASP Top 10 + STRIDE threat model. 8/10+ confidence gate. Each finding includes a concrete exploit scenario.

**AI-native binding.** For any feature touching auth, RLS, PII, or external integrations: `/cso` runs before `/ship`. Findings update:

- `specs/<feature>.md` Non-functional-requirements section (new security NFR) if a class of issue is found.
- `tests/scenarios/<feature>.md` security-and-privacy section (new `S-5NN` scenarios).
- `runbooks/<related-incident>.md` if the issue class needs operational response.

### `/investigate` -> systematic-debugging integration

**gstack behavior.** Iron Law: no fixes without investigation. Traces data flow, tests hypotheses, stops after 3 failed fixes. Auto-freezes to the module being investigated.

**AI-native binding.** When investigating an incident:

1. `/investigate` produces a root-cause analysis.
2. The analysis lands in `runbooks/<scenario>.md` post-incident section.
3. New scenarios cover the failure mode in `tests/scenarios/<feature>.md`.
4. MemPalace `decisions` triple records the root cause and the fix decision.

### `/ship` + `/land-and-deploy` -> Release-engineer role + CHANGELOG

**gstack behavior.** `/ship` syncs main, runs tests, audits coverage, pushes, opens PR. `/land-and-deploy` merges PR, waits for CI, verifies production health.

**AI-native binding.** Always runs:

- Adds the `CHANGELOG.md` entry (per Definition of Done).
- Updates spec status from `accepted` to `shipped`.
- Writes MemPalace diary entry in the relevant room.
- Auto-invokes `/document-release` to update READMEs / `CLAUDE.md` `## 19 Where to find everything` if anything changed.

### `/retro` -> sprint-planning closed loop (Closed Loops Inventory C.2)

**gstack behavior.** Weekly engineering retro. Per-person breakdowns, shipping streaks, test health trends, growth opportunities. `/retro global` runs across all your projects.

**AI-native binding.** This is the closed-loop closer for `Closed Loops Inventory C.2 Sprint planning + retrospective`. Output lands in `docs/sprints/SPRINT-N-retro.md`. Read by next sprint's planning session.

### `/learn` -> MemPalace integration

**gstack behavior.** Manages what gstack learned across sessions: project-specific patterns, pitfalls, preferences. Compounds across sessions.

**AI-native binding.** Complements MemPalace. Where MemPalace stores explicit knowledge (decisions with rationale, session diaries, structured triples), `/learn` stores tacit patterns ("this codebase uses zod for tool schemas", "the founder prefers indigoDark on hover, never white"). Both feed Claude Code at session start.

---

## End-to-end flow with all three layers active

A new non-trivial feature, from "I want X" to merged in production:

1. **User asks Claude Code for the feature.** Trivial / non-trivial check applies.
2. **Superpowers `brainstorming` activates** -> design doc draft.
3. **gstack `/office-hours` runs** with the design doc as input -> reframed product spec, six forcing questions answered.
4. **Claude Code writes `specs/<feature>.md`** from `/office-hours` output. Status: `draft`. Surfaces to user.
5. **User approves.** Status: `proposed`.
6. **gstack `/autoplan` runs** (CEO -> design -> eng -> DX in sequence). Spec hardened; scenarios file populated.
7. **User approves taste decisions.** Status: `accepted`.
8. **Superpowers `using-git-worktrees`** -> branch `feat/<feature>` created. Clean baseline confirmed.
9. **Multi-agent template kicks in** (per `docs/MULTI_AGENT_TEMPLATE.md`): Orchestrator + Implementer (one or many in parallel).
10. **Superpowers `test-driven-development`** runs RED-GREEN-REFACTOR per `S-NNN` scenario.
11. **gstack `/review` runs** between tasks. Output in MULTI_AGENT_TEMPLATE Reviewer format.
12. **gstack `/cso` runs** if the feature touches auth / RLS / PII / integrations. Findings -> spec / scenarios / runbooks updates.
13. **gstack `/qa` runs** against staging URL. Real browser. Real clicks. Bugs fixed in atomic commits. Regression tests added as new scenarios.
14. **Superpowers `verification-before-completion`** confirms real pass/fail with evidence.
15. **gstack `/ship` runs.** Tests pass, coverage audit, PR opened with spec link in description, `CHANGELOG.md` entry, MemPalace diary entry, `/document-release` auto-invoked.
16. **gstack `/codex` runs** as second-opinion review (independent from OpenAI Codex CLI).
17. **Reviewer green -> gstack `/land-and-deploy`** merges, CI passes, production verified.
18. **gstack `/canary` runs** for post-deploy monitoring window.
19. **Spec status -> `shipped`.**

The entire flow is one Claude Code session (or coordinated set per multi-agent template). The user reviews at steps 5, 7, 11/12/13, 17.

---

## Install

One-time per developer machine. Paste in Claude Code:

```
Install gstack: run `git clone --single-branch --depth 1 https://github.com/garrytan/gstack.git ~/.claude/skills/gstack && cd ~/.claude/skills/gstack && ./setup` then add a "gstack" section to CLAUDE.md that says to use the /browse skill from gstack for all web browsing, never use mcp__claude-in-chrome__* tools, and lists the available skills: /office-hours, /plan-ceo-review, /plan-eng-review, /plan-design-review, /design-consultation, /design-shotgun, /design-html, /review, /ship, /land-and-deploy, /canary, /benchmark, /browse, /connect-chrome, /qa, /qa-only, /design-review, /setup-browser-cookies, /setup-deploy, /setup-gbrain, /retro, /investigate, /document-release, /codex, /cso, /autoplan, /plan-devex-review, /devex-review, /careful, /freeze, /guard, /unfreeze, /gstack-upgrade, /learn.
```

### Team mode for shared repos (recommended for any project repo created from this template)

From inside the new project repo:

```bash
(cd ~/.claude/skills/gstack && ./setup --team) && ~/.claude/skills/gstack/bin/gstack-team-init required && git add .claude/ CLAUDE.md && git commit -m "require gstack for AI-assisted work"
```

Switches to team mode. Bootstraps the repo so teammates auto-update gstack on every Claude Code session start. No vendored files in the repo, no version drift.

### Verify

In a new Claude Code session: type `/office-hours` and the YC office-hours skill should activate.

---

## What to do if gstack is not installed

The AI-native flow still works. Cost: skills don't auto-trigger; the user must explicitly request brainstorming / planning / review. The structural rules (specs first, scenarios first, response template, artifacts mandatory) still apply because they live in `CLAUDE.md` section 20.

**Recommendation.** Install gstack. The install cost is one paste; the velocity gain is permanent.

---

## Combined precedence (when gstack and AI-native pack diverge)

If gstack and this guide appear to disagree on something:

- **Behavior** (when a skill triggers, what it does internally): gstack wins.
- **Artifact placement** (where output goes): this pack wins.
- **Binding rules** (no spec without scenarios; no merge without DoD; no agent without threshold): the constitution wins, always.

Most apparent conflicts are resolved by remembering: gstack ships specialist roles, this pack ships structure, the constitution ships the non-negotiables.

---

## Keeping gstack updated

```
/gstack-upgrade
```

Detects global vs vendored install, syncs both, shows what changed. When you update, scan the new release notes for any changes that affect this mapping. If a skill is renamed, deprecated, or has a materially different output shape, update this guide accordingly and add a `CHANGELOG.md` entry.
