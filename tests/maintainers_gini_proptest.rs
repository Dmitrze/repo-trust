#![allow(
    clippy::unused_async,
    clippy::float_cmp,
    clippy::doc_lazy_continuation,
    clippy::unreadable_literal,
    clippy::too_many_lines
)]

//! Property-based tests for `features::maintainers::gini` and `bus_factor`.
//!
//! Invariants we want regardless of input distribution:
//! 1. Gini is bounded `[0.0, 1.0]`.
//! 2. Equal contributions ⇒ Gini ≈ 0.
//! 3. Single-author distribution ⇒ Gini = 0 (degenerate base case).
//! 4. Bus factor never exceeds the number of contributors.
//! 5. Bus factor of an all-equal distribution ≈ ⌈n/2⌉ when total > 0.

use proptest::prelude::*;
use repo_trust::features::maintainers::{bus_factor, gini};

proptest! {
    #[test]
    fn gini_is_bounded(values in proptest::collection::vec(0u64..10_000, 0..50)) {
        let g = gini(&values);
        prop_assert!((0.0..=1.0).contains(&g), "gini out of bounds: {g}");
    }

    #[test]
    fn gini_is_zero_for_equal_distribution(n in 1usize..30, val in 1u64..1000) {
        let values: Vec<u64> = vec![val; n];
        let g = gini(&values);
        prop_assert!(g.abs() < 1e-9, "expected ~0, got {g}");
    }

    #[test]
    fn gini_is_zero_for_single_author(val in 0u64..10_000) {
        prop_assert_eq!(gini(&[val]), 0.0);
    }

    #[test]
    fn bus_factor_bounded_by_contributor_count(values in proptest::collection::vec(0u64..1000, 1..20)) {
        let bf = bus_factor(&values);
        prop_assert!(bf <= values.len() as u64);
    }

    #[test]
    fn bus_factor_zero_total_returns_zero(n in 1usize..20) {
        let values: Vec<u64> = vec![0; n];
        prop_assert_eq!(bus_factor(&values), 0);
    }

    #[test]
    fn bus_factor_top_heavy_is_one(top in 1u64..1000, n in 0usize..10) {
        // top + n authors with 0 contributions → top alone covers 100% ≥ 50%.
        let mut v = vec![top];
        v.extend(vec![0u64; n]);
        prop_assert_eq!(bus_factor(&v), 1);
    }
}
