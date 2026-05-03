# /agents — Agent Definitions

> **Every LLM agent in this project — client-facing and internal — is fully defined here.** Prompt, policy, tool schema, eval threshold, runbook reference. One folder per agent. The repo is the source of truth; the database stores deployed `agent_versions`, but those are derived from this directory.
>
> See `AI_NATIVE_CONSTITUTION.md` (principles 2, 5, 6) and `docs/AI_NATIVE_PLAYBOOK.md` section 6.
>
> **If this project does not have LLM agents as part of the product**, delete this directory in your downstream project. Otherwise, fill in the agent inventory below per project.

---

## What goes here

One directory per agent, named in kebab-case.

### How to define your agent inventory

Replace this section with the actual list of agents for your project. Group by audience (client-facing vs internal vs orchestration). For each agent, give a one-line description.

**Example structure** (replace with your project's actual agents):

```
Client-facing agents (what end users interact with):
  - agents/<agent-1>/   - <one-line description>
  - agents/<agent-2>/   - <one-line description>
  ...

Internal operations agents (what the company runs on):
  - agents/onboarder/       - ingests intake, instantiates templates, schedules kickoff
  - agents/calibrator/      - LLM-as-judge over transcripts; flags drift; proposes diffs
  - agents/qa-sampler/      - human-in-the-loop sampling on a fraction of conversations
  - agents/incident-responder/ - watches health signals; pages on-call; drafts incident reports
  - agents/reporter/        - generates monthly outcome reports per client (KPIs + narrative + PDF)
  - agents/expander/        - nightly scan for upsell signals; drafts expansion pitches

Account Manager (one, special-cased) - the primary client-facing surface:
  - agents/am/              - the AI Account Manager
```

---

## What lives in each agent directory

Start from `_TEMPLATE/` (use this README's structure when copying). Each agent has:

```
agents/<agent-name>/
  README.md             - 1-paragraph what + why; pointer to the spec; current status.
  prompt.md             - the system prompt for this agent. Plain markdown.
                          Versioned via git history; current file = current production prompt.
  policy.md             - hard rules and red lines: what the agent must never do,
                          what triggers escalation, what gets refused.
  tools.json            - typed tool schema (zod-derived JSON). Source of truth for
                          what tools the agent can call and with what arguments.
  threshold.md          - the probabilistic satisfaction threshold for this agent:
                          eval suite, judge rubric, numeric threshold, drift action.
  eval-set/             - the curated evaluation examples used in Braintrust. Versioned
                          alongside the prompt: prompt change -> eval change -> review.
    examples.jsonl       - the actual examples (input + expected behavior + tags).
    rubric.md            - the LLM judge's rubric.
  changelog.md          - per-agent change log: what changed, when, why, eval delta.
```

For each agent, the matching `specs/<agent-name>.md` and `tests/scenarios/<agent-name>.md` exist. The spec is the prose contract; this directory is the operational artifact.

---

## What does *not* go here

- Agent runtime code (Edge Function code, orchestration logic) — lives in `supabase/functions/<function-name>/` and `src/` once those exist.
- Conversation history — lives in Postgres (`conversations`, `messages` tables).
- Per-tenant overrides — those live in `tenants.config` JSONB or in the `agent_versions` table; never hard-coded in prompts here.
- Marketing copy about the agent — lives in `docs/Marketing Playbook.md` and on landing pages.

---

## How prompts are deployed

This directory is the **source of truth**; the runtime reads `agent_versions` rows from the database. The flow is:

1. Edit `agents/<agent>/prompt.md` (or `policy.md` / `tools.json` / `threshold.md`) on a feature branch.
2. Run the Braintrust eval against the `eval-set/` and confirm threshold is met.
3. Open a PR. Reviewer reads diff against the spec.
4. On merge to `develop`, an Edge Function (`deploy-agent-version`) reads the new prompt and inserts an `agent_versions` row marked `pending`.
5. Ops promotes the version through `/ops/deploy` to staging -> prod, role-gated.
6. The version's text in the database equals the file in this directory at the time of promotion. Never diverges.

The runtime never reads files from this directory at request time. The deploy step copies content into the database; that's the boundary.

---

## Versioning rules

- Every change to `prompt.md`, `policy.md`, `tools.json`, or `threshold.md` is a versioned edit — git tracks the history; `agent_versions` table tracks deployed versions.
- The current file content equals the *intended* current production prompt.
- A separate row in `agent_versions` per deployed version, with the prompt text snapshotted, eval scores recorded, deployer recorded, audit logged.
- Rolling back is a one-click action in `/ops/deploy`: select a prior `agent_versions` row, promote it.

---

## Threshold rule (binding)

For every agent: `threshold.md` defines a numeric threshold below which the agent cannot deploy.

Default floors (adjust per project; raise if the agent's risk profile demands it):

| Agent class | Default threshold (mean judge score, 0-1) |
|---|---|
| High-risk client-facing (voice, booking) | 0.90 |
| Standard client-facing (review, support, retention) | 0.85 |
| Approval-gated client-facing (social) | 0.80 (approvals required regardless) |
| Internal calibration / reporting | 0.85 |
| Human-in-the-loop QA | n/a (sampled) |
| Account Manager | 0.88 |

Lowering a threshold requires an explicit decision artifact (MemPalace `decisions` triple + CHANGELOG entry).

---

## Multi-tenant adaptation

The prompt in this directory is the **base** prompt. At runtime the AM service composes a per-tenant prompt by:

1. Reading the base prompt from the deployed `agent_versions` row.
2. Merging in tenant-specific context: vertical, business name, hours, services, integrations enabled, recent KPI summary, tone preference.
3. Calling the model with the composed prompt.

Tenant context is **never** hard-coded into base prompts. If you find yourself writing "for restaurants..." branches in `prompt.md`, either split into separate vertical agents or move the branching to the runtime composer.

---

## How Claude Code uses this directory

When Claude Code is asked to:

- **Build a new agent.** Create `agents/<n>/` from the template; write `prompt.md`, `policy.md`, `tools.json`, `threshold.md`, `eval-set/examples.jsonl`, `eval-set/rubric.md`. Match the spec in `specs/<n>.md`.
- **Tune an existing agent.** Edit the relevant file(s). Run the eval. Update `changelog.md` for that agent. Update the matching spec if behavior changed.
- **Investigate a regression.** Read the agent's `changelog.md` for recent changes; diff `prompt.md` against the version that was working; check the eval-set for whether the failing case was covered.
- **Add a new tool to an agent.** Update `tools.json` (zod-derived); update `prompt.md` to describe when to call it; add scenarios in `tests/scenarios/<agent>.md`; run the eval; deploy.

---

## How humans use this directory

- Reading what an agent does today: `prompt.md` + `policy.md` + `tools.json`.
- Reviewing a proposed change: read the diff against the existing files; compare eval scores in the PR body; check if `threshold.md` was respected.
- Onboarding a new ops teammate: walk them through one client-facing agent first, then `agents/calibrator/` — the rest follow the pattern.

---

*Maintainer: the founder. The DRI for any individual agent's behavior is named in that agent's spec frontmatter; for the platform-level rules in this README, the DRI is the founder.*
