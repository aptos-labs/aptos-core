// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_crypto::arkworks::random::{
    unsafe_random_point, unsafe_random_point_slow, unsafe_random_points, unsafe_random_points_slow,
};
use ark_ec::{pairing::Pairing, CurveGroup};
use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use rand::{rngs::StdRng, SeedableRng};

const SAMPLE_SIZE: usize = 4_064; // 254 * 16

// ---------------------- Batch Benchmark ----------------------

fn bench_unsafe_random_points<C: CurveGroup>(c: &mut Criterion, label: &str) {
    let mut group = c.benchmark_group(label);

    // Slow version (scalar multiple)
    group.bench_function("unsafe_random_points_slow (batch)", |b| {
        b.iter_batched(
            || StdRng::seed_from_u64(0xDEADBEEF),
            |mut rng| unsafe_random_points_slow::<C, _>(SAMPLE_SIZE, &mut rng),
            BatchSize::PerIteration,
        )
    });

    // Hash-to-curve version
    group.bench_function("unsafe_random_points_hash (batch)", |b| {
        b.iter_batched(
            || StdRng::seed_from_u64(0xCAFEBABE),
            |mut rng| unsafe_random_points::<C, _>(SAMPLE_SIZE, &mut rng),
            BatchSize::PerIteration,
        )
    });
}

// ---------------------- Single-Point Benchmark ----------------------

fn bench_single_random_points<C: CurveGroup>(c: &mut Criterion, label: &str) {
    let mut group = c.benchmark_group(label);

    // Slow version (single point)
    group.bench_function("unsafe_random_point_slow (single)", |b| {
        b.iter_batched(
            || StdRng::seed_from_u64(0xBADF00D),
            |mut rng| unsafe_random_point_slow::<C, _>(&mut rng),
            BatchSize::PerIteration,
        )
    });

    // Hash-to-curve version (single point)
    group.bench_function("unsafe_random_point_hash (single)", |b| {
        b.iter_batched(
            || StdRng::seed_from_u64(0xB16B00B5),
            |mut rng: StdRng| unsafe_random_point::<C, _>(&mut rng),
            BatchSize::PerIteration,
        )
    });
}

fn criterion_benchmark(c: &mut Criterion) {
    type G1 = <ark_bn254::Bn254 as Pairing>::G1;

    bench_unsafe_random_points::<G1>(c, "Unsafe Random Point Benchmarks (batch)");
    bench_single_random_points::<G1>(c, "Unsafe Random Point Benchmarks (single)");
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
