# Support

## How to get help

The fastest way to get an answer depends on the kind of question you have.

| If you... | Then... |
| --- | --- |
| Found a bug or unexpected behavior | Open a [bug report issue](https://github.com/Dmitrze/repo-trust/issues/new?template=bug_report.yml) |
| Want a new feature or have a feature idea | Open a [feature request issue](https://github.com/Dmitrze/repo-trust/issues/new?template=feature_request.yml) |
| Disagree with how a repo was scored | Open a [methodology question issue](https://github.com/Dmitrze/repo-trust/issues/new?template=methodology_question.yml) |
| Want to discuss design, methodology, or "how would you handle X" | Start a thread in [Discussions](https://github.com/Dmitrze/repo-trust/discussions) |
| Found a security vulnerability | See [SECURITY.md](SECURITY.md) — do **not** open a public issue |
| Want to contribute code or docs | See [CONTRIBUTING.md](CONTRIBUTING.md) |
| Want to sponsor maintenance | [GitHub Sponsors](https://github.com/sponsors/Dmitrze) |

## What to include in a bug report

A useful bug report has, at minimum:

- The output of `repo-trust --version`
- Your operating system and Rust version (`rustc --version`)
- The exact command you ran
- The full output (or a redacted version if it contains secrets)
- What you expected to happen
- What actually happened

Even better: a JSON report attached as a file (`repo-trust scan <repo> --json > report.json`) so we can see exactly what the tool decided.

## Response expectations

This project is maintained by volunteers. Reasonable response targets:

- **Security reports:** acknowledged within 5 business days (see SECURITY.md).
- **Bug reports:** triaged within 7 days.
- **Feature requests:** triaged within 14 days. We will not commit to building every requested feature.
- **Methodology questions:** triaged within 7 days. These often turn into improvements to `docs/methodology.md`.
- **Pull requests:** initial review within 7 days for small PRs, longer for substantive changes.

If something is urgent and the project is the underlying cause (e.g. a CI integration is producing wrong results in production), say so in the issue title — we will prioritize.

## What we don't do

We do not provide:

- Private one-on-one consulting on how to interpret your repo's score.
- Custom scoring weights for specific organizations (use the `--weights` flag and `weights.toml`).
- Hosted scanning. The tool is fully local.
- Removal of public scores on demand. We don't publish a public scoreboard, so this rarely comes up — but if you find Repo Trust output in a third-party site you disagree with, that is between you and that site.

## Commercial support

There is no formal commercial support tier. If your team needs high-touch support, the best option is:

1. Sponsor the project at the highest tier on [GitHub Sponsors](https://github.com/sponsors/Dmitrze) (gets you priority issue triage and direct line via your sponsor profile).
2. Open a [Discussion](https://github.com/Dmitrze/repo-trust/discussions) describing what you need — for some asks, the answer is "we'll add it to the public roadmap".
