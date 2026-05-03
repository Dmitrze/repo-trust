## Summary

<!-- One or two sentences: what does this PR do, and why? -->

## Type of change

<!-- Check all that apply -->

- [ ] Bug fix (non-breaking change that fixes an issue)
- [ ] New feature (non-breaking change that adds functionality)
- [ ] Breaking change (fix or feature that changes existing behavior)
- [ ] Scoring change (changes module weights, thresholds, or scoring logic)
- [ ] Documentation update
- [ ] Refactor (no behavior change)
- [ ] CI / build / tooling

## Linked issue(s)

<!-- e.g. Closes #123, Fixes #456 -->

## How was this verified?

<!-- Concrete steps a reviewer can run to confirm. "Ran cargo test" is not enough. -->

```bash
# example
cargo test --all-features
repo-trust scan octocat/Hello-World --mode quick
```

## Screenshots / output (if relevant)

<!-- Terminal output, JSON snippets, before/after screenshots -->

## Risks

<!-- What could break? Who is affected? Is there a rollback path? -->

## Checklist

- [ ] Branch is rebased onto current `main`.
- [ ] `cargo fmt --all -- --check` passes.
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes.
- [ ] `cargo test --all-features` passes.
- [ ] Added tests for new behavior, regression test for bug fixes.
- [ ] Public items have rustdoc comments.
- [ ] Updated `CHANGELOG.md` under `[Unreleased]`.
- [ ] If scoring changed: bumped scoring version in `docs/scoring-model.md`.
- [ ] If JSON schema changed: bumped `schema_version` and noted migration.
- [ ] No new runtime dependencies, or justified the addition above.
- [ ] No secrets, tokens, or PII in the diff.

## Notes for reviewer

<!-- Anything specific you want the reviewer to look at? Areas of uncertainty? -->
