# MemPalace Integration Guide

> **What this is.** MemPalace gives Claude Code persistent memory across sessions. Without it, every new session starts blank. With it, a new session wakes up with ~170 tokens of compressed context and queries the palace on demand. This guide tells you how to install it, organize memory, and — most importantly — the daily patterns that make it useful rather than just installed.

---

## 1. What MemPalace is (and is not)

**What it is.**
- A local-first AI memory system. All data lives on your machine; no cloud.
- A Python package (`mempalace`) + an MCP server exposing ~29 tools to Claude Code.
- A structured memory vault using the method-of-loci metaphor: **Wings -> Rooms -> Halls -> Drawers**. Plus a temporal knowledge graph for facts with validity windows.
- MIT-licensed, Python 3.9+. Project: github.com/milla-jovovich/mempalace.

**What it is not.**
- Not a production dependency of the project. It lives in the developer's local environment and never ships to production.
- Not a replacement for `docs/`. Documentation is the long-form reference; MemPalace is the working memory *between* documentation updates.
- Not a chat log. Diaries are short and structured; raw conversation text stays in the Claude Code session.
- Not a substitute for `CLAUDE.md`. The top-level instruction file stays lean; MemPalace holds the sprawling long-tail.

---

## 2. Why this project uses it

Three concrete problems MemPalace solves:

1. **Session continuity for Claude Code.** A new session opens, reads `CLAUDE.md`, and is still missing "what did we decide about X yesterday?". MemPalace's compressed wake-up gives an index of prior sessions so it can query specifics on demand.
2. **Decision log with rationale.** Architectural and product decisions with "why we chose X over Y" — the knowledge-graph layer preserves trade-offs in queryable form.
3. **Cross-session handoff.** When you switch from product work to engineering work, MemPalace ensures the engineering session knows what the product session decided — without re-explaining.

---

## 3. Install (5 minutes)

### 3.1 Install the Python package

```bash
pipx install mempalace
```

### 3.2 Initialize the palace in the repo

```bash
cd <project-root>
mempalace init .
```

This creates `.mempalace/` (gitignored) containing the local SQLite + ChromaDB index. The existing `mempalace.yaml` at the repo root is auto-detected.

### 3.3 Mine existing content

```bash
mempalace mine . --mode files
mempalace mine ~/Downloads/claude-export.json --mode convos --extract general
```

### 3.4 Register the MCP server with Claude Code

In Claude Code MCP config (`~/.claude/mcp.json` or via Claude Code settings):

```json
{
  "mcpServers": {
    "mempalace": {
      "command": "mempalace",
      "args": ["mcp-server"]
    }
  }
}
```

Restart Claude Code. You should see the `mempalace_*` tools available.

### 3.5 Verify

In a new Claude Code session: `mempalace_status` should show palace path, wing count, room count, drawer count.

---

## 4. Structure — `mempalace.yaml` explained

The canonical wing/room structure lives at `mempalace.yaml` in the repo root. The default starter has nine rooms:

| Room | What goes in it |
|---|---|
| **product** | Spec changes, tier definitions, pricing decisions, SLA commitments, feature-scope changes. |
| **technical** | Architecture, schema, RLS policies, Edge Functions, integrations. |
| **frontend** | React components, routes, design system, tokens, Tailwind config, shadcn patterns. |
| **ops** | Internal operations, agents, onboarding, calibration, QA, incidents. |
| **gtm** | Strategy, marketing, customer journey, vertical landing pages, outbound. |
| **brand** | Brand assets, logo, visual identity, tone of voice. |
| **decisions** | Architectural and product decisions with rationale, trade-offs, rejected alternatives. |
| **sessions** | Working session notes, checkpoints, what was completed, next steps, blockers. |
| **general** | Anything that doesn't match other rooms. |

The `decisions` and `sessions` rooms get the most writes — budget for them accordingly.

### 4.1 When to add a room

Only add a new room if a topic would produce 10+ drawers over the next month and doesn't fit an existing room.

---

## 5. Daily patterns

### 5.1 At session start

```
mempalace_search wing:<project> room:sessions "<feature or topic>"
mempalace_search wing:<project> room:decisions "<feature or topic>"
```

This gives the agent context from prior sessions and the relevant decisions before it touches code.

### 5.2 At session end (mandatory)

Write a diary entry for the wing/room you worked in. One to three sentences:

- What was done.
- What decisions were made.
- What's next / what's blocked.

This is the single discipline that matters. ~40 seconds per session. Skip it and the palace stops being useful within a week.

### 5.3 For architectural decisions

Add a knowledge-graph triple in the `decisions` room:

```
<project> -- chose -- <option> over <alternative> -- because <rationale>
```

When the fact changes, *invalidate* the entry (do not delete — history is preserved for audit).

---

## 6. Best practices

### 6.1 Write small, focused drawers

Good: one drawer per decision, per finding, per session summary. Titled specifically.

Bad: one large drawer titled "Thoughts on the backend" with 50 unrelated notes.

### 6.2 Use the right room

Before writing, ask: "in two months, if I search for this, what word will I use?" That word should be in the room description or keywords.

### 6.3 Prefer the graph for facts, drawers for narrative

- Facts -> graph triple. Queryable.
- Narrative ("why we decided") -> drawer. Readable.

Use both.

### 6.4 Write even when it feels obvious

The moment something feels obvious is the exact moment to write it, because a future session (or teammate) will not find it obvious.

### 6.5 Don't dump raw logs or large files

MemPalace is not an archive. Large documents go in `docs/` and are referenced from a drawer.

---

## 7. Anti-patterns

- **Mining then ignoring.** Running `mempalace mine` once and never using the palace afterward.
- **Writing to `general` by default.** The `general` room is a trash can. If things keep ending up there, your room structure needs revision.
- **Storing secrets.** API keys, tokens, passwords — never. MemPalace is gitignored but lives on your disk unencrypted.
- **Over-mining Claude conversations.** Mining a 100K-token chat dumps hundreds of mostly-noise drawers. Extract decisions manually or use `--extract general` to filter.
- **Duplicating `CLAUDE.md`.** Don't paste sections of CLAUDE.md into the palace. Reference it.
- **Skipping diary entries.** The single discipline that matters. Skip it and the palace stops being useful.

---

## 8. Troubleshooting

### `mempalace: command not found`

Python path issue. `pipx install mempalace`, or ensure `$(python -m site --user-base)/bin` is in `$PATH`.

### MCP server not appearing in Claude Code

Restart Claude Code fully. Check MCP config syntax. Test directly: `mempalace mcp-server --help`.

### `mempalace_search` returns no results

Was the palace mined? `mempalace status` shows drawer counts per room. Drop the `room` argument and search the whole wing. Try broader keywords.

### Palace seems corrupt after a crash

```bash
mempalace repair
# or, nuclear option:
rm -rf .mempalace && mempalace init . && mempalace mine .
```

Diary entries are the only non-reproducible content; export before nuking if you care.

---

## 9. Quick command reference

```bash
# Install
pipx install mempalace

# Initialize in repo
cd <project-root>
mempalace init .

# Mine existing content
mempalace mine .

# Search
mempalace search wing:<project> room:<room> "<query>"

# Status
mempalace status

# Repair
mempalace repair
```

For full tool reference, run `mempalace --help` or invoke `mempalace_*` MCP tools from Claude Code.
