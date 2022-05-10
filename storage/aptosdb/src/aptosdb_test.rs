// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    get_first_seq_num_and_limit, test_helper,
    test_helper::{arb_blocks_to_commit, put_as_state_root, put_transaction_info},
    AptosDB, ROCKSDB_PROPERTIES,
};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_temppath::TempPath;
use aptos_types::{
    proof::SparseMerkleLeafNode,
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::{ExecutionStatus, TransactionInfo, PRE_GENESIS_VERSION},
};
use proptest::prelude::*;
use std::time::Duration;
use storage_interface::{DbReader, Order, TreeState};
use test_helper::{test_save_blocks_impl, test_sync_transactions_impl};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]

    #[test]
    fn test_save_blocks(input in arb_blocks_to_commit()) {
        test_save_blocks_impl(input);
    }

    #[test]
    fn test_sync_transactions(input in arb_blocks_to_commit()) {
        test_sync_transactions_impl(input);
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
fn test_get_latest_tree_state() {
    let tmp_dir = TempPath::new();
    let db = AptosDB::new_for_test(&tmp_dir);

    // entirely emtpy db
    let empty = db.get_latest_tree_state().unwrap();
    assert_eq!(empty, TreeState::new_empty(),);

    // unbootstrapped db with pre-genesis state
    let key = StateKey::Raw(String::from("test_key").into_bytes());
    let value = StateValue::from(String::from("test_val").into_bytes());

    put_as_state_root(&db, PRE_GENESIS_VERSION, key.clone(), value.clone());
    let hash = SparseMerkleLeafNode::new(key.hash(), value.hash()).hash();
    let pre_genesis = db.get_latest_tree_state().unwrap();
    assert_eq!(
        pre_genesis,
        TreeState::new_at_state_checkpoint(0, vec![], hash)
    );

    // bootstrapped db (any transaction info is in)
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

    let bootstrapped = db.get_latest_tree_state().unwrap();
    assert_eq!(
        bootstrapped,
        TreeState::new(
            1,
            vec![txn_info.hash()],
            txn_info.state_checkpoint_hash().unwrap(),
            Vec::new()
        ),
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
