---
feature: <kebab-case-feature-name>
status: draft   # draft | proposed | accepted | shipped | deprecated
spec: ../../specs/<feature>.md
dri: <name or @handle>
created: <YYYY-MM-DD>
updated: <YYYY-MM-DD>
---

# <Feature Name> — Scenarios

Link: [`specs/<feature>.md`](../../specs/<feature>.md)

> Concrete success and failure cases for this feature. Implementations are iterated against this file until all scenarios pass at the threshold defined in the spec.

---

## Coverage at a glance

| Category | Count | Notes |
|---|---|---|
| Happy path | 0 | |
| Edge cases | 0 | |
| Failure modes | 0 | |
| Accessibility | 0 | |
| Performance | 0 | |
| Security / privacy | 0 | |

Fill these in once scenarios are written. Aim for the rough mix described in `tests/scenarios/README.md`.

---

## Happy path

### S-001: <title>

**Given** <preconditions>
**When** <action>
**Then** <observable outcome>

Notes:
- (optional)

---

### S-002: <title>

**Given** ...
**When** ...
**Then** ...

---

## Edge cases

### S-101: <title>

**Given** ...
**When** ...
**Then** ...

---

### S-102: <title>

**Given** ...
**When** ...
**Then** ...

---

## Failure modes

### S-201: <title> (<external dependency> unavailable)

**Given** <external dep> is down
**When** <user action>
**Then** the system <degrades gracefully / shows error / queues for retry>; the user sees <copy>; an artifact is logged in <where>.

---

### S-202: malformed input

**Given** ...
**When** ...
**Then** ...

---

## Accessibility

### S-301: keyboard navigation

**Given** the user is using only a keyboard
**When** they navigate the feature
**Then** every interactive element is reachable in a logical tab order; focus is always visible; no keyboard traps.

---

### S-302: screen reader announces state

**Given** ...
**When** ...
**Then** ...

---

## Performance

### S-401: latency under typical load

**Given** <typical-load definition>
**When** the user performs <action>
**Then** p95 first token < <latency from spec>; p99 total response < <latency from spec>.

---

## Security / privacy

### S-501: RLS isolation

**Given** a user with `tenant_id = A`
**When** they request data
**Then** they receive only rows where `tenant_id = A`; no rows from other tenants are returned at any layer (Edge Function, response, logs).

---

### S-502: PII redaction before LLM call

**Given** the user submits text containing email and phone
**When** the request is sent to the LLM
**Then** the LLM sees redacted placeholders (`{EMAIL_A}`, `{PHONE_A}`); the original PII is restored only in the user-visible response, not in logs.

---

## Deprecated scenarios (historical)

Keep removed scenarios here for context. Format:

- `S-NNN — <title> — deprecated YYYY-MM-DD — reason — link to decision artifact.`

---

## How an agent reads this file

1. Match each scenario against current implementation behavior.
2. For any failing scenario, decide: fix the code, fix the scenario, or escalate to the human (spec change).
3. Run the corresponding test code (Vitest / Playwright / Braintrust) and report pass/fail per scenario.
4. Do not silently skip scenarios.
