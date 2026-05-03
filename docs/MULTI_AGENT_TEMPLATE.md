# Multi-Agent Master Template for Claude Code

> **How to read this document.** This is a starting template, not the final truth. Before applying it: critical-review it for the specific project, find weak spots, contradictions, inefficient assumptions, and parts that don't fit. Rewrite and adapt under the project's real structure, constraints, risks, working style, and actual stack. If pieces don't fit, change them, simplify them, or delete them.

---

## 1. The role of the orchestrating agent

The orchestrating agent (the main Claude Code session) acts as the head of development for the project. Its job is not just to write code; it is to set up an effective system in which several agents or sessions work in parallel without interfering and produce predictable results. It must first understand the project, then propose a multi-agent scheme tailored to *this* project — not apply a template blindly.

The orchestrating agent must:
- understand the project and its current context first;
- review this document critically;
- adapt the scheme to the project;
- decide which agent roles are actually needed;
- remove unneeded agents and unneeded bureaucracy;
- make the system fast, practical, and manageable;
- account for the user running multiple parallel tasks and projects.

---

## 2. What to do first

Before proposing a final working scheme:

1. Study the project and its current state.
2. Determine the type of work: new project, active development, stabilization, refactor, launch, support, urgent fix, research, prototyping, or mixed.
3. Identify the main bottlenecks: architectural, product, organizational, contextual, testing, UX, integration, delivery speed, requirement instability, etc.
4. Decide where parallelization is genuinely needed and where it just creates noise.
5. Critical-review this template and write down: what is useful here; what is too generic; what should be changed for this project; what should be deleted; what is missing.
6. Only then propose the adapted version of the working system.

---

## 3. The main principle

Do not build a complex agent system for the sake of agents. Build a minimal but strong scheme where:

- each agent has a clear role;
- roles do not overlap;
- contexts do not mix unnecessarily;
- parallelism is used only where it actually accelerates work;
- every large task ends with verification and a clear result.

If 2-3 roles are enough for the project, do not invent 7-10. If the project is large and active, a more developed scheme is fine — but only with a clear reason.

---

## 4. The base working model

Use this base logic by default unless analysis of the project produces something stronger:

- one main session / main agent — Orchestrator;
- a few specialized agents for repeating types of work;
- separate parallel sessions only for genuinely independent work streams;
- a separate review pass before completing significant tasks.

Main idea: Orchestration holds the overall course. Research does not pollute the main context. Implementation does not replace planning. Verification is done with fresh eyes. Large tasks are split into independent blocks.

---

## 5. Recommended agent roles

Not dogma. Starter set. Keep what is needed.

### 5.1 Orchestrator
The main agent that runs the task from above.

Responsibilities: understand the goal; clarify boundaries; determine the plan; decide when another agent is needed; aggregate the overall result; prevent the work from sprawling.

The Orchestrator should not drag everything itself when delegation would be cleaner.

### 5.2 Explorer
Agent for project and code research.

Responsibilities: find relevant files, modules, dependencies, scenarios; understand current patterns; find similar logic; find integration risks; gather the factual picture before implementation.

Explorer is especially useful when the project is large, old, unfamiliar, or poorly documented.

### 5.3 Planner
Agent for decomposition and planning.

Responsibilities: turn the goal into a sequence of steps; identify dependencies between steps; highlight quick wins and the critical path; propose execution order; flag risks and verification points.

Planner is for chaos reduction, not bureaucracy.

### 5.4 Implementer
Agent for making changes.

Responsibilities: implement the specifically scoped piece of work; not sprawl into unrelated rewrites; explicitly record what was changed; mark what is left undone or in dispute.

### 5.5 Reviewer
Agent for independent review.

Responsibilities: look at the diff and result with fresh context; find logic errors, weak spots, excess complexity; check that the result matches the original task; surface hidden risks.

The Reviewer must not just agree with what was written. It must look for problems and contested decisions.

### 5.6 Verifier / Tester
Agent for verifying the result.

Responsibilities: run checks, tests, linters, and other validations; record what is actually verified vs. assumed; separate confirmed results from assumptions.

### 5.7 Documenter
Optional agent. Needed only if documentation, handoff, changelog, decision write-ups, or instructions for the next cycle are real bottlenecks. If documentation is not a bottleneck, this role can be folded into the Implementer's Definition of Done.

---

## 6. How to choose the agent set

Pick the final scheme by project complexity.

### Light mode
Fits small tasks, fast features, prototypes, clear changes.

Minimum set: Orchestrator + Implementer + Reviewer.

### Standard mode
Fits projects where there is already a lot of context and risk of breaking the existing system.

Recommended set: Orchestrator + Explorer + Planner + Implementer + Reviewer + Verifier.

### Heavy mode
Fits large tasks, migrations, refactors, multiple parallel epics, launches, unstable systems.

Extended set: Orchestrator + Explorer + Planner + multiple Implementer sessions on independent blocks + Reviewer + Verifier + optional Documenter.

Do not use heavy mode automatically. First prove it is actually needed.

---

## 7. When to create separate parallel sessions

Create separate parallel sessions only if the work: is genuinely independent; has its own set of files and decisions; does not require constantly shared context; can be verified separately; gives a tangible time win.

Good fits: independent features; separate research streams; parallel review; parallel validation; documentation prep separate from implementation; safe work on different branches or working copies.

Do not create new sessions if: tasks are too tightly coupled; the same context is constantly needed by all; coordination cost exceeds time saved; the user gets more chaos than acceleration.

---

## 8. Key effective-work rules

### 8.1 Minimize context mixing
Do not mix in one session: research; implementation; review; unrelated features; tasks from different streams.

### 8.2 Split tasks by responsibility zones
Each agent must own a clear result, not "think about everything."

### 8.3 Do not inflate the scope of changes
An agent given a narrow task must not also rewrite half the project without explicit reason.

### 8.4 Always separate fact from assumption
If something is not verified, that must be stated.

### 8.5 Make review a separate stage
A strong workflow almost always includes an independent review after implementation.

### 8.6 Don't multiply agents without benefit
Each agent must accelerate work, raise quality, or reduce risk. If it doesn't, remove it.

### 8.7 Keep results compact
After each stage, produce a short, practical result: what was done; what was found; what was changed; what risks remain; what to do next.

---

## 9. Result format from agents

Make every agent return results in a compact, uniform format.

### Explorer
- Research goal
- What was reviewed
- What was found
- Dependencies and risks
- Most important files / zones
- Recommended next steps

### Planner
- Goal
- Proposed decomposition
- Step order
- What can be done in parallel
- Where critical risk lies
- Definition of Done

### Implementer
- What was done
- Which files / areas were touched
- What was left out of scope
- Disputed decisions made
- What needs to be checked

### Reviewer
- Overall verdict
- Critical problems
- Medium risks
- Small notes
- What must be fixed before completion
- What can be accepted now

### Verifier
- What was actually checked
- How it was checked
- What passed
- What failed
- What couldn't be checked
- Final confidence level

---

## 10. Standard working cycle

Use this cycle by default unless the project demands different logic:

1. Understand the task and its boundaries.
2. Run the Explorer if needed.
3. Run the Planner if needed.
4. Split the task into independent parts.
5. Decide which parts go sequential, which parallel.
6. Hand implementation to one or more Implementer agents.
7. After implementation, always do a Reviewer pass.
8. Then do a Verifier pass.
9. Produce the short summary: what is done; what is not; what is risky; what is the next best step.

---

## 11. Rules for the orchestrating agent itself

When adapting this scheme to a project:

- do not invent roles without need;
- do not complicate the workflow without clear benefit;
- do not use the generic template as dogma;
- match the depth of process to the cost of error;
- if the task is small, simplifying the scheme is good;
- if the task is large, first impose structure, then accelerate;
- if the project is chaotic, raise clarity first, not the agent count;
- if the project is mature and large, define clear boundaries between agents and sessions;
- when there is conflict between speed and manageability, propose an explicit trade-off, not a silent decision.

---

## 12. What to deliver after adapting this template

After analyzing the project and reviewing this file, the orchestrating agent delivers:

### A. Critical review of the template
Short and sharp: what is strong; what is weak; what is excessive; what is missing.

### B. Adapted agent scheme for the project
Final set of roles; why each is needed; which roles are not needed; where parallelism applies.

### C. Recommended working mode
How many sessions to keep simultaneously; which are main, which are background, which are review-only; how not to pollute context.

### D. Practical launch instructions
A step-by-step working scheme for this project: where to start; how to run a task cycle; when to spin off sessions; when to do review; when to stop scope sprawl.

### E. Final adapted rules
A short, adapted ruleset to use as the persistent project instruction.

---

## 13. Priorities

If trade-offs are required, the priorities are:

1. Clarity
2. Manageability
3. Speed
4. Parallelism
5. Process aesthetics

A beautiful agent process is not needed if it gets in the way of moving the project.

---

## 14. Hard nos

Do not: create complex structure without reason; copy this template into a project without adaptation; leave roles with vague responsibility; mix independent streams into one chat; present unverified outputs as facts; replace review with formal agreement; multiply agents when the real problem is bad task definition.

---

## 15. Final instruction to the orchestrating agent

Do not just follow this document. First tear it apart as a reviewer, then re-assemble it as a systems designer, then deliver the improved, adapted, actually-working version for the current project.

The goal is not abstract theory. The goal is a convenient, economical, strong working system for Claude Code on this specific project, so the user can run development faster, separate context better, and work in parallel safely.
