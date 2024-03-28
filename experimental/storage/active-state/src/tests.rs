// Copyright © Aptos Foundation

// SMT + JMT vs SMT + 64M size active state tree
// Compare the updates per second and memory usage
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
use tempfile::tempdir;

pub struct TestConfig {
    pub state_merkledb: Arc<StateMerkleDb>,
    pub current_smt: SparseMerkleTree<StateValue>,
    pub last_smt: SparseMerkleTree<StateValue>,
    pub updates_since_base: ShardedStateUpdates,
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
    let seed: &[_] = &[1, 2, 3, 4];
    let mut actual_seed = [0u8; 32];
    actual_seed[..seed.len()].copy_from_slice(seed);

    let mut rng = StdRng::from_seed(actual_seed);
    let base_smt = create_empty_smt();
    println!("base_smt generation: {:?}", base_smt.generation());
    // update the base smt with new kvs
    let new_kvs: Vec<(StateKey, Option<StateValue>)> =
        std::iter::repeat_with(|| (gen_key(HashValue::random()), gen_value(&mut rng)))
            .take(1000000)
            .collect();
    let proof_reader = ProofReader::new(Vec::new()); // insert into empty SMT
    let current_smt = base_smt
        .batch_update(
            new_kvs
                .iter()
                .map(|(k, v)| (CryptoHash::hash(k), v.as_ref()))
                .collect(),
            &proof_reader,
        )
        .unwrap();

    //populate the updates_since_base
    let mut updates_since_base = ShardedStateUpdates::default();
    new_kvs.into_iter().for_each(|(k, v)| {
        updates_since_base[k.get_shard_id() as usize].insert(k, v);
    });

    TestConfig {
        state_merkledb,
        current_smt,
        last_smt: base_smt,
        updates_since_base,
    }
}

#[test]
fn test_smt_with_jmt() {
    let db_dir = tempdir().unwrap();
    let test_config = initialize_test(db_dir.path());
    let base_version = Some(0u64);
    let version = 2u64; // an aribitrary version for the test
                        // write the new nodes to the JMT

    let (shard_root_nodes, batches_for_shards): (Vec<Node<StateKey>>, Vec<SchemaBatch>) = {
        THREAD_MANAGER.get_non_exe_cpu_pool().install(|| {
            (0..16u8)
                .into_par_iter()
                .map(|shard_id| {
                    let node_hashes = test_config
                        .current_smt
                        .new_node_hashes_since(&test_config.last_smt, shard_id);
                    test_config.state_merkledb.merklize_value_set_for_shard(
                        shard_id,
                        jmt_update_refs(&jmt_updates(
                            &test_config.updates_since_base[shard_id as usize]
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
fn test_usize_overflow() {
    let a = usize::MAX;
    let b: usize = a - 1;
    println!("{a}, {b}")
}
