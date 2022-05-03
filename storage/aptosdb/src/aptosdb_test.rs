// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    get_first_seq_num_and_limit, schema::jellyfish_merkle_node::JellyfishMerkleNodeSchema,
    test_helper, test_helper::arb_blocks_to_commit, AptosDB, ROCKSDB_PROPERTIES,
};
use aptos_crypto::{
    hash::{CryptoHash, SPARSE_MERKLE_PLACEHOLDER_HASH},
    HashValue,
};
use aptos_jellyfish_merkle::node_type::{Node, NodeKey};
use aptos_temppath::TempPath;
use aptos_types::{
    proof::SparseMerkleLeafNode,
    state_store::{
        state_key::StateKey,
        state_value::{StateKeyAndValue, StateValue},
    },
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
    assert_eq!(
        empty,
        TreeState::new(0, vec![], *SPARSE_MERKLE_PLACEHOLDER_HASH,)
    );

    // unbootstrapped db with pre-genesis state
    let key = StateKey::Raw(String::from("test_key").into_bytes());
    let value = StateValue::from(String::from("test_val").into_bytes());

    db.db
        .put::<JellyfishMerkleNodeSchema>(
            &NodeKey::new_empty_path(PRE_GENESIS_VERSION),
            &Node::new_leaf(
                key.hash(),
                StateKeyAndValue::new(key.clone(), value.clone()),
            ),
        )
        .unwrap();
    let hash = SparseMerkleLeafNode::new(key.hash(), value.hash()).hash();
    let pre_genesis = db.get_latest_tree_state().unwrap();
    assert_eq!(pre_genesis, TreeState::new(0, vec![], hash));

    // bootstrapped db (any transaction info is in)
    let txn_info = TransactionInfo::new(
        HashValue::random(),
        HashValue::random(),
        HashValue::random(),
        Some(HashValue::random()),
        0,
        ExecutionStatus::MiscellaneousError(None),
    );
    test_helper::put_transaction_info(&db, 0, &txn_info);
    let bootstrapped = db.get_latest_tree_state().unwrap();
    assert_eq!(
        bootstrapped,
        TreeState::new(1, vec![txn_info.hash()], txn_info.state_change_hash(),)
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
