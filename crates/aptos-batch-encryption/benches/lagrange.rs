use aptos_batch_encryption::shared::algebra::shamir::ThresholdConfig;
use ark_poly::EvaluationDomain;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::collections::HashSet;

pub fn all_lagrange(c: &mut Criterion) {
    let mut group = c.benchmark_group("ThresholdConfig::all_lagrange");

    for n in [256, 512, 1024] {
        let t = n * 2 / 3 + 1;

        let tc = ThresholdConfig::new(n, t);
        let xs = HashSet::from_iter(tc.domain.elements().take(t));

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("n={}, t={}", n, t)),
            &(tc, xs),
            |b, input| {
                b.iter(|| input.0.all_lagrange(&input.1));
            },
        );
    }
}

criterion_group!(benches, all_lagrange);
criterion_main!(benches);
