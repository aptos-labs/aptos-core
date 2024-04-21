// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use proptest::std_facade::HashMap;
use crate::{
    db::{
        get_first_seq_num_and_limit, test_helper,
        test_helper::{
            arb_blocks_to_commit, put_as_state_root, put_transaction_auxiliary_data,
            put_transaction_infos,
        },
        AptosDB,
    },
    pruner::{LedgerPrunerManager, PrunerManager, StateMerklePrunerManager},
    schema::stale_node_index::StaleNodeIndexSchema,
};
use aptos_config::config::{
    EpochSnapshotPrunerConfig, LedgerPrunerConfig, PrunerConfig, RocksdbConfigs,
    StateMerklePrunerConfig, StorageDirPaths, BUFFERED_STATE_TARGET_ITEMS,
    DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
};
use aptos_crypto::{hash::CryptoHash, HashValue, PrivateKey, Uniform};
use aptos_storage_interface::{DbReader, ExecutedTrees, Order};
use aptos_temppath::TempPath;
use aptos_types::{
    ledger_info::LedgerInfoWithSignatures,
    proof::SparseMerkleLeafNode,
    state_store::{
        state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
    },
    transaction::{
        ExecutionStatus, TransactionAuxiliaryData, TransactionAuxiliaryDataV1, TransactionInfo,
        TransactionToCommit, VMErrorDetail, Version,
    },
    vm_status::StatusCode,
};
use proptest::prelude::*;
use std::{collections::HashSet, default, sync::Arc};
use proptest::test_runner::TestRunner;
use aptos_crypto::ed25519::{Ed25519PrivateKey, Ed25519Signature};
use aptos_executor_types::StateComputeResult;
use aptos_types::chain_id::ChainId;
use aptos_types::ledger_info::generate_ledger_info_with_sig;
use aptos_types::on_chain_config::ValidatorSet;
use aptos_types::proof::position::Position;
use aptos_types::proptest_types::{AccountInfoUniverse, BlockInfoGen, LedgerInfoGen, LedgerInfoWithSignaturesGen, ValidatorSetGen};
use aptos_types::transaction::{RawTransaction, Script, SignedTransaction, Transaction, TransactionPayload};
use aptos_types::write_set::WriteSet;
use move_core_types::account_address::AccountAddress;
use move_core_types::vm_status::StatusType::Execution;
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

        let ledger_pruner =
            LedgerPrunerManager::new(Arc::clone(&aptos_db.ledger_db), LedgerPrunerConfig {
                enable,
                prune_window: 100,
                batch_size: 1,
                user_pruning_window_offset: 0,
            });
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
        aux_1
    );
    assert_eq!(
        db.get_transaction_auxiliary_data_by_version(1).unwrap(),
        aux_2
    );
}

#[test]
fn test_get_latest_executed_trees() {
    let tmp_dir = TempPath::new();
    let db = AptosDB::new_for_test(&tmp_dir);

    // entirely empty db
    let empty = db.get_latest_executed_trees().unwrap();
    assert!(empty.is_same_view(&ExecutedTrees::new_empty()));

    // bootstrapped db (any transaction info is in)
    let key = StateKey::raw(String::from("test_key").into_bytes());
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
    put_transaction_infos(&db, 0, &[txn_info.clone()]);

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
        BUFFERED_STATE_TARGET_ITEMS,
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
        db.save_transactions_for_test(
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

    fn create_signed_transaction(gas_unit_price: u64) -> SignedTransaction {
        let private_key = Ed25519PrivateKey::generate_for_testing();
        let public_key = private_key.public_key();

        let transaction_payload = TransactionPayload::Script(Script::new(vec![], vec![], vec![]));
        let raw_transaction = RawTransaction::new(
            AccountAddress::random(),
            0,
            transaction_payload,
            0,
            gas_unit_price,
            0,
            ChainId::new(10), // This is the value used in aptos testing code.
        );
        SignedTransaction::new(raw_transaction, public_key, Ed25519Signature::dummy_signature())
    }

    #[test]
    fn test_revert_last_commit() {
        let tmp_dir = TempPath::new();
        let db = AptosDB::new_for_test(&tmp_dir);

        // Commit some transactions
        let txns_to_commit = vec![
            vec![
                TransactionToCommit::new(
                    Transaction::UserTransaction(create_signed_transaction(100)),
                    TransactionInfo::new_placeholder(
                        0,
                        Some(HashValue::random()),
                        ExecutionStatus::Success,
                    ),
                    arr_macro::arr![HashMap::new(); 16],
                    WriteSet::default(),
                    Vec::new(),
                    false,
                    TransactionAuxiliaryData::default(),
                ),
            ],
            vec![
                TransactionToCommit::new(
                    Transaction::UserTransaction(create_signed_transaction(200)),
                    TransactionInfo::new_placeholder(
                        1,
                        Some(HashValue::random()),
                        ExecutionStatus::Success,
                    ),
                    arr_macro::arr![HashMap::new(); 16],
                    WriteSet::default(),
                    Vec::new(),
                    false,
                    TransactionAuxiliaryData::default(),
                ),
            ],
        ];
        //let ledger_info_with_sigs = generate_ledger_info_with_sigs(1, &db, None);

        let commit_info_gen_strategy = BlockInfoGen::arbitrary();
        let commit_info_gen = commit_info_gen_strategy.new_tree(&mut TestRunner::default()).unwrap().current();
        let ledger_info_gen = LedgerInfoGen {
            consensus_data_hash: HashValue::random(),
            commit_info_gen,
        };

        let block_size: usize = 100;
        let universe_strategy = AccountInfoUniverse::arbitrary_with(2);
        let mut universe = universe_strategy.new_tree(&mut TestRunner::default()).unwrap().current();
        let ledger_info= ledger_info_gen.materialize(&mut universe, block_size);
        let validator_set = universe.get_validator_set(ledger_info.epoch());
        let ledger_info_sig = generate_ledger_info_with_sig(validator_set, ledger_info);
        let mut in_memory_state = db.state_store.buffered_state.lock().current_state().clone();
        let last_version= 42;
        db.save_transactions_for_test(
            &txns_to_commit[0],
            0,
            Some(0),
            Some(&ledger_info_sig),
            false,
            in_memory_state.clone(),
        ).unwrap();

        db.revert_last_commit(0, Default::default(), Some(&ledger_info_sig)).unwrap();

        // Check that the latest version is now one less
        // assert_eq!(
        //     db.get_latest_ledger_info().unwrap().ledger_info().version(),
        //     last_version - 1
        // );

        let dummy_version = 1;
        // Check that the transaction at the reverted version is no longer queryable
        assert!(db
            .get_transaction_by_version(dummy_version, last_version, false)
            .unwrap()
            .events.is_none());

        // Check that the transaction info at the reverted version is no longer queryable

        // @TODO: implement get_transaction_info_by_version on db
        // assert!(db
        //     .get_transaction_info_by_version(last_version, false)
        //     .unwrap()
        //     .is_none());

        // Check that the events at the reverted version are no longer queryable
        // @TODO: implement get_events_by_version on db
        // assert!(db
        //     .get_events_by_version(last_version, false)
        //     .unwrap()
        //     .is_empty());

        // Check that the transaction accumulator is reverted
        // @TODO implement get_hash_by_version on db
        // let position = Position::from_postorder_index(last_version).unwrap();
        // assert!(db
        //     .ledger_db
        //     .transaction_accumulator_db()
        //     .get_hash_by_position(&position)
        //     .unwrap()
        //     .is_none());
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]

    #[test]
    fn test_state_merkle_pruning(input in arb_blocks_to_commit()) {
        aptos_logger::Logger::new().init();
        test_state_merkle_pruning_impl(input);
    }
}
