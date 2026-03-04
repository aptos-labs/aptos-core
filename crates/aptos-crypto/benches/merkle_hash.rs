#[macro_use]
extern crate criterion;

use aptos_crypto::hash::{CryptoHasher, SparseMerkleInternalHasher};
use aptos_crypto::HashValue;
use criterion::{BenchmarkId, Criterion};
use rand::{thread_rng, Rng};
use std::hint::black_box;

fn random_hash_value() -> HashValue {
    let mut rng = thread_rng();
    let mut bytes = [0u8; 32];
    rng.fill(&mut bytes);
    HashValue::new(bytes)
}

/// Old path: clone pre-seeded hasher + 2x update + finish
#[inline(never)]
fn hash_old(left: &[u8; 32], right: &[u8; 32]) -> HashValue {
    let mut state = SparseMerkleInternalHasher::default();
    state.update(left);
    state.update(right);
    state.finish()
}

/// New path: CryptoHasher::hash_pair (routes through PreSeededKeccak)
#[inline(never)]
fn hash_new(left: &[u8; 32], right: &[u8; 32]) -> HashValue {
    SparseMerkleInternalHasher::hash_pair(left, right)
}

fn bench_merkle_internal_hash(c: &mut Criterion) {
    let mut group = c.benchmark_group("merkle_internal_hash");
    group.sample_size(5000);

    group.bench_function("old_clone_update", |b| {
        b.iter_with_setup(
            || (random_hash_value(), random_hash_value()),
            |(left, right)| black_box(hash_old(left.as_ref(), right.as_ref())),
        )
    });

    group.bench_function("new_preseeded", |b| {
        b.iter_with_setup(
            || (random_hash_value(), random_hash_value()),
            |(left, right)| black_box(hash_new(left.as_ref(), right.as_ref())),
        )
    });

    group.finish();
}

/// Benchmark simulating proof verification: chain of N internal node hashes
fn bench_proof_chain(c: &mut Criterion) {
    let mut group = c.benchmark_group("proof_chain");
    group.sample_size(1000);

    for depth in [8, 16, 32, 64] {
        let siblings: Vec<HashValue> = (0..depth).map(|_| random_hash_value()).collect();

        group.bench_function(BenchmarkId::new("old", depth), |b| {
            b.iter_with_setup(
                || random_hash_value(),
                |leaf_hash| {
                    let mut current = leaf_hash;
                    for sibling in &siblings {
                        current =
                            hash_old(current.as_ref(), sibling.as_ref());
                    }
                    black_box(current)
                },
            )
        });

        group.bench_function(BenchmarkId::new("new", depth), |b| {
            b.iter_with_setup(
                || random_hash_value(),
                |leaf_hash| {
                    let mut current = leaf_hash;
                    for sibling in &siblings {
                        current =
                            hash_new(current.as_ref(), sibling.as_ref());
                    }
                    black_box(current)
                },
            )
        });
    }

    group.finish();
}

criterion_group!(
    name = merkle_hash_benches;
    config = Criterion::default();
    targets = bench_merkle_internal_hash, bench_proof_chain
);
criterion_main!(merkle_hash_benches);
