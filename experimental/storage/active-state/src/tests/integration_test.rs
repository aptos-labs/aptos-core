// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Copyright © Aptos Foundation

// SMT + JMT vs SMT + 64M size active state tree
// Compare the updates per second and memory usage
use crate::{
    pipeline::{ExecutionMode, Pipeline, PipelineConfig},
    ActiveState, MAX_ITEMS,
};
use aptos_config::config::{RocksdbConfigs, StorageDirPaths};
use aptos_crypto::{
    hash::{CryptoHash, SPARSE_MERKLE_PLACEHOLDER_HASH},
    HashValue,
};
use aptos_db::state_merkle_db::StateMerkleDb;
use aptos_experimental_runtimes::thread_manager::THREAD_MANAGER;
use aptos_jellyfish_merkle::node_type::Node;
use aptos_schemadb::SchemaBatch;
use aptos_scratchpad::{test_utils::proof_reader::ProofReader, SparseMerkleTree};
use aptos_storage_interface::{jmt_update_refs, jmt_updates, Result};
use aptos_types::state_store::{
    state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
    ShardedStateUpdates,
};
use rand::{distributions::Standard, prelude::StdRng, Rng, SeedableRng};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::{path::Path, sync::Arc};
use tempfile::{tempdir, TempDir};

pub struct TestConfig {
    pub state_merkledb: Arc<StateMerkleDb>,
    pub current_smt: Option<SparseMerkleTree<StateValue>>,
    pub base_smt: SparseMerkleTree<StateValue>,
}

// create a SparseMerkleTree for with updates
fn create_empty_smt() -> SparseMerkleTree<StateValue> {
    SparseMerkleTree::new(
        *SPARSE_MERKLE_PLACEHOLDER_HASH,
        StateStorageUsage::new_untracked(),
    )
}

fn gen_key(value: HashValue) -> StateKey {
    StateKey::raw(value.to_vec())
}

fn gen_value(rng: &mut StdRng) -> Option<StateValue> {
    if rng.gen_ratio(1, 10) {
        None
    } else {
        let bytes: Vec<u8> = rng.sample_iter::<u8, _>(Standard).take(100).collect();
        Some(StateValue::new_legacy(bytes.into()))
    }
}

fn generate_test_data() -> Vec<(StateKey, Option<StateValue>)> {
    let seed: &[_] = &[1, 2, 3, 4];
    let mut actual_seed = [0u8; 32];
    actual_seed[..seed.len()].copy_from_slice(seed);

    let mut rng = StdRng::from_seed(actual_seed);
    std::iter::repeat_with(|| (gen_key(HashValue::random()), gen_value(&mut rng)))
        .take(1000000)
        .collect()
}

impl TestConfig {
    fn initialize_test(db_dir: &Path) -> TestConfig {
        println!("db_dir: {:?}", db_dir);
        let state_merkledb = Arc::new(
            StateMerkleDb::new(
                &StorageDirPaths::from_path(db_dir),
                RocksdbConfigs::default(),
                false,
                1000000usize,
            )
            .unwrap(),
        );

        let new_kvs = generate_test_data();
        let base_smt = create_empty_smt();

        TestConfig {
            state_merkledb,
            current_smt: None,
            base_smt,
        }
    }

    pub fn generate_jtm_updates(
        &mut self,
        new_kvs: Vec<(StateKey, Option<StateValue>)>,
    ) -> ShardedStateUpdates {
        let current_smt = self
            .base_smt
            .batch_update(
                new_kvs
                    .iter()
                    .map(|(k, v)| (CryptoHash::hash(k), v.as_ref()))
                    .collect(),
                &ProofReader::new(Vec::new()),
            )
            .unwrap();

        self.current_smt = Some(current_smt);
        //populate the updates_since_base
        let mut updates_since_base = ShardedStateUpdates::default();
        new_kvs.into_iter().for_each(|(k, v)| {
            updates_since_base[k.get_shard_id() as usize].insert(k, v);
        });
        updates_since_base
    }

    pub fn get_base_smt(&self) -> SparseMerkleTree<StateValue> {
        self.base_smt.clone()
    }
}

#[test]
fn test_smt_with_jmt() {
    let db_dir = tempdir().unwrap();
    let mut test_config = TestConfig::initialize_test(db_dir.path());
    let base_version = Some(0u64);
    let version = 2u64; // an aribitrary version for the test
                        // write the new nodes to the JMT
    let sharded_updates = test_config.generate_jtm_updates(generate_test_data());
    let (shard_root_nodes, batches_for_shards): (Vec<Node<StateKey>>, Vec<SchemaBatch>) = {
        THREAD_MANAGER.get_non_exe_cpu_pool().install(|| {
            (0..16u8)
                .into_par_iter()
                .map(|shard_id| {
                    let node_hashes = test_config
                        .current_smt
                        .as_ref()
                        .unwrap()
                        .new_node_hashes_since(&test_config.base_smt, shard_id);
                    test_config.state_merkledb.merklize_value_set_for_shard(
                        shard_id,
                        jmt_update_refs(&jmt_updates(
                            &sharded_updates[shard_id as usize]
                                .iter()
                                .map(|(k, v)| (k, v.as_ref()))
                                .collect(),
                        )),
                        Some(&node_hashes),
                        version,
                        base_version,
                        base_version,
                        base_version,
                    )
                })
                .collect::<Result<Vec<_>>>()
                .expect("Error calculating StateMerkleBatch for shards.")
                .into_iter()
                .unzip()
        })
    };

    // calculate the top levels batch
    let (_root_hash, top_levels_batch) = {
        test_config
            .state_merkledb
            .calculate_top_levels(shard_root_nodes, version, base_version, base_version)
            .expect("Error calculating StateMerkleBatch for top levels.")
    };

    test_config
        .state_merkledb
        .commit(version, top_levels_batch, batches_for_shards)
        .unwrap();
    println!("committed version: {}", version);
}

#[test]
fn test_smt_with_lru() {
    let db_dir = tempdir().unwrap();
    let test_config = TestConfig::initialize_test(db_dir.path());
    let mut ast = ActiveState::new(test_config.get_base_smt(), MAX_ITEMS);
    let value_set = generate_test_data();
    ast.batch_put_value_set(value_set).unwrap();
}

#[test]
fn test_pipeline_flow() {
    let path: String = TempDir::new().unwrap().path().to_str().unwrap().to_string();
    let config = PipelineConfig::new(1, 3, path, ExecutionMode::AST);
    let pipeline: Pipeline = Pipeline::new(config);
    pipeline.run();
}

#[test]
fn test_usize_overflow() {
    let a = usize::MAX;
    let b: usize = a - 1;
    println!("{a}, {b}")
}
