# Token Budget

> **How we track AI token spend against shipped outcomes, weekly.** This is principle 8 (token-maxing) made operational.
>
> See `AI_NATIVE_CONSTITUTION.md` (principle 8) and `docs/AI_NATIVE_PLAYBOOK.md` section 8.

---

## Why this exists

At an AI-native company, the unit economic question is not "are we spending too much on AI?". It's:

```
tokens converted into shipped outcomes per week
```

A high token bill that produces a high outcome rate is the goal. A low token bill with a low outcome rate is failure dressed up as frugality. We track both numbers explicitly so the trend is visible and we can correct fast.

---

## What counts as a token

All AI provider spend the company incurs:

- Anthropic (Claude in chat, Claude Code, future internal agents).
- OpenAI (AM, internal agents, embeddings).
- Retell AI / voice models (when applicable).
- Braintrust eval costs.
- Any other LLM provider used for the product or development.

We convert all spend to USD weekly. Tokens are the conceptual unit; dollars are the practical one.

---

## What counts as a shipped outcome

Only artifacts that produce client-facing or measurable internal value. Examples:

**Engineering outcomes:**
- A feature merged with spec + scenarios + DoD checked.
- An agent prompt update that lifted the eval score above its threshold.
- A runbook drafted from an actual incident.
- A documented decision (MemPalace triple + spec/doc updated).
- A successful automated artifact (auto-CHANGELOG, auto-retro, etc.) once the automation lands.

**Operational outcomes:**
- A client onboarded.
- A monthly report delivered.
- An incident resolved with post-mortem written.
- A calibration cycle completed (drift identified -> fixes deployed).

**GTM outcomes:**
- A demo booked.
- A pilot signed.
- A landing page shipped or iterated.
- A SEO post published with metadata complete.

What does **not** count: conversations that produced no artifact; drafts not committed; exploratory token spend with nothing to show.

---

## Weekly entry format

Every Monday morning (or whenever the week resets, but consistently), add an entry below in this format:

```
## Week of YYYY-MM-DD

### Spend
- Anthropic: $X
- OpenAI: $Y
- Retell: $Z
- Braintrust: $W
- Other: $V
- **Total: $T**

### Outcomes shipped (count + brief)
- N engineering outcomes:
  - <one-line description, link to PR or doc>
  - ...
- N operational outcomes:
  - <one-line description>
  - ...
- N GTM outcomes:
  - <one-line description>
  - ...

### Ratio
- $T / total outcomes = $X per outcome
- Trend vs last 4 weeks: <up / flat / down> + one-sentence interpretation.

### Notes
- Anything unusual: spike, drop, deliberate experiment, broken loop discovered.
```

---

## Trend rules

What we look for:

- **Spend up, outcomes up, ratio improving:** healthy growth. Continue.
- **Spend up, outcomes flat or down, ratio degrading:** a loop is open somewhere. Find it: which week did the ratio break? What changed? Usually a process that went from automated to manual, or a feature that's burning tokens without producing artifacts. Fix the loop, not the budget.
- **Spend down, outcomes down:** comfort zone. Founder discipline check (`docs/Founder Discipline.md`). Probably need a deliberate token-spend experiment.
- **Spend down, outcomes up, ratio improving fast:** great in the short term, suspicious if sustained. Often means we're shipping smaller things — verify outcome quality hasn't degraded.

---

## Alerts (informal, until automated)

The founder reads the trend each week. If any of the following:

- Ratio doubled in one week.
- Three weeks of "spend up, outcomes flat".
- Total spend exceeds $X (initial threshold: founder sets explicitly each quarter).

The founder schedules a 30-minute review with himself, in the calendar, with a specific output: a one-paragraph diagnosis logged in MemPalace `decisions` room.

---

## Weekly entries

Newest entries at the top.

---

*(First entry will be added the next Monday after this file is created. Pre-history token spend is unmeasured and stays so — we start the clock when the discipline starts.)*

---

## Future automation

This is currently a manual ritual. Targets for automation, in priority order:

1. **Auto-import of provider spend.** A scheduled function pulls the previous week's bill from each provider and pre-fills the spend section.
2. **Auto-count of outcomes.** Same function reads the prior week's merges, MemPalace entries with specific tags, and operational counters, and pre-fills the outcomes section.
3. **Trend alerting.** A weekly cron compares the new ratio to the trailing 4-week average and pages the founder if any alert condition trips.

Until these land, the manual entry is the discipline. It takes ~15 minutes a week. Skipping it means losing the signal.

---

*Maintainer: the founder. Weekly entries are not delegated.*
