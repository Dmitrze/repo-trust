# Project Governance

How decisions are made, who makes them, and how the project's leadership evolves.

---

## Current model (May 2026 — v0.x)

Repo Trust is a **single-maintainer project**. The current sole maintainer is [@Dmitrze](https://github.com/Dmitrze).

This is the kind of bus-factor risk we measure in our own Maintainer Health module. We are explicit about it. The maintainer's first priority for the v1.0 release cycle is recruiting **at least one** co-maintainer.

## Decision-making

| Decision type | Who decides |
| --- | --- |
| Bug fixes, small features, doc improvements | Any maintainer (currently 1) on review |
| New CLI flag, new sub-signal | Maintainer + community input via discussion (≥ 7 days) |
| New scoring weight or threshold change | Requires ADR + benchmark report + community input ≥ 14 days |
| Adding / removing a module | Requires ADR + scoring-version major bump + 30-day RFC |
| License or governance change | Requires unanimous active-maintainer agreement |

## Path to multi-maintainer governance

When we have 2+ active maintainers, governance moves to a **lazy consensus** model:
- Pull requests need 1 maintainer approval to merge.
- Sensitive changes (scoring, governance) need 2 maintainer approvals.
- Disagreement is resolved by discussion; if discussion stalls > 14 days, the PR is closed pending an ADR.

When we have 5+ active maintainers, we'll formalize a steering committee with charter and term limits. We are not there.

## Becoming a maintainer

To be considered for maintainer status:
1. Sustained, high-quality contributions over ≥ 3 months.
2. Demonstrated understanding of the methodology principles in `docs/methodology.md`.
3. Track record of constructive code review on others' PRs.
4. Nominated by an existing maintainer; lazy consensus from existing maintainers (no objections within 14 days).

We do not require any specific number of PRs — quality and judgment matter more than quantity. Methodology contributions count equally with code contributions.

## Maintainer responsibilities

A maintainer is expected to:
- Triage incoming issues within their declared focus area.
- Review PRs in a reasonable timeframe (target: 7 days for small, 14 days for substantive).
- Help shape the methodology rather than just stamp scores.
- Defer to community input on sensitive decisions; the project belongs to its users, not its maintainers.

A maintainer is **not** expected to:
- Be online any specific number of hours per week.
- Avoid taking breaks. Going inactive for a quarter is fine; just say so.
- Defer to outside organizations (employers, funders) on technical decisions about the tool itself.

## Sponsor relationships

GitHub Sponsors and similar funding does **not** confer governance authority. Sponsors get:
- Visibility on `BACKERS.md` (if they consent).
- Priority issue triage (response, not preferential acceptance).

Sponsors do **not** get:
- Influence over scoring weights or methodology.
- Removal of scores about their organization.
- Closed-source enterprise features.

If a sponsor opens a methodology question, it goes through the same triage as any other community input.

## Project conduct

All interactions are governed by [`CODE_OF_CONDUCT.md`](../CODE_OF_CONDUCT.md). Enforcement is by the maintainers, with reporting via [GitHub Security Advisories](https://github.com/Dmitrze/repo-trust/security/advisories/new) (the same private channel used for security reports).

## Project succession

If the current maintainer becomes unavailable for > 6 months without notice, the most-active contributor at that time is invited to assume maintenance. If no co-maintainer exists by that time, the project enters a public 30-day notice period seeking a new maintainer; if none emerges, the project is archived with a clear notice in the README.

This is intentionally documented to reduce uncertainty if it ever happens.
