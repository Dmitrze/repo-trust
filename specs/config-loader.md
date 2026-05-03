---
feature: config-loader
status: accepted
dri: "@Dmitrze"
created: 2026-05-03
updated: 2026-05-03
related_agents: []
related_scenarios: ["tests/scenarios/config-loader.md"]
related_runbooks: []
related_docs: ["docs/architecture.md#11-configuration"]
---

# Config loader

> Layered configuration via `figment`: built-in defaults → user `~/.repo-trust/config.toml` → project `./.repo-trust.toml` → env (`REPO_TRUST_*`) → CLI flags. Powers all runtime tunables: weights, thresholds, sample sizes, output defaults, GitHub token resolution.

---

## 1. Goal

A single `Config::load()` call returns a fully-resolved `Config` struct that every subsequent module can read from. Layering precedence is documented and tested. CLI flags always win.

We know it worked when: a user can override `[stars] sample_size_standard = 500` in `./.repo-trust.toml`, run a scan, and the scan honors 500 — verifiable in the report's `caveats` or via debug logging.

---

## 2. Non-functional requirements

- **Startup overhead:** <10ms config resolution (excluding token reads from external secret managers, which are out of scope).
- **Errors are loud:** missing config file = silent (use defaults); malformed TOML = abort with actionable error pointing at the offending file + line.
- **No secrets in logs:** `token` field is `String`-typed but tagged with `serde(skip_serializing)` and partially redacted in `--debug` output.

---

## 3. Boundaries

### In scope (Phase 1)
- `Config` struct with sub-structs: `GithubConfig`, `ScanConfig`, `WeightsConfig`, `StarsConfig`, `OutputConfig`.
- Embedded `default.toml` via `include_str!("default.toml")`.
- `figment` providers chained in priority order (defaults → user → project → env → CLI overrides).
- Token resolution from env (`GITHUB_TOKEN` by default, configurable via `[github] token_env = "..."`).
- Loader called once in `cli::run()` and threaded into `RepositoryContext`.

### Out of scope (explicit)
- Config schema migration (v1.1+).
- Validation beyond TOML parse + serde derive (e.g. weight sum bounds; we already document non-strict in `ModuleWeights::total()`).
- Interactive `repo-trust config wizard`.
- Secret-manager integrations (1Password, Bitwarden) — token is env or file only.

---

## 4. Probabilistic satisfaction threshold

N/A.

---

## 5. Happy-path scenario

1. User installs repo-trust, runs `repo-trust scan octocat/Hello-World`.
2. `Config::load(cli_overrides)` is called.
3. No `~/.repo-trust/config.toml` exists, no `./.repo-trust.toml` exists, no `REPO_TRUST_*` env vars set, no overriding CLI flag.
4. Resolved config equals the embedded `default.toml` parsed values (weights from `ModuleWeights::default()`, `sample_size_standard = 200`, etc.).
5. `GITHUB_TOKEN` env present → `Config::github.token = Some("ghp_...")`.
6. Scan proceeds with the resolved config; `weights_used` in the report matches the resolved weights.

---

## 6. Architecture sketch

```
[ figment::Figment ]
   .merge(Toml::string(include_str!("default.toml")))   # defaults
   .merge(Toml::file(home_dir/".repo-trust/config.toml"))  # user
   .merge(Toml::file(cwd/".repo-trust.toml"))             # project
   .merge(Env::prefixed("REPO_TRUST_").split("__"))       # env
   .merge(Serialized::defaults(cli_overrides))            # CLI flags (highest)
   .extract::<Config>()?
```

Reference: `docs/architecture.md` §11.

---

## 7. Closed loop

- **Goal metric:** `tests/config_layering.rs::cli_flag_beats_env` passes; layering precedence captured in 5 unit tests.
- **Where it lives:** CI; MemPalace `decisions/tradeoffs` if any layering surprise emerges.
- **Read by:** Reviewer on PR; user-facing `--debug` log shows resolved config source per field.
- **Improvement path:** add `repo-trust config show --source` in v1.1 to surface where each value came from.

---

## 8. Definition of Done

- [ ] Spec status `accepted`.
- [ ] `tests/scenarios/config-loader.md` lists ≥5 scenarios (precedence cases + malformed file + missing token).
- [ ] `src/config/default.toml` exists with all defaults documented inline.
- [ ] `src/config/mod.rs` re-exports `Config`, `GithubConfig`, `ScanConfig`, `WeightsConfig`, `StarsConfig`, `OutputConfig`.
- [ ] `src/config/loader.rs::load(cli_overrides)` returns `anyhow::Result<Config>`.
- [ ] Unit tests cover default load, user-file override, project-file override, env override, CLI override, malformed-TOML error path.
- [ ] CHANGELOG entry.
- [ ] No new runtime crates (figment, dirs, toml already in Cargo.toml).
- [ ] All quality gates green.

---

## 9. Open questions

- None.

---

## 10. Closed questions (history)

- 2026-05-03 — embed `default.toml` in binary or read from disk? — Embed via `include_str!` per architecture §11; users can override but never have to install a config file.

---

## 11. References

- `docs/architecture.md` §11 — configuration.
