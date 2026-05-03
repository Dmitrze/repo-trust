//! Criterion benchmarks for the scoring layer.
//!
//! Run with: `cargo bench`
//!
//! These benchmarks are intentionally minimal until Phase 1 modules land
//! and we have realistic feature inputs to pass through the aggregator.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use repo_trust::models::{Confidence, ModuleResult, ModuleWeights};
use repo_trust::scoring::aggregate;
use std::collections::BTreeMap;

fn five_perfect_modules() -> Vec<ModuleResult> {
    [
        "stars",
        "activity",
        "maintainers",
        "adoption",
        "security",
    ]
    .iter()
    .map(|name| ModuleResult {
        module: (*name).to_string(),
        score: 100,
        confidence: Confidence::High,
        sub_scores: BTreeMap::new(),
        sample_size: None,
        missing_data: vec![],
    })
    .collect()
}

fn bench_aggregate(c: &mut Criterion) {
    let modules = five_perfect_modules();
    let weights = ModuleWeights::default();
    c.bench_function("aggregate_5_perfect_modules", |b| {
        b.iter(|| aggregate(black_box(&modules), black_box(&weights)));
    });
}

criterion_group!(benches, bench_aggregate);
criterion_main!(benches);
