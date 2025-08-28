// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    db::{
        aptosdb_internal::get_first_seq_num_and_limit,
        test_helper,
        test_helper::{arb_blocks_to_commit, put_transaction_auxiliary_data},
        AptosDB,
    },
    pruner::{LedgerPrunerManager, PrunerManager, StateMerklePrunerManager},
    schema::stale_node_index::StaleNodeIndexSchema,
};
use aptos_config::config::{
    EpochSnapshotPrunerConfig, LedgerPrunerConfig, PrunerConfig, RocksdbConfigs,
    StateMerklePrunerConfig, StorageDirPaths, BUFFERED_STATE_TARGET_ITEMS_FOR_TEST,
    DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_storage_interface::{DbReader, Order};
use aptos_temppath::TempPath;
use aptos_types::{
    ledger_info::LedgerInfoWithSignatures,
    proof::SparseMerkleLeafNode,
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::{
        ExecutionStatus, TransactionAuxiliaryData, TransactionAuxiliaryDataV1, TransactionInfo,
        TransactionToCommit, VMErrorDetail, Version,
    },
    vm_status::StatusCode,
    write_set::WriteSet,
};
use proptest::prelude::*;
use std::{collections::HashSet, sync::Arc};
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
        let state_merkle_pruner = StateMerklePrunerManager::<StaleNodeIndexSchema>::new(
            Arc::clone(&aptos_db.state_merkle_db()),
            StateMerklePrunerConfig {
                enable,
                prune_window: 20,
                batch_size: 1,
            },
        );
        assert_eq!(state_merkle_pruner.is_pruner_enabled(), enable);
        assert_eq!(state_merkle_pruner.get_prune_window(), 20);

        let ledger_pruner = LedgerPrunerManager::new(
            Arc::clone(&aptos_db.ledger_db),
            LedgerPrunerConfig {
                enable,
                prune_window: 100,
                batch_size: 1,
                user_pruning_window_offset: 0,
            },
            None,
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
        .state_merkle_pruner
        .save_min_readable_version(5)
        .unwrap();
    db.ledger_pruner.save_min_readable_version(10).unwrap();
    assert_eq!(
        db.error_if_state_merkle_pruned("State", 4)
            .unwrap_err()
            .to_string(),
        "AptosDB Other Error: Version 4 is not epoch ending."
    );
    assert!(db.error_if_state_merkle_pruned("State", 5).is_ok());
    assert_eq!(
        db.error_if_ledger_pruned("Transaction", 9)
            .unwrap_err()
            .to_string(),
        "AptosDB Other Error: Transaction at version 9 is pruned, min available version is 10."
    );
    assert!(db.error_if_ledger_pruned("Transaction", 10).is_ok());
}

#[test]
fn test_get_transaction_auxiliary_data() {
    let tmp_dir = TempPath::new();
    let db = AptosDB::new_for_test(&tmp_dir);
    let aux_1 = TransactionAuxiliaryData::V1(TransactionAuxiliaryDataV1 {
        detail_error_message: Some(VMErrorDetail::new(StatusCode::TYPE_MISMATCH, None)),
    });
    let aux_2 = TransactionAuxiliaryData::V1(TransactionAuxiliaryDataV1 {
        detail_error_message: Some(VMErrorDetail::new(
            StatusCode::ARITHMETIC_ERROR,
            Some("divided by 0".to_string()),
        )),
    });
    let txns = vec![aux_1.clone(), aux_2.clone()];
    put_transaction_auxiliary_data(&db, 0, &txns);
    assert_eq!(
        db.get_transaction_auxiliary_data_by_version(0).unwrap(),
        Some(aux_1)
    );
    assert_eq!(
        db.get_transaction_auxiliary_data_by_version(1).unwrap(),
        Some(aux_2)
    );
}

#[test]
fn test_get_latest_ledger_summary() {
    let tmp_dir = TempPath::new();
    let db = AptosDB::new_for_test(&tmp_dir);

    db.save_transactions_for_test(
        &[],
        0,    /* first_version */
        None, /* ledger_info_with_sigs */
        true, /* sync_commit */
    )
    .unwrap();

    // entirely empty db
    let empty = db.get_pre_committed_ledger_summary().unwrap();
    assert_eq!(empty.next_version(), 0);

    // bootstrapped db (any transaction info is in)
    let key = StateKey::raw(b"test_key");
    let value = StateValue::from(b"test_val".to_vec());
    let state_hash = SparseMerkleLeafNode::new(key.hash(), value.hash()).hash();
    let txn_info = TransactionInfo::new(
        HashValue::random(),
        HashValue::random(),
        HashValue::random(),
        Some(state_hash),
        0,
        ExecutionStatus::MiscellaneousError(None),
        Some(HashValue::random()),
    );
    let root_hash = txn_info.hash();
    let mut txn_to_commit = TransactionToCommit::dummy();
    txn_to_commit.transaction_info = txn_info;
    txn_to_commit.write_set = WriteSet::new_for_test([(key, Some(value))]);

    db.save_transactions_for_test(
        &[txn_to_commit],
        0,    /* first_version */
        None, /* ledger_info_with_sigs */
        true, /* sync_commit */
    )
    .unwrap();

    let bootstrapped = db.get_pre_committed_ledger_summary().unwrap();
    assert_eq!(bootstrapped.next_version(), 1);
    assert_eq!(bootstrapped.transaction_accumulator.root_hash(), root_hash,);
    assert_eq!(bootstrapped.state_summary.root_hash(), state_hash);
}

pub fn test_state_merkle_pruning_impl(
    input: Vec<(Vec<TransactionToCommit>, LedgerInfoWithSignatures)>,
) {
    // set up DB with state prune window 5 and epoch ending state prune window 10
    let tmp_dir = TempPath::new();
    let db = AptosDB::open(
        StorageDirPaths::from_path(tmp_dir),
        /*readonly=*/ false,
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
        BUFFERED_STATE_TARGET_ITEMS_FOR_TEST,
        DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
        None,
    )
    .unwrap();

    // augment DB in blocks
    let mut next_ver: Version = 0;
    let mut snapshot_versions = vec![];
    for (txns_to_commit, ledger_info_with_sigs) in input.iter() {
        db.save_transactions_for_test(
            txns_to_commit,
            next_ver, /* first_version */
            Some(ledger_info_with_sigs),
            true, /* sync_commit */
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
        let pruner = &db.state_store.state_db.state_merkle_pruner;
        let epoch_snapshot_pruner = &db.state_store.state_db.epoch_snapshot_pruner;
        pruner.set_worker_target_version(*snapshots.first().unwrap());
        epoch_snapshot_pruner.set_worker_target_version(std::cmp::min(
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
    #[ignore]
    // TODO(grao): Fix this.
    fn test_state_merkle_pruning(input in arb_blocks_to_commit()) {
        aptos_logger::Logger::new().init();
        test_state_merkle_pruning_impl(input);
    }
}
