// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    get_first_seq_num_and_limit,
    pruner::{
        ledger_pruner_manager::LedgerPrunerManager, state_pruner_manager::StatePrunerManager,
    },
    test_helper,
    test_helper::{arb_blocks_to_commit, put_as_state_root, put_transaction_info},
    AptosDB, PrunerManager, StaleNodeIndexSchema,
};
use aptos_config::config::{
    EpochSnapshotPrunerConfig, LedgerPrunerConfig, PrunerConfig, RocksdbConfigs,
    StateMerklePrunerConfig, DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD, TARGET_SNAPSHOT_SIZE,
};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_temppath::TempPath;
use aptos_types::ledger_info::LedgerInfoWithSignatures;
use aptos_types::state_store::state_storage_usage::StateStorageUsage;
use aptos_types::transaction::{TransactionToCommit, Version};
use aptos_types::{
    proof::SparseMerkleLeafNode,
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::{ExecutionStatus, TransactionInfo},
};
use proptest::prelude::*;
use std::collections::HashSet;
use std::sync::Arc;
use storage_interface::{DbReader, DbWriter, ExecutedTrees, Order};
use test_helper::{test_save_blocks_impl, test_sync_transactions_impl};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]

    #[test]
    fn test_save_blocks(input in arb_blocks_to_commit(), threshold in 10..20usize) {
        test_save_blocks_impl(input, threshold);
    }

    #[test]
    fn test_sync_transactions(input in arb_blocks_to_commit(), threshold in 10..20usize) {
        test_sync_transactions_impl(input, threshold);
    }
}

#[test]
fn test_get_first_seq_num_and_limit() {
    assert!(get_first_seq_num_and_limit(Order::Ascending, 0, 0).is_err());

    // ascending
    assert_eq!(
        get_first_seq_num_and_limit(Order::Ascending, 0, 4).unwrap(),
        (0, 4)
    );
    assert_eq!(
        get_first_seq_num_and_limit(Order::Ascending, 0, 1).unwrap(),
        (0, 1)
    );

    // descending
    assert_eq!(
        get_first_seq_num_and_limit(Order::Descending, 2, 1).unwrap(),
        (2, 1)
    );
    assert_eq!(
        get_first_seq_num_and_limit(Order::Descending, 2, 2).unwrap(),
        (1, 2)
    );
    assert_eq!(
        get_first_seq_num_and_limit(Order::Descending, 2, 3).unwrap(),
        (0, 3)
    );
    assert_eq!(
        get_first_seq_num_and_limit(Order::Descending, 2, 4).unwrap(),
        (0, 3)
    );
}

#[test]
fn test_too_many_requested() {
    let tmp_dir = TempPath::new();
    let db = AptosDB::new_for_test(&tmp_dir);

    assert!(db.get_transactions(0, 1001 /* limit */, 0, true).is_err());
    assert!(db.get_transaction_outputs(0, 1001 /* limit */, 0).is_err());
}

#[test]
fn test_pruner_config() {
    let tmp_dir = TempPath::new();
    let aptos_db = AptosDB::new_for_test(&tmp_dir);
    for enable in [false, true] {
        let state_pruner = StatePrunerManager::<StaleNodeIndexSchema>::new(
            Arc::clone(&aptos_db.state_merkle_db),
            StateMerklePrunerConfig {
                enable,
                prune_window: 20,
                batch_size: 1,
            },
        );
        assert_eq!(state_pruner.is_pruner_enabled(), enable);
        assert_eq!(state_pruner.get_prune_window(), 20);

        let ledger_pruner = LedgerPrunerManager::new(
            Arc::clone(&aptos_db.ledger_db),
            Arc::clone(&aptos_db.state_store),
            LedgerPrunerConfig {
                enable,
                prune_window: 100,
                batch_size: 1,
                user_pruning_window_offset: 0,
            },
        );
        assert_eq!(ledger_pruner.is_pruner_enabled(), enable);
        assert_eq!(ledger_pruner.get_prune_window(), 100);
    }
}

#[test]
fn test_error_if_version_pruned() {
    let tmp_dir = TempPath::new();
    let db = AptosDB::new_for_test(&tmp_dir);
    db.state_store
        .state_db
        .state_pruner
        .testonly_update_min_version(5);
    db.ledger_pruner.testonly_update_min_version(10);
    assert_eq!(
        db.error_if_state_merkle_pruned("State", 4)
            .unwrap_err()
            .to_string(),
        "Version 4 is not epoch ending."
    );
    assert!(db.error_if_state_merkle_pruned("State", 5).is_ok());
    assert_eq!(
        db.error_if_ledger_pruned("Transaction", 9)
            .unwrap_err()
            .to_string(),
        "Transaction at version 9 is pruned, min available version is 10."
    );
    assert!(db.error_if_ledger_pruned("Transaction", 10).is_ok());
}

#[test]
fn test_get_latest_executed_trees() {
    let tmp_dir = TempPath::new();
    let db = AptosDB::new_for_test(&tmp_dir);

    // entirely empty db
    let empty = db.get_latest_executed_trees().unwrap();
    assert!(empty.is_same_view(&ExecutedTrees::new_empty()));

    // bootstrapped db (any transaction info is in)
    let key = StateKey::Raw(String::from("test_key").into_bytes());
    let value = StateValue::from(String::from("test_val").into_bytes());
    let hash = SparseMerkleLeafNode::new(key.hash(), value.hash()).hash();
    put_as_state_root(&db, 0, key, value);
    let txn_info = TransactionInfo::new(
        HashValue::random(),
        HashValue::random(),
        HashValue::random(),
        Some(hash),
        0,
        ExecutionStatus::MiscellaneousError(None),
    );
    put_transaction_info(&db, 0, &txn_info);

    let bootstrapped = db.get_latest_executed_trees().unwrap();
    assert!(
        bootstrapped.is_same_view(&ExecutedTrees::new_at_state_checkpoint(
            txn_info.state_checkpoint_hash().unwrap(),
            StateStorageUsage::new_untracked(),
            vec![txn_info.hash()],
            1,
        ))
    );
}

pub fn test_state_merkle_pruning_impl(
    input: Vec<(Vec<TransactionToCommit>, LedgerInfoWithSignatures)>,
) {
    // set up DB with state prune window 5 and epoch ending state prune window 10
    let tmp_dir = TempPath::new();
    let db = AptosDB::open(
        &tmp_dir,
        false, /* is_read_only */
        PrunerConfig {
            ledger_pruner_config: LedgerPrunerConfig {
                enable: true,
                prune_window: 10,
                batch_size: 1,
                user_pruning_window_offset: 0,
            },
            state_merkle_pruner_config: StateMerklePrunerConfig {
                enable: true,
                prune_window: 5,
                batch_size: 1,
            },
            epoch_snapshot_pruner_config: EpochSnapshotPrunerConfig {
                enable: true,
                prune_window: 10,
                batch_size: 1,
            },
        },
        RocksdbConfigs::default(),
        false, /* enable_indexer */
        TARGET_SNAPSHOT_SIZE,
        DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
    )
    .unwrap();

    // augment DB in blocks
    let mut in_memory_state = db
        .state_store
        .buffered_state()
        .lock()
        .current_state()
        .clone();
    let _ancester = in_memory_state.current.clone();
    let mut next_ver: Version = 0;
    let mut snapshot_versions = vec![];
    for (txns_to_commit, ledger_info_with_sigs) in input.iter() {
        test_helper::update_in_memory_state(&mut in_memory_state, txns_to_commit.as_slice());
        db.save_transactions(
            txns_to_commit,
            next_ver,                /* first_version */
            next_ver.checked_sub(1), /* base_state_version */
            Some(ledger_info_with_sigs),
            true, /* sync_commit */
            in_memory_state.clone(),
        )
        .unwrap();

        next_ver += txns_to_commit.len() as u64;

        let last_version = next_ver - 1;
        let is_epoch_ending = ledger_info_with_sigs.ledger_info().ends_epoch();
        snapshot_versions.push((last_version, is_epoch_ending));

        let state_merkle_min_readable = last_version.saturating_sub(5);
        let epoch_snapshot_min_readable = last_version.saturating_sub(10);
        let snapshots: Vec<_> = snapshot_versions
            .iter()
            .filter(|(v, _is_epoch_ending)| *v >= state_merkle_min_readable)
            .map(|(v, _)| *v)
            .collect();
        let epoch_snapshots: Vec<_> = snapshot_versions
            .iter()
            .filter(|(v, is_epoch_ending)| *is_epoch_ending && *v >= epoch_snapshot_min_readable)
            .map(|(v, _)| *v)
            .collect();

        // Prune till the oldest snapshot readable.
        let pruner = &db.state_store.state_db.state_pruner;
        let epoch_snapshot_pruner = &db.state_store.state_db.epoch_snapshot_pruner;
        pruner
            .pruner_worker
            .set_target_db_version(*snapshots.first().unwrap());
        epoch_snapshot_pruner
            .pruner_worker
            .set_target_db_version(std::cmp::min(
                *snapshots.first().unwrap(),
                *epoch_snapshots.first().unwrap_or(&Version::MAX),
            ));
        pruner.wait_for_pruner().unwrap();
        epoch_snapshot_pruner.wait_for_pruner().unwrap();

        // Check strictly that all trees in the window accessible and all those nodes not needed
        // must be gone.
        let non_pruned_versions: HashSet<_> = snapshots
            .into_iter()
            .chain(epoch_snapshots.into_iter())
            .collect();

        let expected_nodes: HashSet<_> = non_pruned_versions
            .iter()
            .flat_map(|v| db.state_store.get_all_jmt_nodes_referenced(*v).unwrap())
            .collect();
        let all_nodes: HashSet<_> = db
            .state_store
            .get_all_jmt_nodes()
            .unwrap()
            .into_iter()
            .collect();

        assert_eq!(expected_nodes, all_nodes);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]

    #[test]
    fn test_state_merkle_pruning(input in arb_blocks_to_commit()) {
        test_state_merkle_pruning_impl(input);
    }
}
