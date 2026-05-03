# /runbooks — Operational Procedures

> **What to do when things go wrong, in step-by-step form.** A runbook is the artifact a tired ops person reads at 2 AM and follows without judgment calls. If a runbook requires improvisation, it is incomplete.
>
> See `AI_NATIVE_CONSTITUTION.md` (principle 2, principle 6) and `docs/AI_NATIVE_PLAYBOOK.md` section 11.2.

---

## What goes here

One file per scenario, named `<kebab-case-scenario>.md`. Examples (created over time, as scenarios appear):

- `voice-ai-down.md` — voice agent or its provider is failing calls.
- `gbp-review-publish-failure.md` — a Review Manager auto-publish to Google Business Profile failed.
- `pii-leak-suspected.md` — a PII redaction failure or potential exposure was detected.
- `agent-version-rollback.md` — how to roll back a deployed agent_version that's underperforming.
- `tenant-data-isolation-breach-suspected.md` — RLS misconfiguration or leak suspected.
- `stripe-invoice-failure.md` — client invoice generation or charge failed.
- `oauth-token-expired.md` — a tenant's Google or Twilio OAuth token expired and integrations are failing.
- `monthly-report-not-generated.md` — the Reporter agent failed to produce a tenant's monthly report.
- `onboarding-stuck.md` — a tenant is past day N of the SLA without progress.
- `eval-suite-broken.md` — Braintrust eval suite is throwing errors and blocking deploys.

This list grows as we encounter incident classes. Every Sev1/Sev2 incident gets a runbook (or updates an existing one) before the post-mortem closes.

---

## What does *not* go here

- General architecture descriptions — `docs/Technical Blueprint.md`.
- Spec-level documentation — `specs/`.
- Development tasks (e.g., "how to add a new vertical") — those are spec'd in `specs/` or written as a how-to in `docs/`.
- One-off notes from a specific incident — those go in MemPalace `ops` room or in the post-mortem doc; runbooks are reusable.

---

## Format

Start from `_TEMPLATE.md`. Every runbook has:

1. **When to use this runbook** — the triggering signal, the symptom, the alert text.
2. **Severity** — Sev1 / Sev2 / Sev3 with definitions.
3. **First five minutes** — the steps to take immediately, ordered, no ambiguity.
4. **Diagnosis** — a decision tree to identify root cause.
5. **Resolution paths** — one path per likely root cause, with concrete commands or UI steps.
6. **Verification** — how you know the issue is resolved.
7. **Post-incident** — the artifacts that must exist after resolution: post-mortem, MemPalace entry, CHANGELOG entry, spec/scenario update if behavior changed.
8. **Related** — links to specs, scenarios, related runbooks, dashboards.

Key principle: **decisions, not narrative.** A runbook is a sequence of decisions and actions. It does not explain why the system was designed that way — that's `docs/`. It does not philosophize about reliability — that's playbooks.

---

## Lifecycle

Runbooks are living documents:

- A new runbook is created during or immediately after a Sev1/Sev2 incident, even if rough.
- Every subsequent incident of the same class refines the runbook — either confirming the steps or correcting them.
- Runbooks are tested: every quarter, the on-call ops person dry-runs at least one randomly selected runbook against staging to verify it still works.
- A runbook that hasn't been updated in 12 months gets a `stale` flag at the top until it's reviewed.

---

## How agents use this directory

When the Incident Responder agent fires:

1. It identifies the incident class from the alert signal.
2. It looks up the matching runbook in this directory.
3. It pages the on-call human and includes a link to the runbook in the page.
4. It optionally pre-executes any read-only diagnosis steps and includes findings in the page ("I checked X, here's what I found").
5. The human follows the runbook.
6. The agent watches for the resolution signal and confirms it on the dashboard.

When Claude Code is asked to design a new feature that introduces new failure modes:

1. It identifies the new failure modes (covered in `tests/scenarios/<feature>.md` failure-modes section).
2. For each Sev1/Sev2-class failure mode, it proposes a runbook.
3. The runbook lands in the same PR as the feature.

---

## How humans use this directory

- On-call rotation: read every runbook before your first shift.
- During an incident: open the runbook, follow it. Resist the urge to improvise; if you must improvise, take notes and update the runbook afterward.
- Post-incident: spend 5 minutes updating the runbook with what you learned. This is mandatory, not optional.

---

## Severity definitions (used across all runbooks)

- **Sev1** — production-impacting for multiple tenants OR data integrity OR security: page immediately; founder is notified; resolution time target < 1 hour.
- **Sev2** — production-impacting for one tenant OR a major feature degraded: page in business hours, ack within 30 min; resolution time target < 4 hours.
- **Sev3** — minor degradation, no client-visible failure, recoverable on next cycle: log a ticket; resolution within 1 sprint.

Incidents that don't fit these definitions get a Sev3 default and a discussion in the post-mortem about whether the definitions need updating.

---

*Maintainer: the founder. The DRI for any individual runbook is named in that runbook's frontmatter (typically the on-call lead).*
