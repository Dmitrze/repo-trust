---
feature: <kebab-case-feature-name>
status: draft   # draft | proposed | accepted | shipped | deprecated
dri: <name or @handle>
created: <YYYY-MM-DD>
updated: <YYYY-MM-DD>
related_agents: []        # e.g. ["voice-ai", "booking-agent"]
related_scenarios: []     # e.g. ["tests/scenarios/<feature>.md"]
related_runbooks: []      # e.g. ["runbooks/<scenario>.md"]
related_docs: []          # e.g. ["docs/Technical Blueprint.md#section"]
---

# <Feature Name>

> One-paragraph summary that a Claude Code session can read in 30 seconds and know what this is.

---

## 1. Goal

One sentence: **who benefits, what changes for them, how we will know it worked.**

---

## 2. Non-functional requirements

List only those that materially constrain the design. Quantify where possible.

- **Latency:** e.g. first token < 500ms; total response p95 < 4s.
- **Accuracy:** e.g. tool-call argument correctness >= 95% on the eval set.
- **Throughput:** e.g. 100 concurrent sessions per Edge Function instance.
- **Cost:** e.g. < $0.04 per request on average.
- **Accessibility:** e.g. keyboard nav, ARIA labels, contrast WCAG AA.
- **Security / privacy:** e.g. PII redacted before LLM call; RLS-scoped reads; audit log on every tool call.
- **Browser / device:** e.g. mobile-first, 320px minimum width, iOS Safari 15+.

---

## 3. Boundaries

### In scope

- ...

### Out of scope (explicit)

- ...

Being explicit about what's *out* prevents scope creep during implementation.

---

## 4. Probabilistic satisfaction threshold (LLM features only)

Delete this section if the feature is not LLM-driven.

- **Eval suite:** path or Braintrust project name; how many examples; how it was constructed.
- **Judge rubric:** the rubric the LLM judge uses; link to the rubric file.
- **Threshold:** numeric score below which `agent_version` cannot deploy. Example: judge mean >= 0.85, no individual scenario below 0.7.
- **Drift detection:** how we detect threshold violations in production.
- **Action on threshold breach:** automatic rollback to last known good `agent_version`; ops alerted; root-cause analysis logged.

---

## 5. Happy-path scenario

One concrete user flow described step by step. Detailed enough that an implementer can build the skeleton from this section alone.

1. ...
2. ...
3. ...

*Edge cases and failure modes belong in `/tests/scenarios/<feature>.md`, not here.*

---

## 6. Architecture sketch

Optional but encouraged. A 5-10 line ASCII diagram or a short list of components, or both.

```
[ Component A ] -> [ Component B ] -> [ Component C ]
      |                  |
      v                  v
[ Storage ]       [ External API ]
```

Reference the relevant section of `docs/Technical Blueprint.md` for the full architecture; this section is just the local picture.

---

## 7. Closed loop

This is mandatory. The feature does not ship without a closed loop.

- **Goal metric:** the single number that tells us this feature is doing its job.
- **Where it lives:** which dashboard / view / report surfaces it.
- **Who reads it:** which agent or human reviews it on what cadence.
- **Improvement path:** how the next iteration of the feature will use what we learn from the metric.

---

## 8. Definition of Done

Binary checklist. The feature is not shipped if any of these is unchecked.

- [ ] Spec status is `accepted`.
- [ ] `tests/scenarios/<feature>.md` exists and lists at least 5 concrete scenarios.
- [ ] Implementation merged via a PR that links this spec.
- [ ] Automated tests (Vitest unit, Playwright E2E where relevant) pass.
- [ ] (LLM features) Probabilistic satisfaction threshold met on the eval set.
- [ ] Closed-loop metric is on the relevant dashboard.
- [ ] `CHANGELOG.md` entry added.
- [ ] Relevant `docs/` files updated (or noted not to need updates).
- [ ] MemPalace diary entry written for the relevant room.
- [ ] No new `any` introduced; build passes typecheck.
- [ ] Mobile (320px+) verified.
- [ ] No secrets in the diff.

---

## 9. Open questions

- ...
- ...

When a question is resolved, move it to the closed-questions section below with the answer and the date.

---

## 10. Closed questions (history)

Format: `YYYY-MM-DD — question — resolution — link to decision artifact (MemPalace triple, PR, etc.).`

- ...

---

## 11. References

- `AI_NATIVE_CONSTITUTION.md` (root) — binding principles.
- `docs/AI_NATIVE_PLAYBOOK.md` section 6 — software factory flow.
- `docs/Technical Blueprint.md` section X — architecture context.
- `tests/scenarios/<feature>.md` — the concrete scenarios for this feature.
- `agents/<agent-name>/` — agent prompt + policy + tool schema (LLM features only).
