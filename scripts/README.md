# Helper scripts

This directory holds developer-facing helper scripts. None of these are
shipped with the binary or referenced from `Cargo.toml`.

Planned scripts:

- `run_benchmark.rs` — runs the curated benchmark set in
  `examples/benchmark-set.csv` and produces `docs/benchmarks/<version>.md`.
- `update_fixtures.rs` — regenerates `tests/fixtures/<repo>/` from live
  upstream APIs against a known repo. Used when an upstream API genuinely
  changes its response shape.
- `verify_release.rs` — sanity-checks a tagged release locally before
  publishing.

Add a script via PR with a clear top-of-file docstring explaining its
purpose and usage.
