# Closed Loops Inventory

> **Living dashboard of every important process in this project and its loop status.** Updated continuously — by Claude Code as it builds new features, by the founder during weekly review, by internal calibration agents when they detect drift.
>
> See `AI_NATIVE_CONSTITUTION.md` (principle 2) and `docs/AI_NATIVE_PLAYBOOK.md` section 3 for the framing.
>
> **A process is a closed loop only if it satisfies all five conditions** (`docs/AI_NATIVE_PLAYBOOK.md` section 3.2): goal in writing, action produces machine-readable artifact, artifact lands in index, an agent reads it, metric measured automatically.

---

## How to read this document

Each process row has:

- **Process** — what it is.
- **Status** — `closed` (green) / `partial` (yellow) / `open` (red) / `not yet built` (white).
- **Goal artifact** — where the goal is written.
- **Action artifact** — what each execution produces.
- **Index** — where the artifact lands.
- **Reader agent** — the agent (or human) that reads the artifact.
- **Metric** — the measured outcome.
- **Improvement path** — how the next iteration uses what we learn.
- **DRI** — the one person responsible.
- **Notes** — anything else.

When a row goes from yellow to green, log a decision artifact and add it to `CHANGELOG.md`.

---

## How to fill this template for a new project

This file ships with the five buckets (A-E) below, each with empty rows. For your project:

1. **Bucket A — Product surfaces.** List every customer/user-facing feature that has its own loop. For each, fill the row with the spec path, what artifact each execution produces, where it lands, who reads it, what metric is tracked, and the improvement path.
2. **Bucket B — Internal operations.** List the internal processes that run the company (onboarding, calibration, incident response, reporting, expansion).
3. **Bucket C — Engineering process.** Standardized across projects: feature delivery, sprint planning, code review, CHANGELOG. Adjust to your project's reality.
4. **Bucket D — GTM and customer feedback.** Sales call loop, pilot feedback loop, marketing/content conversion loop.
5. **Bucket E — Founder operations.** Founder-discipline ritual, token-budget ritual.

Delete the example placeholder rows below; fill in your project's actual processes.

---

## A. Product surfaces

### A.1 <Surface name>

- **Status:** white (not yet built)
- **Goal artifact:** `specs/<feature>.md`
- **Action artifact:** <e.g. database row + log + trace>
- **Index:** <where the artifact lands>
- **Reader agent:** <agent name or human DRI>
- **Metric:** <single number that defines success>
- **Improvement path:** <how the next iteration learns>
- **DRI:** <name>

### A.2 <Surface name>

*(repeat structure)*

---

## B. Internal operations

### B.1 Onboarding

- **Status:** white
- **Goal artifact:** `specs/onboarding.md`
- **Action artifact:** intake record + provisioning checklist + kickoff event
- **Index:** database + ops console
- **Reader agent:** Onboarder agent
- **Metric:** time from signed order to live (target <= N days), completion %
- **Improvement path:** vertical templates evolve
- **DRI:** <name>

### B.2 Calibration cycle (weekly)

- **Status:** white
- **Goal artifact:** `specs/calibrator.md` + `agents/calibrator/`
- **Action artifact:** weekly drift report + proposed prompt diffs
- **Index:** database + Braintrust + ops console
- **Reader agent:** ops human
- **Metric:** drift score per agent, time-to-resolve
- **Improvement path:** Calibrator proposes -> ops approves -> measured next cycle
- **DRI:** <name>

### B.3 Incident response

- **Status:** white
- **Goal artifact:** `runbooks/*.md`
- **Action artifact:** incident ticket + post-mortem + runbook update
- **Index:** database + `runbooks/` directory + decision log
- **Reader agent:** Incident Responder + post-mortem-generator
- **Metric:** MTTA, MTTR, Sev1+Sev2 count per month
- **Improvement path:** every incident updates a runbook; quarterly dry-run
- **DRI:** <name>

### B.4 Monthly reporting

*(if applicable to your project)*

### B.5 Expansion / growth signals

*(if applicable)*

---

## C. Engineering process

### C.1 Feature delivery (spec -> scenarios -> code -> ship)

- **Status:** yellow (partial) — process defined, application begins with first feature
- **Goal artifact:** `AI_NATIVE_CONSTITUTION.md` + `CLAUDE.md` section 20 + `specs/_TEMPLATE.md`
- **Action artifact:** PR with linked spec + scenarios + CHANGELOG entry
- **Index:** GitHub + this repo's `specs/` + `tests/scenarios/`
- **Reader agent:** Reviewer agent + human reviewer
- **Metric:** % of merged PRs that link a spec; spec/code coverage ratio
- **Improvement path:** post-merge agent flags PRs without specs
- **DRI:** <founder name>

### C.2 Sprint planning + retrospective

- **Status:** red (open)
- **Goal artifact:** `docs/Roadmap.md` (if you have one)
- **Action artifact:** `docs/sprints/SPRINT-N-retro.md`
- **Index:** GitHub + decision log
- **Reader agent:** sprint-planner agent (build after enough signal)
- **Metric:** shipped vs planned ratio per sprint
- **Improvement path:** retro feeds next sprint's spec list
- **DRI:** <founder name>

### C.3 Code review

- **Status:** yellow
- **Goal artifact:** `CLAUDE.md` section 11 + the constitution section 20.3
- **Action artifact:** review comments on PR
- **Index:** GitHub PRs
- **Reader agent:** automated review agent + humans
- **Metric:** % of PRs merged with no review-blocking issues
- **Improvement path:** Reviewer rubric evolves; spec-template evolves
- **DRI:** <founder name>

### C.4 CHANGELOG maintenance

- **Status:** yellow — file created; auto-generation pending
- **Goal artifact:** `CHANGELOG.md` (root)
- **Action artifact:** entries on every merge
- **Index:** root file + GitHub commits
- **Reader agent:** post-merge changelog-generator (to build)
- **Metric:** % of merges with CHANGELOG entries
- **Improvement path:** automate via GitHub Action
- **DRI:** <founder name>

---

## D. GTM and customer feedback

### D.1 Sales call -> deal closed

- **Status:** red / white
- **Goal artifact:** `docs/GTM Strategy.md` (if exists)
- **Action artifact (target):** recording + transcript + summary + deal stage update
- **Index (target):** customer-feedback inbox + AI notetaker storage
- **Reader agent (target):** GTM analyzer
- **Metric:** win rate, time-to-close, common objection clusters
- **Improvement path:** outbound copy + landing page + ICP refinement
- **DRI:** <founder name>

### D.2 Pilot feedback

- **Status:** white
- **Goal artifact:** `docs/Customer Journey.md`
- **Action artifact:** every signal (DM, call, ticket, NPS) captured queryable
- **Index:** customer-feedback inbox + database
- **Reader agent:** AM (or equivalent) + ops + Calibrator
- **Metric:** NPS proxy, time-to-resolution, churn risk score
- **Improvement path:** signals feed spec/scenario/prompt iteration
- **DRI:** <founder name>

### D.3 Marketing / content conversion

- **Status:** yellow / white
- **Goal artifact:** `docs/Marketing Playbook.md`
- **Action artifact:** weekly metrics + content iteration log
- **Index:** analytics + GitHub commits to landing pages
- **Reader agent:** content-iteration agent
- **Metric:** unique visitors -> conversion ratio per surface
- **Improvement path:** A/B copy + page-element changes
- **DRI:** <founder name>

---

## E. Founder operations

### E.1 Founder weekly conviction check

- **Status:** yellow — ritual defined, execution starts immediately
- **Goal artifact:** `docs/Founder Discipline.md`
- **Action artifact:** weekly entry in that file
- **Index:** docs file + decision log
- **Reader agent:** the founder
- **Metric:** "new capability landed" entry every week; 3 weeks blank -> deliberate experiment
- **Improvement path:** broken priors compound into product/process patterns
- **DRI:** <founder name>

### E.2 Token budget -> outcomes ratio

- **Status:** yellow
- **Goal artifact:** `docs/Token Budget.md`
- **Action artifact:** weekly entry: tokens spent / outcomes shipped
- **Index:** docs file + provider billing dashboards
- **Reader agent:** founder; later, a token-budget agent
- **Metric:** trend of `tokens / shipped outcome` (down = good)
- **Improvement path:** exposes open loops where tokens go without outcomes
- **DRI:** <founder name>

---

## Maintenance protocol

This document is updated:

- **Every time a process changes status** (open -> partial -> closed). Logged in the decision log and noted in `CHANGELOG.md`.
- **Every time a new process is introduced.** New row added.
- **Every quarter** — founder reviews the whole document; flags decayed loops as yellow again if they've drifted.
- **Every time a Calibrator detects drift** in an agent that affects a closed loop — row updated to reflect actual state.

*Maintainer: the founder. Last material change: when the template is adapted to a new project, replace this with the date.*
