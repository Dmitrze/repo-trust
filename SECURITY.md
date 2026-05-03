# Security Policy

## Reporting a vulnerability

**Please do not report security vulnerabilities through public GitHub issues, discussions, or pull requests.**

Instead, report them privately via GitHub's [Security Advisories](https://github.com/Dmitrze/repo-trust/security/advisories/new) feature. This sends the report directly to maintainers and creates a private space for triage.

If you cannot use Security Advisories, you can also contact the maintainer through their [GitHub profile](https://github.com/Dmitrze) — request an encrypted channel before sharing details.

When reporting, please include:

- A description of the vulnerability
- Steps to reproduce, or a proof-of-concept
- The version affected (`repo-trust --version` output)
- Any potential impact you've identified
- Whether you've already disclosed this elsewhere

We aim to acknowledge reports within **5 business days** and to provide a remediation timeline within **15 business days** of acknowledgment.

## Supported versions

Until `v1.0.0`, only the `main` branch is supported. Once `v1.0.0` ships, the support window will be:

| Version | Supported |
| --- | --- |
| Latest minor of latest major | ✅ |
| Previous major (security fixes only) | ✅ for 12 months after EOL announcement |
| Older | ❌ |

## Disclosure policy

We follow **coordinated disclosure**:

1. Reporter privately discloses to maintainers.
2. Maintainers confirm and develop a fix.
3. We agree on a public disclosure date with the reporter, typically 90 days from the initial report.
4. We publish a GitHub Security Advisory with credit to the reporter (unless they request anonymity).
5. Patched releases are announced on the [GitHub Releases page](https://github.com/Dmitrze/repo-trust/releases).

If a vulnerability is being actively exploited in the wild, we may shorten the disclosure window.

## Scope

We accept reports about:

- The Repo Trust CLI itself (any crate in this repository).
- Any official Repo Trust container image (`ghcr.io/dmitrze/repo-trust`).
- Documentation that, if followed, would lead to a security issue.

We do **not** accept reports about:

- Vulnerabilities in third-party APIs we consume (GitHub, deps.dev, OSV) — report those upstream.
- Issues in repositories we *scan* — those are the scanned project's concern, not ours.
- Theoretical issues without a working PoC against a current release.

## Recognition

Reporters who follow this policy and disclose responsibly are credited in the published Security Advisory. We don't currently offer monetary bounties, but we will list significant contributors in our `BACKERS.md` and credit them in release notes.
