# /specs — Feature Specifications

> **Specs are the source of truth for what gets built.** Every non-trivial feature in this repo starts as a spec in this directory. Code is downstream of the spec.
>
> See `AI_NATIVE_CONSTITUTION.md` (principle 5) and `docs/AI_NATIVE_PLAYBOOK.md` section 6 for why.

---

## What goes here

One file per feature, named `<kebab-case-feature>.md`.

## What does *not* go here

- Trivial bugfixes (typos, single-line CSS tweaks). Those go directly in a PR with a short description.
- Marketing copy (lives in `docs/Marketing Playbook.md` if applicable).
- Architectural decisions across multiple features (those go in technical docs and as MemPalace `decisions` triples).
- Operational runbooks (those live in `runbooks/`).

When in doubt: if a Claude Code session would benefit from reading this to implement the feature, it goes in `/specs`.

---

## Format

Start from `_TEMPLATE.md`. Every spec answers, at minimum:

1. **Goal** — one sentence: who benefits, what changes, how we know it worked.
2. **Non-functional requirements** — latency, accuracy, throughput, cost, accessibility, security.
3. **Boundaries** — what is in scope; what is explicitly out of scope.
4. **Probabilistic satisfaction threshold** (LLM features only) — the Braintrust judge score below which the feature does not deploy.
5. **Definition of Done** — the binary checklist that distinguishes "shipped" from "in progress".
6. **DRI** — the one person responsible for the outcome.
7. **Links** — to scenarios, related agents, related runbooks, related docs.
8. **Open questions** — explicit list of unresolved decisions.

The spec stays alive across iterations. When a feature changes, the spec changes first; the diff to the spec lands in the same PR as the diff to the code.

---

## Lifecycle

A spec moves through these states (tracked in the file's frontmatter):

- `draft` — being written. Do not implement against it yet.
- `proposed` — ready for review by the DRI and (for cross-cutting work) the founder.
- `accepted` — approved; implementation may begin.
- `shipped` — the feature is live in production; the spec describes current behavior.
- `deprecated` — the feature has been removed or replaced; the spec is retained for historical reference.

Agents implementing against a spec must verify it is `accepted` or `shipped` (for fixes / iterations).

---

## How agents use this directory

Claude Code, on receiving a non-trivial task, follows this loop:

1. Search this directory for an existing spec matching the feature.
2. If one exists: read it; if it's `draft` or `proposed`, do not implement — instead, propose moving it to `accepted` or flag missing pieces.
3. If none exists: create one starting from `_TEMPLATE.md`; mark it `draft`; surface it to the user for review.
4. Only after the spec is `accepted` does implementation begin.
5. After implementation, update the spec to `shipped` and link the PR.

---

## Relationship to scenarios (`/tests/scenarios/`)

A spec describes *what* the feature does and *how we know it's working* in prose. The scenarios in `/tests/scenarios/` describe *concrete success and failure cases* in a format an agent can iterate against. Both are required for non-trivial features.

- Spec answers: what is this, what's success, what's the threshold.
- Scenarios answer: given X, the system does Y; given malformed X, the system does Z.

---

*Maintainer: the founder. The DRI for any individual spec is the one named in that spec's frontmatter.*
