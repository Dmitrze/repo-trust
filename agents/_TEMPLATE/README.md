# <Agent Name>

> **One paragraph: what this agent does, who it serves, and the single most important thing it must get right.**

Spec: [`specs/<agent-name>.md`](../../specs/<agent-name>.md)
Scenarios: [`tests/scenarios/<agent-name>.md`](../../tests/scenarios/<agent-name>.md)
Status: `draft` | `proposed` | `accepted` | `shipped` | `deprecated`
DRI: <n>
Threshold: see `threshold.md`

---

## Files in this directory

- `README.md` — this file. Quick orientation.
- `prompt.md` — system prompt; current file = intended current production prompt.
- `policy.md` — hard rules, escalation triggers, refusals.
- `tools.json` — zod-derived tool schema, source of truth.
- `threshold.md` — probabilistic satisfaction threshold + drift policy.
- `eval-set/examples.jsonl` — curated examples for Braintrust.
- `eval-set/rubric.md` — LLM judge rubric.
- `changelog.md` — per-agent change history with eval deltas.

---

## When this agent is invoked

<List the triggers: inbound call, GBP review event, scheduled cron, AM tool call, etc.>

---

## What it must never do

This duplicates the binding rules from `policy.md` for quick reference. Full version: `policy.md`.

- ...
- ...

---

## Closed-loop summary

- **Goal metric:** <single number>
- **Where it lives:** <dashboard / report>
- **Reviewer:** <agent name or human DRI>
- **Cadence:** <e.g. weekly Calibrator>
- **On drift:** <action: alert, auto-rollback, ops queue>

---

## Related

- Spec: `specs/<agent-name>.md`
- Scenarios: `tests/scenarios/<agent-name>.md`
- Runbook for incidents: `runbooks/<agent-name>-incident.md`
- Tech architecture: `docs/Technical Blueprint.md` section X
