// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::{sync::Arc, time::Duration};

use aptos_config::config::{LedgerPrunerConfig, StateMerklePrunerConfig};
use proptest::prelude::*;

use crate::{
    get_first_seq_num_and_limit,
    pruner::{
        ledger_pruner_manager::LedgerPrunerManager, state_pruner_manager::StatePrunerManager,
    },
    test_helper,
    test_helper::{arb_blocks_to_commit, put_as_state_root, put_transaction_info},
    AptosDB, PrunerManager, ROCKSDB_PROPERTIES,
};

use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_state_view::state_storage_usage::StateStorageUsage;
use aptos_temppath::TempPath;
use aptos_types::{
    proof::SparseMerkleLeafNode,
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::{ExecutionStatus, TransactionInfo},
};
use storage_interface::{DbReader, ExecutedTrees, Order};
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
        let state_pruner = StatePrunerManager::new(
            Arc::clone(&aptos_db.state_merkle_db),
            StateMerklePrunerConfig {
                enable,
                prune_window: 20,
                batch_size: 1,
                user_pruning_window_offset: 0,
            },
        );
        assert_eq!(state_pruner.is_pruner_enabled(), enable);
        assert_eq!(state_pruner.get_pruner_window(), 20);

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
        assert_eq!(ledger_pruner.get_pruner_window(), 100);
    }
}

#[test]
fn test_error_if_version_pruned() {
    let tmp_dir = TempPath::new();
    let db = AptosDB::new_for_test(&tmp_dir);
    db.state_pruner.testonly_update_min_version(5);
    db.ledger_pruner.testonly_update_min_version(10);
    assert_eq!(
        db.error_if_state_merkle_pruned("State", 4)
            .unwrap_err()
            .to_string(),
        "State at version 4 is pruned, min available version is 5."
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

    // entirely emtpy db
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

#[test]
fn test_rocksdb_properties_reporter() {
    fn get_metric() -> i64 {
        ROCKSDB_PROPERTIES
            .get_metric_with_label_values(&[
                "transaction_info",
                "aptos_rocksdb_is-file-deletions-enabled",
            ])
            .unwrap()
            .get()
    }

    assert_eq!(get_metric(), 0);
    let tmp_dir = TempPath::new();
    let _db = AptosDB::new_for_test(&tmp_dir);
    std::thread::sleep(Duration::from_secs(1));
    assert_eq!(get_metric(), 1);
}
