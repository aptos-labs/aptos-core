#[macro_use]
extern crate criterion;

use aptos_crypto::hash::{
    CryptoHash, CryptoHasher, PreSeededKeccak, SparseMerkleInternalHasher,
    SPARSE_MERKLE_INTERNAL_PRESEEDED,
};
use aptos_crypto::HashValue;
use criterion::{measurement::Measurement, BenchmarkGroup, BenchmarkId, Criterion};
use once_cell::sync::Lazy;
use rand::{thread_rng, Rng};
use std::hint::black_box;
use std::marker::PhantomData;

// Reproduce the same struct locally to avoid depending on aptos-types
struct MerkleTreeInternalNode<H> {
    left_child: HashValue,
    right_child: HashValue,
    _hasher: PhantomData<H>,
}

impl<H: CryptoHasher> CryptoHash for MerkleTreeInternalNode<H> {
    type Hasher = H;

    fn hash(&self) -> HashValue {
        let mut state = Self::Hasher::default();
        state.update(self.left_child.as_ref());
        state.update(self.right_child.as_ref());
        state.finish()
    }
}

type SparseMerkleInternalNode = MerkleTreeInternalNode<SparseMerkleInternalHasher>;

fn random_hash_value() -> HashValue {
    let mut rng = thread_rng();
    let mut bytes = [0u8; 32];
    rng.fill(&mut bytes);
    HashValue::new(bytes)
}

fn bench_merkle_internal_hash(c: &mut Criterion) {
    let mut group = c.benchmark_group("merkle_internal_hash");
    group.sample_size(5000);

    // --- Current implementation: clone hasher + 2x update + finish ---
    group.bench_function("current", |b| {
        b.iter_with_setup(
            || (random_hash_value(), random_hash_value()),
            |(left, right)| {
                let node = SparseMerkleInternalNode {
                    left_child: left,
                    right_child: right,
                    _hasher: PhantomData,
                };
                black_box(node.hash())
            },
        )
    });

    // --- Optimized: PreSeededKeccak::hash_pair ---
    group.bench_function("preseeded_keccak", |b| {
        // Force init
        Lazy::force(&SPARSE_MERKLE_INTERNAL_PRESEEDED);

        b.iter_with_setup(
            || (random_hash_value(), random_hash_value()),
            |(left, right)| {
                black_box(
                    SPARSE_MERKLE_INTERNAL_PRESEEDED
                        .hash_pair(left.as_ref(), right.as_ref()),
                )
            },
        )
    });

    group.finish();
}

/// Benchmark simulating proof verification: chain of N internal node hashes
fn bench_proof_chain(c: &mut Criterion) {
    let mut group = c.benchmark_group("proof_chain");
    group.sample_size(1000);

    for depth in [8, 16, 32, 64] {
        // Pre-generate sibling hashes
        let siblings: Vec<HashValue> = (0..depth).map(|_| random_hash_value()).collect();

        group.bench_function(BenchmarkId::new("current", depth), |b| {
            b.iter_with_setup(
                || random_hash_value(),
                |leaf_hash| {
                    let mut current = leaf_hash;
                    for sibling in &siblings {
                        let node = SparseMerkleInternalNode {
                            left_child: current,
                            right_child: *sibling,
                            _hasher: PhantomData,
                        };
                        current = node.hash();
                    }
                    black_box(current)
                },
            )
        });

        group.bench_function(BenchmarkId::new("preseeded_keccak", depth), |b| {
            Lazy::force(&SPARSE_MERKLE_INTERNAL_PRESEEDED);

            b.iter_with_setup(
                || random_hash_value(),
                |leaf_hash| {
                    let mut current = leaf_hash;
                    for sibling in &siblings {
                        current = SPARSE_MERKLE_INTERNAL_PRESEEDED
                            .hash_pair(current.as_ref(), sibling.as_ref());
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
