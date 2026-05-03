# AI-Native Playbook

> **Purpose.** This is the long-form reference for how to build and operate an AI-native company. The 9-principle distillation is in `AI_NATIVE_CONSTITUTION.md` (repo root). This document is what you read when you need the *how* behind a principle.
>
> **Source.** Distilled from Diana (YC) — *How To Build An AI Native Company*.
>
> **Audience.** Claude Code, Claude in chat, future internal agents, and every human collaborator. This document and the constitution take precedence over older docs where they overlap.
>
> **Template note.** Sections 1-10 and 12-13 are generic. Section 11 is *project-specific* and must be rewritten when this template is adapted to a new project.

---

## Table of contents

1. The thesis
2. AI as OS
3. Open loop vs closed loop
4. Queryable organization
5. Context parity
6. Specs and tests as source of truth ("AI software factory")
7. No human middleware (org design)
8. Token-maxing
9. Founder discipline
10. Startup edge
11. The project-specific application *(rewrite per project)*
12. Anti-patterns
13. Glossary

---

## 1. The thesis

The shift in 2026 is not about productivity. It is about new capability. The right person with AI tools can now build features that used to require an entire team — or were not possible at all. Companies that frame AI as "add a copilot to make our team faster" miss the shift. Companies that frame AI as the operating system the company runs on capture it.

---

## 2. AI as OS

**Principle.** AI is not a tool we use. AI is the operating system the company runs on. Every workflow, every decision, every process flows through an intelligent layer that learns and improves.

**Operationally.**
- Every meeting -> AI notetaker -> transcript + summary in queryable storage. Default-on, not opt-in.
- Every customer signal -> captured in machine-readable form (ticket, conversation, NPS, review) -> indexed.
- Every decision -> logged in MemPalace `decisions` room as a knowledge-graph triple with rationale.
- Every code change -> produces specs, tests, PR, CHANGELOG entry — all queryable.
- Every metric that matters -> on a dashboard the AM (and internal agents) can read.

**Anti-pattern.** "Let's add ChatGPT to the team's Slack." That is a tool, not an OS. The OS frame: "Slack is the user-facing surface; everything in Slack is automatically indexed by an agent that can answer questions about company state in real time."

---

## 3. Open loop vs closed loop

**Principle.** No important process runs as an open loop.

### 3.1 Definitions

- **Open loop.** Decision -> action -> outcome not measured systematically -> process not adjusted. Information is fragmented and interpreted manually. Lossy by nature.
- **Closed loop.** Self-regulating. Continuously monitors output and adjusts the process to better meet the goal. Each iteration produces an artifact that feeds back into the next.

### 3.2 Definition of Done for any process to be "AI-native"

A process counts as AI-native only if all of the following hold:

1. **Goal in writing.** The process has a written spec or charter.
2. **Action produces a machine-readable artifact.** Every execution leaves a record an agent can read.
3. **Artifact lands in the index.** It's discoverable by `project_knowledge_search`, MemPalace, or the relevant ops storage — not just stored on someone's laptop.
4. **An agent reads it.** A reviewer agent or scheduled job reads the artifact and proposes the next action / improvement / flag.
5. **A metric is measured automatically.** Success of the process is visible on a dashboard without anyone manually compiling it.

If a process fails any of these, it is open-loop and gets queued for redesign.

### 3.3 Standard mapping

| Process | Closed-loop shape |
|---|---|
| Sprint planning | Roadmap spec -> agent reads PRs/feedback/Roadmap -> proposes next sprint -> human DRI judges -> MemPalace decision triple. |
| Customer feedback handling | Feedback ingested (Pylon/etc.) -> tagged by agent -> routed to Linear/spec -> spec generates PR -> outcome measured. |
| Agent calibration | Conversation -> Braintrust eval -> drift flagged -> Calibrator proposes prompt diff -> Ops approves -> new `agent_version` deployed -> metric tracked. |
| Ops incident | Health signal trips -> Incident Responder pages on-call -> drafts incident report -> resolution logged -> post-mortem auto-generated -> runbook updated. |
| Monthly client report | KPI data -> Reporter agent generates draft -> ops samples -> AM delivers to client -> client feedback captured -> next report adjusted. |

---

## 4. Queryable organization

**Principle.** The whole company is legible to AI. Every important artifact lives in a stable, indexable place.

### 4.1 What must be an artifact

- **Meetings.** AI notetaker (Fathom, Otter, or custom Whisper) — transcript + summary in queryable storage in the day of the call.
- **Communication.** DMs and email minimized. Working communication in Slack channels (or the equivalent) with an indexing agent.
- **Tickets.** Linear, with links to specs and tests.
- **Code.** GitHub, with PR descriptions that reference the spec they implement.
- **Customer feedback.** A queryable inbox (Pylon, Plain, or similar) — never just "notes from a call".
- **Plans.** Notion or Google Docs with stable URLs, never PDFs in folders.
- **Sales calls.** Recorded, transcribed, indexed by deal + vertical.
- **Standups.** Async-written or recorded; never "discussed verbally and forgotten."
- **Decisions.** MemPalace knowledge-graph triples in `decisions` room with rationale.

### 4.2 Dashboards

One consolidated internal dashboard that surfaces:
- Revenue (Stripe / billing).
- Pipeline (CRM).
- Engineering velocity (GitHub).
- Agent quality (Braintrust judge scores per agent, if applicable).
- Ops state (incidents, tickets, calibration backlog).
- Token spend (per project, per week).
- Hiring pipeline (when relevant).

If an internal teammate (or agent) has to ask "how are we doing on X" and the answer isn't on the dashboard, the dashboard is incomplete.

### 4.3 Indexing rule

Every artifact has a stable URL or repo path. Anything with state but no URL is broken.

---

## 5. Context parity

**Principle.** Give the model the same context you'd give a competent new employee for the same task. No less.

### 5.1 The check

Before any non-trivial task an agent is asked to perform — and before the agent starts — both sides explicitly answer:

1. What does the agent need to know to do this well?
2. What is in the agent's context right now?
3. What is missing?
4. How do we close the gap (search project knowledge, read files, ask the user, run a tool)?

If the gap is not closed, the work is not started. "Try and see" is an open-loop pattern.

### 5.2 The standard sources

For any task in this repo, the agent has access to:
- This playbook + the constitution.
- `CLAUDE.md` (top-level operating manual).
- `docs/` (Boris playbook, multi-agent template, Superpowers integration, MemPalace guide, project-specific docs).
- `specs/` (per-feature spec).
- `tests/scenarios/` (per-feature scenarios).
- `agents/` (per-agent prompt + policy + tool schema).
- `runbooks/` (incident-response and operational procedures).
- The repo source tree itself.
- MemPalace (decisions, sessions, technical, product, ops, gtm, brand, frontend, general rooms).

If information needed for a task is not in this set, that's a gap to close.

---

## 6. Specs and tests as source of truth — the AI software factory

**Principle.** Humans write the spec and the scenarios that define success. Agents write the implementation. Reviews judge output against the spec, not line-by-line code.

### 6.1 The flow

```
Human writes        Human writes              Agent generates    Agent iterates
/specs/X.md     ->  /tests/scenarios/X.md ->  code            -> until scenarios
                                                                  pass
                                                                       |
                                                                       v
                                                              Human judges
                                                              output against spec
```

In the limit, the repo contains specs, scenarios, and prompts; the code is generated and regenerated as needed. We're not at that limit yet — but every feature gets closer.

### 6.2 Mandatory artifacts per feature

For every non-trivial feature shipped to this repo:

- `/specs/<feature>.md` — goal, non-functional requirements, boundaries, definition of done, links to related docs.
- `/tests/scenarios/<feature>.md` — concrete scenarios describing success and failure modes.
- Implementation in `src/` or `supabase/functions/` — generated, reviewed against spec.
- PR description references the spec by path.
- `CHANGELOG.md` entry — agent-generated on merge.

See `specs/README.md` and `tests/scenarios/README.md` for the format.

### 6.3 Probabilistic satisfaction threshold

For LLM-driven features (every agent in this project, if any), the spec includes a probabilistic satisfaction threshold — a Braintrust judge score that must be met before deployment. Below threshold, the `agent_version` doesn't ship. The threshold is a number in the spec, not a vibe.

### 6.4 Definition of Done for a feature

- [ ] Spec exists in `/specs/`.
- [ ] Scenarios exist in `/tests/scenarios/`.
- [ ] Automated tests pass.
- [ ] (For LLM features) Probabilistic satisfaction threshold met.
- [ ] PR references spec; reviewer judged output against spec.
- [ ] CHANGELOG entry generated.
- [ ] Closed loop: a metric for this feature is on the relevant dashboard, and a fed-back improvement path is documented.
- [ ] MemPalace diary entry written in the relevant room.

---

## 7. No human middleware (org design)

**Principle.** The intelligence layer routes information. We do not staff coordinator roles whose job is to pass information between layers.

### 7.1 Three archetypes (Jack Dorsey @ Block model)

| Archetype | Who | Does | Does NOT |
|---|---|---|---|
| **IC / Builder-Operator** | Anyone, including ops/support/sales — not just engineers | Directly builds and runs things. Comes to meetings with working prototypes, not pitch decks. | Does not coordinate other people's work. |
| **DRI** | Anyone with an outcome | One person, one outcome. Owns strategy + customer outcome end-to-end. | No hiding behind committees. |
| **AI Founder** | Founder | Personally builds, coaches, leads by example. On the front line of AI tooling. | Does not delegate AI strategy. |

There is no fourth archetype called "manager" or "coordinator". Information that used to flow through people now flows through the intelligence layer.

### 7.2 DRI rule

Every important outcome has exactly one DRI. If you can't name them in one sentence, the outcome is broken or the org chart is wrong. DRI != "manager"; DRI = the person whose name goes next to the outcome.

---

## 8. Token-maxing

**Principle.** Optimize for tokens converted into shipped outcomes per week. Not for hires, not for hours.

### 8.1 Why

One person with AI tools is the equivalent of what used to take a large engineering team in a pre-AI company. That means dramatically leaner engineering, design, HR, and admin teams. The trade-off: an uncomfortably high API bill, which is a feature, not a bug, because it replaces a far more expensive headcount.

### 8.2 Budgeting

We set a deliberately uncomfortable monthly AI budget and track it against shipped outcomes (features, calibration cycles, reports, customer responses). The unit economic check is:

```
token_spend_this_week / outcomes_shipped_this_week
```

If that ratio is going down month over month while outcome velocity holds, we're doing it right. If it's going up while outcomes flatten, something's broken — usually an open loop somewhere.

Detailed format and weekly entries: `docs/Token Budget.md`.

### 8.3 Default answer to "we need to hire someone for X"

```
Before answering yes:
1. Can an agent do this with the right context? If yes, give it the context.
2. If no, why not? Is the bottleneck context, capability, or judgment?
3. If judgment — is it judgment that requires accumulated taste,
   or judgment we could codify into a spec/rubric and have an agent execute?
4. If it's still a hire — what's the DRI outcome they own?
   Headcount only as a function of unique outcome ownership.
```

---

## 9. Founder discipline

**Principle.** Conviction in these tools is not outsourceable. The founder uses them personally until their own priors break.

### 9.1 The weekly ritual

Every week, the founder writes one short entry — what new capability landed this week that they couldn't do (or do this fast) the week before. If three weeks pass with "nothing new", that's a signal: comfort zone, not progress. Time to spawn a deliberate experiment that breaks a prior.

Format: `docs/Founder Discipline.md`.

### 9.2 Anti-pattern: the AI VP

The failure mode in growing companies is to hire a "VP of AI" or "AI lead" and treat AI strategy as their problem. That is exactly the abdication this principle prohibits. The founder leads from the front; nobody else can compound the conviction the team needs.

### 9.3 Visible AI-leveraged work

When the team grows, the founder periodically demonstrates publicly: "here is something I built this week with agents that I couldn't have built last quarter." This anchors the team's expectation of what's normal.

---

## 10. Startup edge

**Principle.** We have no legacy SOPs, no thousands of people to retrain, no live-product constraints that make change risky. We use that.

### 10.1 What this means in practice

- We design every system AI-native from day one. We do not build "a normal company plus AI tools".
- We refuse to copy the org chart of a $10B incumbent at our scale.
- We refuse to copy the development workflow of a 1000-person eng team at our scale.
- We refuse to copy the customer-success playbook of a 200-CSM SaaS at our scale.
- We start each function from the AI-native principles, then add only what the data shows we need.

### 10.2 What we copy from incumbents

- Compliance basics (RLS, encryption, audit trails, BAAs for healthcare).
- Legal basics (ToS, MSA, DPA templates).
- Hard-won technical patterns (idempotency keys, retries with backoff, dead-letter queues).

We do not copy: org structure; sprint cadence rituals; status-update formats; headcount-driven planning.

---

## 11. The project-specific application *(rewrite per project)*

> **Template placeholder.** When this template is used to bootstrap a new project, replace this section entirely. The structure to follow:
>
> 1. **Product surfaces that must be closed-loop** — list the user-facing surfaces and write the closed-loop shape for each: input -> agent action -> outcome -> measurement -> calibration.
> 2. **Internal operations that must be closed-loop** — list the internal processes (onboarding, calibration, incident response, monthly reporting, expansion).
> 3. **Engineering process** — specifically how spec-first / test-first applies in this project.
> 4. **GTM and customer feedback** — sales-call recording, pilot feedback inbox, win/loss capture, conversion tracking.
>
> Delete this blockquote when you write the real section.

---

## 12. Anti-patterns

If you catch yourself doing any of these, stop and redesign.

- **"Let's just add an AI to the existing process."** That's a tool, not an OS. Redesign AI-first.
- **"Let me ping you in DM."** That's an open loop. Move to a queryable channel.
- **"I'll write up notes after the call."** Lossy. AI notetaker default-on.
- **"We need a VP of AI."** Founder abdicating principle 7.
- **"Let's hire a coordinator to bridge product and ops."** Middleware. Redesign the flow.
- **"Just iterate on the code until it works."** No-spec. Write the spec and scenarios first.
- **"The eval will sort itself out later."** Threshold goes in the spec or the spec is incomplete.
- **"Status update meeting on Mondays."** The dashboard is the status update. Meet only on decisions.
- **"Let's approve this with a committee."** Name the DRI and let them decide.
- **"That feature isn't worth a spec."** If it's worth shipping, it's worth a spec.

---

## 13. Glossary

- **AI as OS** — the framing that the company runs on an intelligent layer rather than uses AI as a tool.
- **Closed loop** — self-correcting process with measured output and adapted process.
- **Open loop** — process without feedback measurement or systematic adjustment.
- **Queryable organization** — one whose data and actions are accessible to agents as an index.
- **Software factory** — specs + tests + agents generating implementation.
- **DRI** — Directly Responsible Individual; one person, one outcome.
- **Token-maxing** — strategy of maximizing useful AI token usage instead of headcount.
- **No human middleware** — refusal of staffing layers whose job is to route information.
- **Probabilistic satisfaction threshold** — the Braintrust judge score below which an LLM `agent_version` cannot deploy.
- **Context parity** — giving an agent the same context a competent new employee would get.
- **AI Founder** — the founder archetype that personally uses AI tools and leads from the front.
- **Builder-Operator** — anyone (engineer, ops, support, sales) who directly builds and runs things, not coordinates them.
