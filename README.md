# AI-Native Foundation

> **GitHub Template repository.** Use this as the starting point for any new AI-native project. The template installs the constitution, playbook, multi-agent operating model, MemPalace memory layer, Boris workflow cycle, and Superpowers-skills integration on day one.
>
> **Maintainer:** Dmitry Melnik. **Source:** distilled from Diana (YC) "How To Build An AI Native Company", Boris Cherny's Claude playbook, the multi-agent master template, and obra/superpowers skills.

---

## What this is

A portable, opinionated, AI-native fundament for new projects. It encodes nine binding principles (constitution), the spec-first / test-first software factory flow, the multi-agent orchestration model (Orchestrator + Explorer + Planner + Implementer + Reviewer + Verifier), Boris's 11-step workflow cycle, MemPalace persistent memory, and how to use the Superpowers Claude Code plugin alongside this stack.

The goal: every new project starts from the same standard, and Claude Code is bound by the same operating rules across projects.

---

## Quick start (5 minutes for a new project)

### 1. Create a new repo from this template

On this repo's GitHub page, click the green **Use this template** button → **Create a new repository**. Name it whatever your project is. Keep it private.

### 2. Clone it locally

```bash
cd ~/projects
git clone git@github.com:<you>/<new-project>.git
cd <new-project>
```

### 3. (Optional but recommended) install the supporting tools

```bash
# MemPalace — persistent memory for Claude Code across sessions
pipx install mempalace
mempalace init .
mempalace mine .

# Superpowers — Claude Code plugin with TDD/brainstorming/planning skills
# (one-time install for your machine, not per-project)
# In Claude Code:
# /plugin marketplace add obra/superpowers-marketplace
# /plugin install superpowers@superpowers-marketplace
```

### 4. Open Claude Code in the project folder and run the bootstrap prompt

```
Read this template repo end to end, in this order:
  1. AI_NATIVE_CONSTITUTION.md
  2. CLAUDE.md
  3. docs/AI_NATIVE_PLAYBOOK.md
  4. docs/BORIS_PLAYBOOK.md
  5. docs/MULTI_AGENT_TEMPLATE.md
  6. docs/SUPERPOWERS_INTEGRATION.md
  7. docs/MEMPALACE_INTEGRATION_GUIDE.md
  8. docs/Token Budget.md, docs/Founder Discipline.md
  9. docs/Closed Loops Inventory.md
  10. specs/README.md, tests/scenarios/README.md, agents/README.md, runbooks/README.md

Then critical-review docs/MULTI_AGENT_TEMPLATE.md per its §1 self-instruction
(don't apply blindly), and adapt the template to my new project:

  Project name: <name>
  One-paragraph product description: <description>
  Stack (if known): <stack>
  Top constraints: <constraints>

Fill the placeholders in CLAUDE.md (§1-§8, §14, §18).
Replace AI_NATIVE_PLAYBOOK.md §11 with this project's specific application.
Fill the empty rows in docs/Closed Loops Inventory.md.
Replace agents/README.md's agent inventory with this project's actual agents (or remove if not LLM-driven).
Update REQUIREMENTS.md to this project's real stack.

Produce diffs only. I'll review and approve before any merge.
Follow CLAUDE.md §20 (Response Template) for your reply.
```

Claude Code adapts the entire foundation in one or two passes. After approval, you have a fully personalized repo and can immediately move to Sprint 1 in spec-first mode.

---

## What's inside

```
.
├── README.md                          ← this file
├── AI_NATIVE_CONSTITUTION.md          ← 9 binding principles + decision checklist
├── CLAUDE.md                          ← generic operating manual; placeholders for project specifics
├── REQUIREMENTS.md                    ← generic stack/setup skeleton
├── CHANGELOG.md                       ← starter
├── mempalace.yaml                     ← starter wing/room layout (9 rooms)
├── .gitignore
├── .env.example
│
├── docs/
│   ├── AI_NATIVE_PLAYBOOK.md          ← long-form reference; §11 is project-specific
│   ├── BORIS_PLAYBOOK.md              ← Boris Cherny's 11-step Claude workflow
│   ├── MULTI_AGENT_TEMPLATE.md        ← multi-agent master template (review-first)
│   ├── SUPERPOWERS_INTEGRATION.md     ← how Superpowers skills integrate with this stack
│   ├── MEMPALACE_INTEGRATION_GUIDE.md ← install + structure + diary patterns
│   ├── Token Budget.md                ← weekly token-vs-outcomes ratio ritual
│   ├── Founder Discipline.md          ← weekly conviction-check ritual
│   └── Closed Loops Inventory.md      ← 5 buckets, fill rows per project
│
├── specs/                             ← spec-first source of truth per feature
│   ├── README.md
│   └── _TEMPLATE.md
│
├── tests/scenarios/                   ← Given/When/Then scenarios per feature
│   ├── README.md
│   └── _TEMPLATE.md
│
├── agents/                            ← LLM agent definitions (prompts, policies, tools, thresholds)
│   ├── README.md
│   └── _TEMPLATE/
│       └── README.md
│
└── runbooks/                          ← incident-response procedures, decisions-not-narrative
    ├── README.md
    └── _TEMPLATE.md
```

---

## Philosophy in five lines

1. AI is the operating system the company runs on, not a tool we use.
2. Every important process is a closed loop with an artifact and a metric.
3. Specs and tests are the source of truth; agents write the code.
4. No human middleware; one DRI per outcome.
5. The founder leads from the front and never delegates AI strategy.

Full treatment in `AI_NATIVE_CONSTITUTION.md` and `docs/AI_NATIVE_PLAYBOOK.md`.

---

## Three rituals you don't skip

- **Weekly Founder Discipline entry** — `docs/Founder Discipline.md`. New capability that landed; prior that broke. 15 minutes, every Monday.
- **Weekly Token Budget entry** — `docs/Token Budget.md`. Spend / shipped outcomes ratio. 15 minutes, every Monday.
- **Quarterly Closed Loops review** — walk `docs/Closed Loops Inventory.md`; flag any decayed loops back to 🟡.

These are non-negotiable. Skipping them collapses the whole system within a quarter.

---

## How to update this template itself

When you ship something good in a real project that should propagate to the template:

1. Identify the file (constitution, playbook, addendum, inventory format, etc.).
2. Open a PR in this repo with the change.
3. Update `CHANGELOG.md` with the date and rationale.
4. Future projects pick it up automatically when they're created from the template.

Don't copy-edit the template from inside a downstream project — changes there don't propagate. Bring them back here.

---

## License

Private. All rights reserved.
