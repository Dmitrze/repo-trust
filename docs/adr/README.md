# Architecture Decision Records

This directory contains the Architecture Decision Records (ADRs) for Repo Trust.

ADRs document the rationale for important architectural choices: what we decided, why, what we considered, and what the consequences are. They exist so that future contributors (and future versions of ourselves) understand the reasoning behind the code, not just the code itself.

## Format

We use a lightweight variant of [Michael Nygard's ADR template](https://github.com/joelparkerhenderson/architecture-decision-record/blob/main/locales/en/templates/decision-record-template-by-michael-nygard/index.md):

```
# NNNN — Title

## Status
Proposed | Accepted | Superseded by ADR-XXXX | Deprecated

## Context
What is the problem we are solving? What are the constraints?

## Decision
What did we decide?

## Consequences
What becomes easier? What becomes harder? What did we explicitly trade off?

## Alternatives considered
What else did we look at and why did we reject it?
```

## Index

| # | Title | Status |
| --- | --- | --- |
| [0001](0001-language-rust.md) | Language: Rust | Accepted |
| [0002](0002-clap-cli.md) | CLI framework: clap v4 | Accepted |
| [0003](0003-sqlite-cache.md) | Local cache: SQLite via rusqlite | Accepted |
| [0004](0004-no-ml-in-v1.md) | No machine learning in v1 | Accepted |
| [0005](0005-federate-not-replicate.md) | Federate upstream tools, do not replicate | Accepted |
| [0006](0006-five-modules.md) | Five trust modules | Accepted |
| [0007](0007-deterministic-output.md) | Deterministic JSON output | Accepted |
| [0008](0008-confidence-separate-from-score.md) | Confidence is independent of score | Accepted |
| [0009](0009-license-apache-2.md) | License: Apache-2.0 with CC-BY-4.0 for methodology | Accepted |
| [0010](0010-plugin-system-deferred.md) | Plugin system deferred to v1.2 | Accepted |
| [0011](0011-module-trait-shipped-shape.md) | TrustModule trait: shipped object-safe `run()` shape | Accepted |
| [0012](0012-repository-context-runtime-handles.md) | RepositoryContext carries runtime handles for v1 | Accepted |

## When to add a new ADR

Add one when:
- The decision will be hard to reverse later.
- The decision affects more than one module.
- The decision is non-obvious or counterintuitive.
- A decision was previously made and is now being changed (supersede).

Don't add one for:
- Implementation details that don't affect contracts.
- Routine library upgrades.
- Style choices already covered by `rustfmt.toml`.
