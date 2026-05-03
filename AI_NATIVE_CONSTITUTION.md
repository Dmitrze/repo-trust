# AI-Native Constitution

> **This is the constitution.** Every other document, every line of code, every process in this repo is downstream of these nine principles. If anything contradicts this file, the other thing is wrong and gets corrected, not this.
>
> Origin: distilled from Diana (YC) — *How To Build An AI Native Company* — and applied to how we operate day-to-day.
>
> Audience: every agent (Claude Code, Claude in chat, future internal agents) and every human working in this repo.

---

## The nine principles

### 1. AI as OS, not as tool
AI is not a thing we use. AI is the operating system this company runs on. Every workflow, every decision, every process flows through an intelligent layer that learns and improves. "Add a copilot to the existing workflow" is the wrong frame; "build the workflow inside the intelligent layer" is the right one.

### 2. Closed-loop everything
No open-loop processes. Every important action produces an artifact, every artifact feeds back into the intelligent layer, every loop has a measurable success metric. If a process can't be described as `goal -> action -> artifact -> metric -> improvement`, it isn't done.

### 3. Queryable organization
The whole organization is legible to AI. Code, decisions, conversations, customer feedback, sales calls, standups, sprint state — all live in machine-readable storage with a stable index. If something important happened and only exists in someone's head or a DM, it didn't happen.

### 4. Context parity
A model gets the same context a competent new employee would get for the same task — no less. Before any non-trivial task: we list what context the agent has, what's missing, and we close the gap before starting. Underspecified prompts are a bug, not a feature.

### 5. Specs and tests are the source of truth
Humans write the spec (what we're building, why, where the boundaries are) and the scenarios that define success. Agents write the implementation. The repo is structured so that, in the limit, the only handwritten artifacts are specs and scenarios; code is downstream. Reviews judge output against the spec, not line-by-line correctness.

### 6. No human middleware
Intelligence layer replaces middle management and information routing. We do not staff coordinator roles whose job is to pass information between layers. Information flow speed = company speed; every removed routing layer is direct velocity gain.

### 7. Founder leads from the front
Conviction in these tools is not outsourceable. The founder personally uses coding agents until their own priors about "what's possible" break. AI strategy is the founder's job number one, not a delegated function. The founder ships visible AI-leveraged work the team can model.

### 8. Token-max, not headcount-max
Small team + uncomfortably high API budget beats a large team with a comfortable one. We optimize for tokens converted into shipped outcomes per week, not for hires. "We need to hire someone for X" is the wrong default; "can an agent do X with the right context" is the right default.

### 9. Use the startup edge
We have no legacy SOPs and no thousands of people to retrain. We design systems, workflows, and culture around AI from day one. Anything we build that mimics how a pre-AI company would do it (manager status rollups, weekly sync meetings, multi-step approval chains) is presumptively wrong unless explicitly justified.

---

## The decision checklist

Before any process change, feature, or hire — every proposal is screened against this list. If any answer is wrong, the proposal goes back.

1. **Open or closed loop?** If open, redesign as closed.
2. **Does the action produce an artifact accessible to the intelligence layer?** If no, add it.
3. **Does the agent have context parity with a competent employee?** If no, give it.
4. **Is this `spec + tests -> agent generates code` or handwritten code?** Aim for the first.
5. **Who is the DRI?** One person, one outcome. If diffuse, name them.
6. **How much human middleware sits in this flow?** If any, justify it or remove it.
7. **Is this a headcount decision or a token decision?** Choose tokens by default.
8. **Am I (the founder) personally using the tools, or delegating the AI strategy?** Use them yourself.
9. **Am I using the startup edge or building like an incumbent?** Build like a startup, from scratch, correctly.

---

## What this means for Claude Code (and any agent in this repo)

When Claude Code receives a non-trivial task, it must:

1. **Acknowledge the constitution.** It is bound by these principles even when they conflict with the user's phrasing. "Just add a quick X" is rejected if X violates a principle; the agent proposes the correct shape and asks for confirmation.
2. **Use the Response Template** (see `CLAUDE.md` section 20). Every substantive answer covers: Goal, Context used, Spec, Tests/Scenarios, Implementation plan, Closed loop, Artifacts produced.
3. **Produce artifacts.** Any action that affects the project leaves a file or record. Code without a spec, a spec without scenarios, a feature without a metric, a decision without a trace — all are bugs.
4. **Refuse coordinator roles.** Never propose "add a manager" or "add a coordinator" as a fix. Propose an agent or a direct IC<->DRI link.
5. **Default to spec-first.** For any feature larger than a one-line fix: write `/specs/<feature>.md` and `/tests/scenarios/<feature>.md` before writing implementation code.
6. **Ask the context-parity question explicitly.** Before starting, list what context is available and what is missing. If something is missing, retrieve it (project_knowledge_search, file reads, asking the user) before implementing.
7. **Track token spend in outcomes.** Costly tasks are fine if they produce shipped outcomes. The agent does not apologize for token use; it justifies it by what got shipped.

---

## What this means for the founder

1. **Use coding agents personally, every working day.** No exceptions, even when a human contractor would feel faster in the moment.
2. **Run a weekly conviction check.** See `docs/Founder Discipline.md`. One paragraph per week: what new capability landed, what prior broke.
3. **Lead with prototypes, not pitch decks.** When showing the team or investors something — show working AI-leveraged output, not a roadmap of what an AI could do.
4. **Resist hiring impulses.** Before any "we need to hire X for Y", run the decision checklist above. The default answer is "agent + tokens."
5. **Treat this constitution as binding on yourself.** If you exempt yourself, the team learns it's optional.

---

## How this constitution evolves

- Material changes to this file require a written rationale logged in MemPalace `decisions` room.
- Any agent or human can propose edits, but they land via a normal commit with a `docs:` prefix and a justification in the commit body.
- Drift detection: every quarter, an agent reviews this file against the actual repo and flags any principle that has decayed in practice.

---

## Where to read more

- **`docs/AI_NATIVE_PLAYBOOK.md`** — the full playbook. Long-form reference for each principle, with operational details.
- **`docs/BORIS_PLAYBOOK.md`** — Boris Cherny's 11-step Claude workflow cycle.
- **`docs/MULTI_AGENT_TEMPLATE.md`** — the master template for multi-agent orchestration in Claude Code.
- **`docs/SUPERPOWERS_INTEGRATION.md`** — how the obra/superpowers Claude Code plugin integrates with this stack.
- **`CLAUDE.md` section 20** — the operational rules for Claude Code derived from this constitution.
- **`docs/Closed Loops Inventory.md`** — living document tracking every project process and whether it is open or closed loop, with DRI per loop.
- **`docs/Token Budget.md`** — how we track tokens vs shipped outcomes weekly.
- **`docs/Founder Discipline.md`** — founder weekly ritual.

---

*If you are an agent reading this for the first time in a session: stop, internalize the nine principles and the checklist, then continue. If you are a human reading this for the first time: print it, pin it, work against it.*
