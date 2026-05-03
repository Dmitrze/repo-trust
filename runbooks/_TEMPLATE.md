---
scenario: <kebab-case-scenario>
severity: <sev1 | sev2 | sev3>
dri: <name or @handle>
on_call: <how to reach the on-call: PagerDuty schedule, phone, Slack channel>
related_specs: []
related_scenarios: []
related_runbooks: []
created: <YYYY-MM-DD>
updated: <YYYY-MM-DD>
last_dry_run: <YYYY-MM-DD>
---

# <Scenario Name>

> One sentence describing the failure this runbook addresses, written for the tired ops person at 2 AM.

---

## 1. When to use this runbook

Use this runbook when **any** of these triggering signals appear:

- Alert: `<alert name in Sentry / Datadog / Inngest / etc.>`
- Symptom: `<observable user-visible symptom>`
- Dashboard: `<which dashboard widget shows what value>`
- Client report: `<"the client said X">`

Do **not** use this runbook for:

- ...
- ...

If the situation seems related but doesn't match the signals above, check `related_runbooks` for a closer match.

---

## 2. Severity

This incident class is **<Sev1 | Sev2 | Sev3>** by default. Justification: <one sentence>.

Upgrade to higher severity if:

- ...
- ...

Downgrade to lower severity only after:

- ...

---

## 3. First five minutes

Do these in order. Do not skip steps. Do not improvise.

1. **Acknowledge.** Acknowledge the page within 5 minutes. Post in `#incidents` channel: "On it: <one-line summary>."
2. **Snapshot.** Capture the state right now: current dashboard reading, error count, affected tenant IDs, time of first symptom.
3. **Stabilize.** <The minimum action that stops the bleeding while you diagnose. E.g., "flip feature flag X to false", "pause the cron", "reroute traffic to fallback".>
4. **Inform.** If client-visible: notify the affected tenant(s) via the AM with a templated message (template at the bottom of this runbook).
5. **Page.** If Sev1: notify the founder. If Sev2 and you're stuck after step 3: notify the founder.

---

## 4. Diagnosis

A decision tree to identify the root cause. Each branch produces a hypothesis to confirm.

```
Is <signal A> present?
  Yes -> hypothesis: <root cause 1>. Confirm by <check>.
  No  -> Is <signal B> present?
           Yes -> hypothesis: <root cause 2>. Confirm by <check>.
           No  -> Is <signal C> present?
                    Yes -> hypothesis: <root cause 3>.
                    No  -> escalate to founder; this runbook is incomplete.
```

For each hypothesis, the confirming check is a concrete command or UI action, not "investigate logs".

---

## 5. Resolution paths

One path per hypothesis confirmed in section 4.

### Path A: <root cause 1>

**Required tools / access:**
- ...

**Steps:**
1. ...
2. ...
3. ...

**Expected outcome:** <what should change in the dashboard / log>

---

### Path B: <root cause 2>

...

---

### Path C: <root cause 3>

...

---

## 6. Verification

After resolution, confirm all of the following:

- [ ] The triggering signal is no longer firing.
- [ ] The dashboard widget that reflects this incident class shows healthy values for at least 15 minutes.
- [ ] No new errors of this class in the last 30 minutes.
- [ ] Affected tenants have been notified of resolution via the AM.
- [ ] If a feature flag or rollback was used: it has been documented in the post-mortem.

---

## 7. Post-incident (mandatory)

Within 24 hours of resolution:

- [ ] **Post-mortem** drafted in `docs/post-mortems/<YYYY-MM-DD>-<scenario>.md` (this directory will be created on first incident). Use the standard template: timeline, root cause, contributing factors, what worked, what didn't, action items.
- [ ] **MemPalace** entry in the `ops` room: one paragraph summary + link to the post-mortem.
- [ ] **CHANGELOG.md** entry if a fix was deployed.
- [ ] **Spec / scenarios update** if the incident revealed missing scenarios in `tests/scenarios/<feature>.md` — add the new scenarios.
- [ ] **This runbook update** — if any step in this runbook was wrong or missing, fix it now while it's fresh.
- [ ] **Drift detection** — if the incident points to a closed loop that decayed, update `docs/Closed Loops Inventory.md`.

---

## 8. Client communication template (use only if client-visible)

The AM sends this on behalf of ops. Edit only the bracketed fields.

```
Hi [client first name] — we detected an issue with [affected feature]
at [time]. The team is on it; current ETA to full restore is [estimate].
[N] [interactions / calls / reviews] were affected; we'll follow up
individually on each within [time]. I'll update you the moment we're
back to normal.
```

Do not send a message that includes anything not in this template without ops approval.

---

## 9. Related

- Specs: `specs/<related-feature>.md`
- Scenarios: `tests/scenarios/<related-feature>.md`
- Runbooks: `runbooks/<related-runbook>.md`
- Dashboard: <link>
- Post-mortem template: `docs/post-mortems/_TEMPLATE.md` (created on first incident).
