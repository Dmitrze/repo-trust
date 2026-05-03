# <PROJECT_NAME> — Development Requirements

Everything you need to run, build, and contribute to <PROJECT_NAME>.

> **Template note:** the items in `<ANGLE_BRACKETS>` are placeholders. Fill them on first use of this template against a new project.

---

## 1. System requirements

- **macOS 13+**, **Linux**, or **Windows with WSL2** (native Windows untested).
- **<runtime: Node.js 20 LTS / Python 3.11 / Go 1.22 / Rust stable / etc.>**.
- **<package manager: pnpm 8+ / uv / cargo / etc.>**.
- **Python 3.9+** (for MemPalace local memory layer — even if you only run `mempalace` via its CLI).
- **git 2.40+**.
- A **GitHub** account with access to `<owner>/<repo>` (private repo).

### Required third-party service accounts

<List the services this project uses. Common starters below; remove what doesn't apply.>

| Service | Purpose | Where to get keys |
|---|---|---|
| <Supabase> | Postgres, Auth, Edge Functions, Storage | https://supabase.com/dashboard |
| <OpenAI> | LLM | https://platform.openai.com/ |
| <Anthropic> | LLM (primary or fallback) | https://console.anthropic.com/ |
| <other> | <purpose> | <url> |

You do **not** need all of these to start development. Document which are required for which phase. See `CLAUDE.md` section 18 (Development phases priority).

---

## 2. Runtime dependencies

<Fill from your project's lockfile. Format:>

| Package | Version | Purpose |
|---|---|---|
| `<dep>` | `<ver>` | <purpose> |

## 3. Dev dependencies

<Same shape as runtime deps. Mark TypeScript / lint tooling / testing.>

---

## 4. Approved additions

Libraries that are not yet in `package.json`/equivalent but are pre-approved to add as needed:

| Package | Why |
|---|---|
| `<lib>` | <reason> |

Approval criterion: small footprint, actively maintained, solves a real problem the standard library doesn't.

---

## 5. Explicitly banned libraries

<List libraries that are forbidden in this project, with reasons. Examples:>

- **moment.js** — use `date-fns`.
- **axios** — use native `fetch`.
- **jQuery** — no.
- **lodash** — use native JS methods or small focused utilities.
- **CSS-in-JS (styled-components, emotion, stitches, vanilla-extract)** — use Tailwind only.

---

## 6. MemPalace (local AI memory)

MemPalace gives Claude Code persistent memory across sessions. It is Python-based but used transparently via its CLI and the MCP integration.

```bash
pipx install mempalace
mempalace init <path-to-project>
mempalace mine . --mode convos
```

MemPalace data lives in `./.mempalace` (gitignored). Structure is defined in `mempalace.yaml` at the repo root. See `docs/MEMPALACE_INTEGRATION_GUIDE.md` for full patterns.

---

## 7. Superpowers (Claude Code plugin)

Superpowers is a one-time install per developer machine, not per-project.

```text
# In Claude Code:
/plugin marketplace add obra/superpowers-marketplace
/plugin install superpowers@superpowers-marketplace
```

Then verify: open a new Claude Code session and ask for help planning a feature. The `brainstorming` skill should auto-activate.

Full mapping of Superpowers skills onto this project's workflow: `docs/SUPERPOWERS_INTEGRATION.md`.

---

## 8. Installation

```bash
# 1. Clone
git clone git@github.com:<owner>/<repo>.git
cd <repo>

# 2. Install deps
<pnpm install / uv sync / cargo build / etc.>

# 3. Set up env
cp .env.example .env.local
# ...edit .env.local with real values

# 4. Initialize MemPalace
pipx install mempalace
mempalace init .

# 5. Run
<pnpm dev / etc.>
```

---

## 9. NPM scripts (or equivalent)

<Fill from your project. Common shape:>

| Script | What it does |
|---|---|
| `<dev>` | Start dev server with hot reload |
| `<build>` | Production build with type-check |
| `<test>` | Run unit tests |
| `<test:e2e>` | Run end-to-end tests |
| `<lint>` | Run linter |

---

## 10. Branching & commit conventions

- `main` -> production.
- `develop` -> staging (if you have a staging environment).
- Feature branches: `feat/<short-name>` off `develop` (or `main` if no staging).
- Fix branches: `fix/<short-name>`.
- **Conventional Commits** required: `feat:`, `fix:`, `refactor:`, `docs:`, `chore:`, `test:`, `style:`.
- **Never** commit `.env*` files (other than `.env.example`).
- **Never** commit real API keys.

---

## 11. Questions / blockers

If a service account, API key, or credential is blocking work, log it in MemPalace under `ops` room (or a comparable room) and notify via Slack or your preferred channel. Do not fabricate values or hardcode placeholders that will leak into commits.
