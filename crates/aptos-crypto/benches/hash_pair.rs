// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Benchmarks for the optimized `hash_two_children` path vs. the generic hasher
//! path for hashing two 32-byte hash values (the Merkle tree internal node pattern).

use aptos_crypto::{
    hash::{
        CryptoHasher, EventAccumulatorHasher, SparseMerkleInternalHasher,
        TransactionAccumulatorHasher,
    },
    HashValue,
};
use criterion::{
    criterion_group, criterion_main, measurement::Measurement, BenchmarkGroup, BenchmarkId,
    Criterion,
};
use rand::{rngs::StdRng, SeedableRng};

/// Hash two HashValues using the generic (old) hasher path.
fn hash_two_children_generic<H: CryptoHasher>(h1: &HashValue, h2: &HashValue) -> HashValue {
    let mut state = H::default();
    state.update(h1.as_ref());
    state.update(h2.as_ref());
    state.finish()
}

fn bench_hash_pair(c: &mut Criterion) {
    let mut group = c.benchmark_group("hash_pair");
    group.sample_size(5000);

    // Generate deterministic random inputs
    let mut rng = StdRng::seed_from_u64(0xDEAD_BEEF);
    let h1 = HashValue::random_with_rng(&mut rng);
    let h2 = HashValue::random_with_rng(&mut rng);

    bench_hasher::<SparseMerkleInternalHasher>(&mut group, "SparseMerkleInternal", &h1, &h2);
    bench_hasher::<TransactionAccumulatorHasher>(&mut group, "TransactionAccumulator", &h1, &h2);
    bench_hasher::<EventAccumulatorHasher>(&mut group, "EventAccumulator", &h1, &h2);

    group.finish();
}

fn bench_hasher<H: CryptoHasher>(
    g: &mut BenchmarkGroup<impl Measurement>,
    name: &str,
    h1: &HashValue,
    h2: &HashValue,
) {
    g.bench_function(BenchmarkId::new("optimized", name), |b| {
        b.iter(|| H::hash_two_children(h1, h2))
    });

    g.bench_function(BenchmarkId::new("generic", name), |b| {
        b.iter(|| hash_two_children_generic::<H>(h1, h2))
    });
}

criterion_group!(
    name = hash_pair_benches;
    config = Criterion::default();
    targets = bench_hash_pair
);
criterion_main!(hash_pair_benches);
