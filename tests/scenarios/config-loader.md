---
feature: config-loader
status: accepted
spec: ../../specs/config-loader.md
dri: "@Dmitrze"
created: 2026-05-03
updated: 2026-05-03
---

# Config loader — Scenarios

Link: [`specs/config-loader.md`](../../specs/config-loader.md)

---

## Coverage at a glance

| Category | Count | Notes |
|---|---|---|
| Happy path | 1 | defaults applied |
| Edge cases | 4 | precedence (user, project, env, CLI) |
| Failure modes | 2 | malformed TOML; conflicting types |

---

## Happy path

### S-001: defaults loaded when no config files / env / flags present

**Given** no `~/.repo-trust/config.toml`, no `./.repo-trust.toml`, no `REPO_TRUST_*` env vars, no CLI overrides
**When** `Config::load(empty_overrides)` is called
**Then** the result is `Ok(Config { weights: ModuleWeights::default(), scan: ScanConfig { default_mode: Standard, ... }, ... })`.

---

## Edge cases (precedence chain)

### S-101: user config file overrides defaults

**Given** `~/.repo-trust/config.toml` contains `[weights] activity = 0.40`
**When** `Config::load` is called
**Then** `cfg.weights.activity == 0.40`; other fields fall back to defaults.

### S-102: project file overrides user file

**Given** user config sets `activity = 0.40`; cwd `./.repo-trust.toml` sets `activity = 0.50`
**When** `Config::load` is called from cwd
**Then** `cfg.weights.activity == 0.50`.

### S-103: env var overrides files

**Given** project config sets `activity = 0.50`; env `REPO_TRUST_WEIGHTS__ACTIVITY=0.60` is set
**When** `Config::load` is called
**Then** `cfg.weights.activity == 0.60`.

### S-104: CLI flag overrides env

**Given** env sets `activity = 0.60`; CLI overrides include `weights.activity = 0.70`
**When** `Config::load(overrides)` is called
**Then** `cfg.weights.activity == 0.70`.

---

## Failure modes

### S-201: malformed TOML returns actionable error

**Given** `./.repo-trust.toml` contains `[weights\nactivity = nope`
**When** `Config::load` is called
**Then** the result is `Err` whose display string mentions the file path and includes a `figment`-derived parse error pointing at the offending line.

### S-202: type mismatch in user config

**Given** `~/.repo-trust/config.toml` contains `[weights] activity = "high"`
**When** `Config::load` is called
**Then** the result is `Err` with a clear "expected float, got string" message.

---

## How an agent reads this file

Same as `tests/scenarios/_TEMPLATE.md` §How an agent reads this file.
