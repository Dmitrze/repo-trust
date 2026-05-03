# 0009 — License: Apache-2.0 with CC-BY-4.0 for methodology

## Status
Accepted (May 2026)

## Context

License choice affects:
- Adoption in commercial/enterprise CI pipelines.
- Patent risk for contributors and consumers.
- Compatibility with downstream linkage.
- Eligibility for foundations and funds (GitHub Secure OSS Fund, OpenSSF, Tidelift).
- The methodology documents' citability and adaptability in academic and industry work.

The project has two distinct concerns that reasonably take different licenses:
1. **The code** — should be permissively licensed for maximum adoption, with a patent grant for enterprise safety.
2. **The methodology documents** — should be citeable, attributable, and adaptable in derivative academic work.

## Decision

- **Code:** [Apache License, Version 2.0](https://www.apache.org/licenses/LICENSE-2.0).
- **Methodology documents** in `docs/`: Apache-2.0 (default for the repo) **plus** a dual grant under [Creative Commons Attribution 4.0 International (CC-BY-4.0)](https://creativecommons.org/licenses/by/4.0/), so they can be cited and adapted in academic/industry work without ambiguity about "is this code license also for prose?"

The LICENSE file at the repo root carries the Apache-2.0 text. The README and CONTRIBUTING explicitly note the CC-BY-4.0 dual grant for methodology docs.

## Consequences

### Easier
- Apache-2.0 is on every enterprise's allowed-license list.
- Patent grant protects contributors and downstream users from patent litigation.
- Methodology can be cited in research papers under standard CC norms.
- Compatible with most other OSS licenses for derivative integration.
- Eligibility for GitHub Secure OSS Fund and similar grants is preserved.

### Harder
- Apache-2.0 has explicit attribution requirements (NOTICE file) that need maintenance.
- Dual licensing of docs adds slight complexity for contributors who modify docs (their changes are also dual-licensed).

## Alternatives considered

### MIT only
**Why considered:** Simplest; widely adopted.

**Why rejected:** No explicit patent grant. For a tool likely to ship in CI, the patent grant in Apache-2.0 is meaningfully better protection.

### GPL-3.0
**Why considered:** Strongest copyleft; ensures derivatives stay open.

**Why rejected:** Some enterprise CI pipelines disallow GPL by default. We want this tool used everywhere; copyleft is a real adoption barrier.

### MPL-2.0
**Why considered:** File-level copyleft; reasonable middle ground.

**Why rejected:** Less familiar to enterprise legal teams than Apache-2.0; no clear advantage for our use case.

### Apache-2.0 only (no CC-BY for docs)
**Why considered:** Simpler.

**Why rejected:** Researchers and journalists are accustomed to CC norms for citing methodology; ambiguity here would discourage citation.
