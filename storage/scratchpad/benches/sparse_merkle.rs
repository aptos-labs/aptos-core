// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::{hash::SPARSE_MERKLE_PLACEHOLDER_HASH, HashValue};
use aptos_scratchpad::{
    test_utils::{naive_smt::NaiveSmt, proof_reader::ProofReader},
    SparseMerkleTree,
};
use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, Throughput};
use itertools::zip_eq;
use rand::{distributions::Standard, prelude::StdRng, seq::IteratorRandom, Rng, SeedableRng};
use std::collections::HashSet;

#[cfg(unix)]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

struct Block {
    smt: SparseMerkleTree,
    updates: Vec<(HashValue, Option<HashValue>)>,
    proof_reader: ProofReader,
}

impl Block {
    fn updates(&self) -> Vec<(HashValue, Option<HashValue>)> {
        self.updates.clone()
    }
}

struct Group {
    name: String,
    blocks: Vec<Block>,
}

impl Group {
    fn run(&self, c: &mut Criterion) {
        let mut group = c.benchmark_group(&self.name);

        for block in &self.blocks {
            let block_size = block.updates.len();
            let one_large_batch = block.updates();

            group.throughput(Throughput::Elements(block_size as u64));

            group.bench_function(BenchmarkId::new("batch_update", block_size), |b| {
                b.iter_batched(
                    || one_large_batch.clone(),
                    // return the resulting smt so the cost of Dropping it is not counted
                    |one_large_batch| -> SparseMerkleTree {
                        block
                            .smt
                            .freeze_self_and_update(one_large_batch, &block.proof_reader)
                            .unwrap()
                    },
                    BatchSize::LargeInput,
                )
            });
        }
        group.finish();
    }
}

struct Benches {
    base_empty: Group,
    base_committed: Group,
    base_uncommitted: Group,
}

impl Benches {
    fn new(block_sizes: &[usize]) -> Self {
        let mut rng = Self::rng();

        // 1 million possible keys
        let keys = std::iter::repeat_with(|| HashValue::random_with_rng(&mut rng))
            .take(1_000_000)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();

        // group: insert to an empty SMT
        let base_empty = Group {
            name: "insert to empty".into(),
            blocks: block_sizes
                .iter()
                .map(|block_size| Block {
                    smt: SparseMerkleTree::new(*SPARSE_MERKLE_PLACEHOLDER_HASH),
                    updates: Self::gen_updates(&mut rng, &keys, *block_size),
                    proof_reader: ProofReader::new(Vec::new()),
                })
                .collect(),
        };

        // all addresses with an existing value
        let values = std::iter::repeat_with(|| Self::gen_value(&mut rng))
            .take(keys.len())
            .collect::<Vec<_>>();
        let existing_state = zip_eq(&keys, &values)
            .filter_map(|(key, value)| value.as_ref().map(|v| (*key, v)))
            .collect::<Vec<_>>();
        let mut naive_base_smt = NaiveSmt::new(&existing_state);

        // group: insert to a committed SMT ("unknown" root)
        let base_committed = Group {
            name: "insert to committed base".into(),
            blocks: block_sizes
                .iter()
                .map(|block_size| {
                    let updates = Self::gen_updates(&mut rng, &keys, *block_size);
                    let proof_reader = Self::gen_proof_reader(&mut naive_base_smt, &updates);
                    Block {
                        smt: SparseMerkleTree::new(naive_base_smt.get_root_hash()),
                        updates,
                        proof_reader,
                    }
                })
                .collect(),
        };

        // group: insert to an uncommitted SMT (some structures in mem)
        let base_uncommitted = Group {
            name: "insert to uncommitted base".into(),
            blocks: base_committed
                .blocks
                .iter()
                .map(|base_block| {
                    // This is an SMT holding updates from the `base_committed` block in mem.
                    let updates = Self::gen_updates(&mut rng, &keys, base_block.updates.len());
                    let proof_reader = Self::gen_proof_reader(&mut naive_base_smt, &updates);

                    Block {
                        smt: base_block
                            .smt
                            .freeze_self_and_update(base_block.updates(), &base_block.proof_reader)
                            .unwrap(),
                        updates,
                        proof_reader,
                    }
                })
                .collect(),
        };

        Self {
            base_empty,
            base_committed,
            base_uncommitted,
        }
    }

    fn run(&self, c: &mut Criterion) {
        self.base_empty.run(c);
        self.base_committed.run(c);
        self.base_uncommitted.run(c);
    }

    fn gen_updates(
        rng: &mut StdRng,
        keys: &[HashValue],
        block_size: usize,
    ) -> Vec<(HashValue, Option<HashValue>)> {
        std::iter::repeat_with(|| Self::gen_update(rng, keys))
            .take(block_size)
            .collect()
    }

    fn gen_update(rng: &mut StdRng, keys: &[HashValue]) -> (HashValue, Option<HashValue>) {
        (*keys.iter().choose(rng).unwrap(), Self::gen_value(rng))
    }

    fn gen_value(rng: &mut StdRng) -> Option<HashValue> {
        if rng.gen_ratio(1, 10) {
            None
        } else {
            let bytes: Vec<u8> = rng.sample_iter::<u8, _>(Standard).take(100).collect();
            Some(HashValue::new_legacy(bytes.into()))
        }
    }

    fn gen_proof_reader(
        naive_smt: &mut NaiveSmt,
        updates: &[(HashValue, Option<HashValue>)],
    ) -> ProofReader {
        let proofs = updates
            .iter()
            .map(|(key, _)| (*key, naive_smt.get_proof(key)))
            .collect();
        ProofReader::new(proofs)
    }

    fn rng() -> StdRng {
        let seed: &[_] = &[1, 2, 3, 4];
        let mut actual_seed = [0u8; 32];
        actual_seed[..seed.len()].copy_from_slice(&seed);

        StdRng::from_seed(actual_seed)
    }
}

fn sparse_merkle_benches(c: &mut Criterion) {
    // Fix Rayon threadpool size to 8, which is realistic as in the current production setting
    // and benchmarking result will be more stable across different machines.
    rayon::ThreadPoolBuilder::new()
        .num_threads(8)
        .thread_name(|index| format!("rayon-global-{}", index))
        .build_global()
        .expect("Failed to build rayon global thread pool.");

    Benches::new(&[2, 4, 8, 16, 32, 100, 1000, 10000]).run(c);
}

criterion_group!(benches, sparse_merkle_benches);
criterion_main!(benches);
