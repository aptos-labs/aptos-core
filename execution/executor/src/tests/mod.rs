// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block_executor::BlockExecutor,
    db_bootstrapper::{generate_waypoint, maybe_bootstrap},
    workflow::{do_get_execution_output::DoGetExecutionOutput, ApplyExecutionOutput},
};
use aptos_crypto::{ed25519::Ed25519PrivateKey, HashValue, PrivateKey, SigningKey, Uniform};
use aptos_db::AptosDB;
use aptos_executor_types::{
    BlockExecutorTrait, ChunkExecutorTrait, TransactionReplayer, VerifyExecutionMode,
};
use aptos_storage_interface::{
    state_store::state_view::cached_state_view::CachedStateView, DbReaderWriter, LedgerSummary,
    Result,
};
use aptos_types::{
    account_address::AccountAddress,
    aggregate_signature::AggregateSignature,
    block_executor::{
        config::BlockExecutorConfigFromOnchain,
        transaction_slice_metadata::TransactionSliceMetadata,
    },
    block_info::BlockInfo,
    bytes::NumToBytes,
    chain_id::ChainId,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    state_store::{state_key::StateKey, state_value::StateValue, StateViewId},
    test_helpers::transaction_test_helpers::{block, TEST_BLOCK_EXECUTOR_ONCHAIN_CONFIG},
    transaction::{
        signature_verified_transaction::{
            into_signature_verified_block, SignatureVerifiedTransaction,
        },
        AuxiliaryInfo, BlockEndInfo, ExecutionStatus, PersistedAuxiliaryInfo, RawTransaction,
        Script, SignedTransaction, Transaction, TransactionAuxiliaryData,
        TransactionListWithProofV2, TransactionOutput, TransactionPayload, TransactionStatus,
        Version,
    },
    write_set::{WriteOp, WriteSet, WriteSetMut},
};
use aptos_vm::VMBlockExecutor;
use itertools::Itertools;
use mock_vm::{
    encode_mint_transaction, encode_reconfiguration_transaction, encode_transfer_transaction,
    MockVM, DISCARD_STATUS, KEEP_STATUS,
};
use proptest::prelude::*;
use std::iter::once;

mod chunk_executor_tests;
#[cfg(test)]
mod mock_vm;

fn execute_and_commit_block(
    executor: &TestExecutor,
    parent_block_id: HashValue,
    txn_index: u64,
) -> HashValue {
    let txn = encode_mint_transaction(gen_address(txn_index), 100);
    let id = gen_block_id(txn_index + 1);

    let output = executor
        .execute_block(
            (id, block(vec![txn])).into(),
            parent_block_id,
            TEST_BLOCK_EXECUTOR_ONCHAIN_CONFIG,
        )
        .unwrap();
    let version = 2 * (txn_index + 1);
    assert_eq!(output.expect_last_version(), version);

    let ledger_info = gen_ledger_info(version, output.root_hash(), id, txn_index + 1);
    executor.commit_blocks(vec![id], ledger_info).unwrap();
    id
}

struct TestExecutor {
    _path: aptos_temppath::TempPath,
    db: DbReaderWriter,
    executor: BlockExecutor<MockVM>,
}

impl TestExecutor {
    fn new() -> TestExecutor {
        let path = aptos_temppath::TempPath::new();
        path.create_as_dir().unwrap();
        let db = DbReaderWriter::new(AptosDB::new_for_test(path.path()));
        let genesis = aptos_vm_genesis::test_genesis_transaction();
        let waypoint = generate_waypoint::<MockVM>(&db, &genesis).unwrap();
        maybe_bootstrap::<MockVM>(&db, &genesis, waypoint).unwrap();
        let executor = BlockExecutor::new(db.clone());

        TestExecutor {
            _path: path,
            db,
            executor,
        }
    }
}

impl std::ops::Deref for TestExecutor {
    type Target = BlockExecutor<MockVM>;

    fn deref(&self) -> &Self::Target {
        &self.executor
    }
}

impl std::ops::DerefMut for TestExecutor {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.executor
    }
}

fn gen_address(index: u64) -> AccountAddress {
    let bytes = index.to_be_bytes();
    let mut buf = [0; AccountAddress::LENGTH];
    buf[AccountAddress::LENGTH - 8..].copy_from_slice(&bytes);
    AccountAddress::new(buf)
}

fn gen_block_id(index: u64) -> HashValue {
    let bytes = index.to_be_bytes();
    let mut buf = [0; HashValue::LENGTH];
    buf[HashValue::LENGTH - 8..].copy_from_slice(&bytes);
    HashValue::new(buf)
}

fn gen_ledger_info(
    version: u64,
    root_hash: HashValue,
    commit_block_id: HashValue,
    timestamp_usecs: u64,
) -> LedgerInfoWithSignatures {
    let ledger_info = LedgerInfo::new(
        BlockInfo::new(
            0,
            0,
            commit_block_id,
            root_hash,
            version,
            timestamp_usecs,
            None,
        ),
        HashValue::zero(),
    );
    LedgerInfoWithSignatures::new(ledger_info, AggregateSignature::empty())
}

#[test]
#[cfg_attr(feature = "consensus-only-perf-test", ignore)]
fn test_executor_status() {
    let executor = TestExecutor::new();
    let parent_block_id = executor.committed_block_id();
    let block_id = gen_block_id(1);

    let txn0 = encode_mint_transaction(gen_address(0), 100);
    let txn1 = encode_mint_transaction(gen_address(1), 100);
    let txn2 = encode_transfer_transaction(gen_address(0), gen_address(1), 500);

    let output = executor
        .execute_block(
            (block_id, block(vec![txn0, txn1, txn2])).into(),
            parent_block_id,
            TEST_BLOCK_EXECUTOR_ONCHAIN_CONFIG,
        )
        .unwrap();

    assert_eq!(
        &vec![
            KEEP_STATUS.clone(),
            KEEP_STATUS.clone(),
            DISCARD_STATUS.clone(),
        ],
        output.compute_status_for_input_txns()
    );
}

#[cfg(feature = "consensus-only-perf-test")]
#[test]
fn test_executor_status_consensus_only() {
    let executor = TestExecutor::new();
    let parent_block_id = executor.committed_block_id();
    let block_id = gen_block_id(1);

    let txn0 = encode_mint_transaction(gen_address(0), 100);
    let txn1 = encode_mint_transaction(gen_address(1), 100);
    let txn2 = encode_transfer_transaction(gen_address(0), gen_address(1), 500);

    let output = executor
        .execute_block(
            (block_id, block(vec![txn0, txn1, txn2])).into(),
            parent_block_id,
            TEST_BLOCK_EXECUTOR_ONCHAIN_CONFIG,
        )
        .unwrap();

    // We should not discard any transactions because we don't actually execute them.
    assert_eq!(
        &vec![
            KEEP_STATUS.clone(),
            KEEP_STATUS.clone(),
            KEEP_STATUS.clone(),
        ],
        output.compute_status_for_input_txns()
    );
}

#[test]
fn test_executor_one_block() {
    let executor = TestExecutor::new();
    let parent_block_id = executor.committed_block_id();
    let block_id = gen_block_id(1);

    let num_user_txns = 100;
    let txns = (0..num_user_txns)
        .map(|i| encode_mint_transaction(gen_address(i), 100))
        .collect::<Vec<_>>();
    let output = executor
        .execute_block(
            (block_id, block(txns)).into(),
            parent_block_id,
            TEST_BLOCK_EXECUTOR_ONCHAIN_CONFIG,
        )
        .unwrap();
    let version = num_user_txns + 1;
    assert_eq!(output.expect_last_version(), version);
    let block_root_hash = output.root_hash();

    let ledger_info = gen_ledger_info(version, block_root_hash, block_id, 1);
    executor.commit_blocks(vec![block_id], ledger_info).unwrap();
}

#[test]
fn test_executor_multiple_blocks() {
    let executor = TestExecutor::new();
    let mut parent_block_id = executor.committed_block_id();

    for i in 0..100 {
        parent_block_id = execute_and_commit_block(&executor, parent_block_id, i);
    }
}

#[test]
#[cfg_attr(feature = "consensus-only-perf-test", ignore)]
fn test_executor_two_blocks_with_failed_txns() {
    let executor = TestExecutor::new();
    let parent_block_id = executor.committed_block_id();

    let block1_id = gen_block_id(1);
    let block2_id = gen_block_id(2);

    let block1_txns = (0..50)
        .map(|i| encode_mint_transaction(gen_address(i), 100))
        .collect::<Vec<_>>();
    let block2_txns = (0..50)
        .map(|i| {
            if i % 2 == 0 {
                encode_mint_transaction(gen_address(i + 50), 100)
            } else {
                encode_transfer_transaction(gen_address(i), gen_address(i + 1), 500)
            }
        })
        .collect::<Vec<_>>();
    let _output1 = executor
        .execute_block(
            (block1_id, block(block1_txns)).into(),
            parent_block_id,
            TEST_BLOCK_EXECUTOR_ONCHAIN_CONFIG,
        )
        .unwrap();
    let output2 = executor
        .execute_block(
            (block2_id, block(block2_txns)).into(),
            block1_id,
            TEST_BLOCK_EXECUTOR_ONCHAIN_CONFIG,
        )
        .unwrap();

    let ledger_info = gen_ledger_info(77, output2.root_hash(), block2_id, 1);
    executor
        .commit_blocks(vec![block1_id, block2_id], ledger_info)
        .unwrap();
}

#[test]
fn test_executor_commit_twice() {
    let executor = TestExecutor::new();
    let parent_block_id = executor.committed_block_id();
    let block1_txns = (0..5)
        .map(|i| encode_mint_transaction(gen_address(i), 100))
        .collect::<Vec<_>>();
    let block1_id = gen_block_id(1);
    let output1 = executor
        .execute_block(
            (block1_id, block(block1_txns)).into(),
            parent_block_id,
            TEST_BLOCK_EXECUTOR_ONCHAIN_CONFIG,
        )
        .unwrap();
    let ledger_info = gen_ledger_info(6, output1.root_hash(), block1_id, 1);
    executor.pre_commit_block(block1_id).unwrap();
    executor.commit_ledger(ledger_info.clone()).unwrap();
    executor.commit_ledger(ledger_info).unwrap();
}

#[test]
fn test_executor_execute_same_block_multiple_times() {
    let executor = TestExecutor::new();
    let parent_block_id = executor.committed_block_id();
    let block_id = gen_block_id(1);
    let version = 100;

    let txns: Vec<_> = (0..version)
        .map(|i| encode_mint_transaction(gen_address(i), 100))
        .collect();

    let mut responses = vec![];
    for _i in 0..10 {
        let output = executor
            .execute_block(
                (block_id, block(txns.clone())).into(),
                parent_block_id,
                TEST_BLOCK_EXECUTOR_ONCHAIN_CONFIG,
            )
            .unwrap();
        responses.push(output);
    }
    assert_eq!(
        responses
            .iter()
            .map(|output| output.root_hash())
            .dedup()
            .count(),
        1,
    );
}

fn create_blocks_and_chunks(
    block_ranges: Vec<std::ops::RangeInclusive<Version>>,
    chunk_ranges: Vec<std::ops::RangeInclusive<Version>>,
) -> (
    Vec<(
        Vec<Transaction>,
        Vec<AuxiliaryInfo>,
        LedgerInfoWithSignatures,
    )>,
    Vec<TransactionListWithProofV2>,
) {
    assert_eq!(*block_ranges.first().unwrap().start(), 1);
    assert_eq!(*chunk_ranges.first().unwrap().start(), 1);
    assert_eq!(
        chunk_ranges.last().unwrap().end(),
        block_ranges.last().unwrap().end(),
    );
    for i in 1..block_ranges.len() {
        let previous_range = &block_ranges[i - 1];
        let range = &block_ranges[i];
        assert!(previous_range.start() <= previous_range.end());
        assert!(range.start() <= range.end());
        assert_eq!(*range.start(), *previous_range.end() + 1);
    }
    for i in 1..chunk_ranges.len() {
        let previous_range = &chunk_ranges[i - 1];
        let range = &chunk_ranges[i];
        assert!(previous_range.start() <= previous_range.end());
        assert!(range.start() <= range.end());
        assert!(*range.start() <= *previous_range.end() + 1);
        assert!(previous_range.end() < range.end());
    }

    let mut out_blocks = Vec::new();

    // To obtain the batches of transactions, we first execute and save all these transactions in a
    // separate DB. Then we call get_transactions to retrieve them.
    let TestExecutor {
        executor: block_executor,
        db,
        ..
    } = TestExecutor::new();

    let mut parent_block_id = block_executor.committed_block_id();
    for block_range in block_ranges {
        let version = *block_range.end();
        // range_size - 1 for the block prologue
        let num_txns = *block_range.end() - *block_range.start();
        let txns: Vec<_> = block_range
            .into_iter()
            .take(num_txns as usize)
            .map(|v| encode_mint_transaction(gen_address(v), 10))
            .collect();
        let block_id = gen_block_id(version);
        let aux_info: Vec<_> = (0..num_txns)
            .map(|i| {
                AuxiliaryInfo::new(
                    PersistedAuxiliaryInfo::V1 {
                        transaction_index: i as u32,
                    },
                    None,
                )
            })
            .collect();
        let output = block_executor
            .execute_block(
                (
                    block_id,
                    into_signature_verified_block(txns.clone()),
                    aux_info.clone(),
                )
                    .into(),
                parent_block_id,
                TEST_BLOCK_EXECUTOR_ONCHAIN_CONFIG,
            )
            .unwrap();
        assert_eq!(output.expect_last_version(), version);
        block_executor.pre_commit_block(block_id).unwrap();
        let ledger_info = gen_ledger_info(version, output.root_hash(), block_id, version);
        out_blocks.push((txns, aux_info, ledger_info));
        parent_block_id = block_id;
    }
    let ledger_info = out_blocks.last().unwrap().2.clone();
    let ledger_version = ledger_info.ledger_info().version();
    block_executor.commit_ledger(ledger_info).unwrap();

    let out_chunks: Vec<_> = chunk_ranges
        .into_iter()
        .map(|range| {
            db.reader
                .get_transactions(
                    *range.start(),
                    *range.end() - *range.start() + 1,
                    ledger_version,
                    false, /* fetch_events */
                )
                .unwrap()
        })
        .collect();

    (out_blocks, out_chunks)
}

fn create_transaction_chunks(
    chunks: Vec<std::ops::RangeInclusive<Version>>,
) -> (Vec<TransactionListWithProofV2>, LedgerInfoWithSignatures) {
    let num_txns = *chunks.last().unwrap().end();
    // last txn is a block epilogue
    let all_txns = 1..=num_txns;
    let (mut blocks, chunks) = create_blocks_and_chunks(vec![all_txns], chunks);

    (chunks, blocks.pop().unwrap().2)
}

fn create_test_transaction(sequence_number: u64) -> Transaction {
    let private_key = Ed25519PrivateKey::generate_for_testing();
    let public_key = private_key.public_key();

    // TODO[Orderless]: Change this to payload v2 format
    let transaction_payload = TransactionPayload::Script(Script::new(vec![], vec![], vec![]));
    let raw_transaction = RawTransaction::new(
        AccountAddress::random(),
        sequence_number,
        transaction_payload,
        0,
        0,
        0,
        ChainId::new(10),
    );
    let signed_transaction = SignedTransaction::new(
        raw_transaction.clone(),
        public_key,
        private_key.sign(&raw_transaction).unwrap(),
    );

    Transaction::UserTransaction(signed_transaction)
}

fn apply_transaction_by_writeset(
    db: &DbReaderWriter,
    transactions_and_writesets: Vec<(Transaction, WriteSet)>,
) {
    let ledger_summary: LedgerSummary = db.reader.get_pre_committed_ledger_summary().unwrap();

    let (txns, txn_outs): (Vec<_>, Vec<_>) = transactions_and_writesets
        .iter()
        .map(|(txn, write_set)| {
            (
                txn.clone(),
                TransactionOutput::new(
                    write_set.clone(),
                    vec![],
                    0,
                    TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(None)),
                    TransactionAuxiliaryData::default(),
                ),
            )
        })
        .chain(once((
            Transaction::StateCheckpoint(HashValue::random()),
            TransactionOutput::new(
                WriteSet::default(),
                Vec::new(),
                0,
                TransactionStatus::Keep(ExecutionStatus::Success),
                TransactionAuxiliaryData::default(),
            ),
        )))
        .unzip();

    let state_view = CachedStateView::new(
        StateViewId::Miscellaneous,
        db.reader.clone(),
        ledger_summary.state.latest().clone(),
    )
    .unwrap();
    let aux_info = txns.iter().map(|_| AuxiliaryInfo::new_empty()).collect();
    let chunk_output = DoGetExecutionOutput::by_transaction_output(
        txns,
        txn_outs,
        aux_info,
        &ledger_summary.state,
        state_view,
    )
    .unwrap();

    let output =
        ApplyExecutionOutput::run(chunk_output, ledger_summary, db.reader.as_ref()).unwrap();

    db.writer
        .save_transactions(
            output.expect_complete_result().as_chunk_to_commit(),
            None,
            true, /* sync_commit */
        )
        .unwrap();
}

#[test]
fn test_deleted_key_from_state_store() {
    let executor = TestExecutor::new();
    let db = &executor.db;
    let dummy_state_key1 = StateKey::raw(b"test_key1");
    let dummy_value1 = 10u64.le_bytes();
    let dummy_state_key2 = StateKey::raw(b"test_key2");
    let dummy_value2 = 20u64.le_bytes();
    // Create test transaction, event and transaction output
    let transaction1 = create_test_transaction(0);
    let transaction2 = create_test_transaction(1);
    let write_set1 = WriteSetMut::new(vec![(
        dummy_state_key1.clone(),
        WriteOp::legacy_modification(dummy_value1.clone()),
    )])
    .freeze()
    .unwrap();

    let write_set2 = WriteSetMut::new(vec![(
        dummy_state_key2.clone(),
        WriteOp::legacy_modification(dummy_value2.clone()),
    )])
    .freeze()
    .unwrap();

    apply_transaction_by_writeset(db, vec![
        (transaction1, write_set1),
        (transaction2, write_set2),
    ]);

    let state_value1_from_db = db
        .reader
        .get_state_value_with_proof_by_version(&dummy_state_key1, 3)
        .unwrap()
        .0
        .unwrap();

    let state_value2_from_db = db
        .reader
        .get_state_value_with_proof_by_version(&dummy_state_key2, 3)
        .unwrap()
        .0
        .unwrap();

    // Ensure both the keys have been successfully written in the DB
    assert_eq!(state_value1_from_db, StateValue::from(dummy_value1.clone()));
    assert_eq!(state_value2_from_db, StateValue::from(dummy_value2.clone()));

    let transaction3 = create_test_transaction(2);
    let write_set3 = WriteSetMut::new(vec![(dummy_state_key1.clone(), WriteOp::legacy_deletion())])
        .freeze()
        .unwrap();

    apply_transaction_by_writeset(db, vec![(transaction3, write_set3)]);

    // Ensure the latest version of the value in DB is None (which implies its deleted)
    assert!(db
        .reader
        .get_state_value_with_proof_by_version(&dummy_state_key1, 5)
        .unwrap()
        .0
        .is_none());

    // Ensure the key that was not touched by the transaction is not accidentally deleted
    let state_value_from_db2 = db
        .reader
        .get_state_value_with_proof_by_version(&dummy_state_key2, 5)
        .unwrap()
        .0
        .unwrap();
    assert_eq!(state_value_from_db2, StateValue::from(dummy_value2));

    // Ensure the previous version of the deleted key is not accidentally deleted
    let state_value_from_db1 = db
        .reader
        .get_state_value_with_proof_by_version(&dummy_state_key1, 3)
        .unwrap()
        .0
        .unwrap();

    assert_eq!(state_value_from_db1, StateValue::from(dummy_value1));
}

#[test]
fn test_reconfig_suffix_empty_blocks() {
    let TestExecutor {
        _path,
        db: _,
        executor,
    } = TestExecutor::new();
    let block_a = TestBlock::new(100, 1, gen_block_id(1));
    // add block gas limit to be consistent with block executor that will add state checkpoint txn
    let mut block_b = TestBlock::new(100, 1, gen_block_id(2));
    let block_c = TestBlock::new(10, 1, gen_block_id(3));
    let block_d = TestBlock::new(10, 1, gen_block_id(4));
    block_b
        .txns
        .push(encode_reconfiguration_transaction().into());
    let parent_block_id = executor.committed_block_id();
    executor
        .execute_block(
            (block_a.id, block_a.txns).into(),
            parent_block_id,
            TEST_BLOCK_EXECUTOR_ONCHAIN_CONFIG,
        )
        .unwrap();
    let output2 = executor
        .execute_block(
            (block_b.id, block_b.txns).into(),
            block_a.id,
            TEST_BLOCK_EXECUTOR_ONCHAIN_CONFIG,
        )
        .unwrap();
    let output3 = executor
        .execute_block(
            (block_c.id, block_c.txns).into(),
            block_b.id,
            TEST_BLOCK_EXECUTOR_ONCHAIN_CONFIG,
        )
        .unwrap();
    let output4 = executor
        .execute_block(
            (block_d.id, block_d.txns).into(),
            block_c.id,
            TEST_BLOCK_EXECUTOR_ONCHAIN_CONFIG,
        )
        .unwrap();

    assert_eq!(output2.root_hash(), output3.root_hash());
    assert_eq!(output2.root_hash(), output4.root_hash());
    let ledger_info = gen_ledger_info(202, output2.root_hash(), block_d.id, 1);

    executor
        .commit_blocks(
            vec![block_a.id, block_b.id, block_c.id, block_d.id],
            ledger_info,
        )
        .unwrap();
}

struct TestBlock {
    txns: Vec<SignatureVerifiedTransaction>,
    id: HashValue,
}

impl TestBlock {
    fn new(num_user_txns: u64, amount: u32, id: HashValue) -> Self {
        let txns = if num_user_txns == 0 {
            Vec::new()
        } else {
            block(
                (0..num_user_txns)
                    .map(|index| encode_mint_transaction(gen_address(index), u64::from(amount)))
                    .collect(),
            )
        };
        TestBlock { txns, id }
    }

    fn inner_txns(&self) -> Vec<Transaction> {
        self.txns.iter().map(|t| t.clone().into_inner()).collect()
    }
}

// Executes a list of transactions by executing and immediately committing one at a time. Returns
// the root hash after all transactions are committed.
fn run_transactions_naive(
    transactions: Vec<SignatureVerifiedTransaction>,
    block_executor_onchain_config: BlockExecutorConfigFromOnchain,
) -> HashValue {
    let executor = TestExecutor::new();
    let db = &executor.db;

    for txn in transactions {
        let ledger_summary = db.reader.get_pre_committed_ledger_summary().unwrap();
        let state_view = CachedStateView::new(
            StateViewId::Miscellaneous,
            db.reader.clone(),
            ledger_summary.state.latest().clone(),
        )
        .unwrap();
        let out = DoGetExecutionOutput::by_transaction_execution(
            &MockVM::new(),
            vec![txn].into(),
            vec![AuxiliaryInfo::new_empty()],
            &ledger_summary.state,
            state_view,
            block_executor_onchain_config.clone(),
            TransactionSliceMetadata::unknown(),
        )
        .unwrap();
        let output = ApplyExecutionOutput::run(out, ledger_summary, db.reader.as_ref()).unwrap();
        db.writer
            .save_transactions(
                output.expect_complete_result().as_chunk_to_commit(),
                None,
                true, /* sync_commit */
            )
            .unwrap();
    }
    db.reader
        .get_pre_committed_ledger_summary()
        .unwrap()
        .transaction_accumulator
        .root_hash()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(5))]

    #[test]
    #[cfg_attr(feature = "consensus-only-perf-test", ignore)]
    fn test_reconfiguration_with_retry_transaction_status(
        (num_user_txns, reconfig_txn_index) in (2..5usize).prop_flat_map(|num_user_txns| {
            (
                Just(num_user_txns),
                0..num_user_txns - 1 // avoid state checkpoint right after reconfig
            )
    }).no_shrink()) {
        let executor = TestExecutor::new();

        let block_id = gen_block_id(1);
        let mut block = TestBlock::new(num_user_txns as u64, 10, block_id);
        let num_input_txns = block.txns.len();
        block.txns[reconfig_txn_index] = encode_reconfiguration_transaction().into();

        let parent_block_id = executor.committed_block_id();
        let output = executor.execute_block(
            (block_id, block.txns.clone()).into(), parent_block_id, TEST_BLOCK_EXECUTOR_ONCHAIN_CONFIG
        ).unwrap();

        // assert: txns after the reconfiguration are with status "Retry"
        let retry_iter = output.compute_status_for_input_txns().iter()
        .skip_while(|status| matches!(*status, TransactionStatus::Keep(_)));
        prop_assert_eq!(
            retry_iter.take_while(|status| matches!(*status,TransactionStatus::Retry)).count(),
            num_input_txns - reconfig_txn_index - 1
        );

        // commit
        let ledger_info = gen_ledger_info(
            reconfig_txn_index as Version + 1 /* version */,
            output.root_hash(),
            block_id,
            1 /* timestamp */
        );
        executor.commit_blocks(vec![block_id], ledger_info).unwrap();
        let parent_block_id = executor.committed_block_id();

        // retry txns after reconfiguration
        let retry_txns = block.txns.iter().skip(reconfig_txn_index + 1).cloned().collect_vec();
        let retry_block_id = gen_block_id(2);
        let retry_output = executor.execute_block(
            ( retry_block_id, retry_txns ).into(),
            parent_block_id,
            TEST_BLOCK_EXECUTOR_ONCHAIN_CONFIG
        ).unwrap();
        prop_assert!(retry_output.compute_status_for_input_txns().iter().all(|s| matches!(*s, TransactionStatus::Keep(_))));

        // Second block has StateCheckpoint/BlockPrologue transaction added.
        let ledger_version = num_input_txns as Version + 1;

        // commit
        let ledger_info = gen_ledger_info(
            ledger_version,
            retry_output.root_hash(),
            retry_block_id,
            12345 /* timestamp */
        );
        executor.commit_blocks(vec![retry_block_id], ledger_info).unwrap();

        // get txn_infos from db
        let db = executor.db.reader.clone();
        prop_assert_eq!(db.expect_synced_version(), ledger_version);
        let (txn_list, persisted_aux_info)= db.get_transactions(
            1, /* start version */
            ledger_version, /* version */
            ledger_version, /* ledger version */
            false /* fetch events */
        ).unwrap().into_parts();
        prop_assert_eq!(&block.inner_txns(), &txn_list.transactions[..num_input_txns]);
        let txn_infos = txn_list.proof.transaction_infos;
        let write_sets = db.get_write_set_iterator(1, ledger_version).unwrap().collect::<Result<_>>().unwrap();
        let event_vecs = db.get_events_iterator(1, ledger_version).unwrap().collect::<Result<_>>().unwrap();

        // replay txns in one batch across epoch boundary,
        // and the replayer should deal with `Retry`s automatically
        let replayer = chunk_executor_tests::TestExecutor::new();
        let chunks_enqueued = replayer.executor.enqueue_chunks(
            txn_list.transactions,
            persisted_aux_info,
            txn_infos,
            write_sets,
            event_vecs,
            &VerifyExecutionMode::verify_all()
        ).unwrap();
        assert_eq!(chunks_enqueued, 2);
        replayer.executor.update_ledger().unwrap();
        replayer.executor.update_ledger().unwrap();

        replayer.executor.commit().unwrap();
        replayer.executor.commit().unwrap();
        prop_assert!(replayer.executor.is_empty());
        let replayed_db = replayer.db.reader.clone();
        prop_assert_eq!(
            replayed_db.get_accumulator_root_hash(ledger_version).unwrap(),
            db.get_accumulator_root_hash(ledger_version).unwrap()
        );
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1))]

    #[test]
    #[cfg_attr(feature = "consensus-only-perf-test", ignore)]
    fn test_executor_restart(a_size in 1..5u64, b_size in 1..5u64, amount in any::<u32>()) {
        let TestExecutor { _path, db, executor } = TestExecutor::new();

        let block_a = TestBlock::new(a_size, amount, gen_block_id(1));
        let block_b = TestBlock::new(b_size, amount, gen_block_id(2));

        let mut parent_block_id;
        let mut root_hash;

        // First execute and commit one block, then destroy executor.
        {
            parent_block_id = executor.committed_block_id();
            let output_a = executor.execute_block(
                (block_a.id, block_a.txns.clone()).into(), parent_block_id, TEST_BLOCK_EXECUTOR_ONCHAIN_CONFIG
            ).unwrap();
            root_hash = output_a.root_hash();
            // Add one transaction for the state checkpoint.
            let ledger_info = gen_ledger_info(block_a.txns.len() as u64 + 1, root_hash, block_a.id, 1);
            executor.commit_blocks(vec![block_a.id], ledger_info).unwrap();
            parent_block_id = block_a.id;
            drop(executor);
        }

        // Now we construct a new executor and run one more block.
        {
            let executor = BlockExecutor::<MockVM>::new(db);
            let output_b = executor.execute_block((block_b.id, block_b.txns.clone()).into(), parent_block_id, TEST_BLOCK_EXECUTOR_ONCHAIN_CONFIG).unwrap();
            root_hash = output_b.root_hash();
            let ledger_info = gen_ledger_info(
                // add two transactions for the state checkpoints
                (block_a.txns.len() + block_b.txns.len() + 2) as u64,
                root_hash,
                block_b.id,
                2,
            );
            executor.commit_blocks(vec![block_b.id], ledger_info).unwrap();
        };

        let expected_root_hash = run_transactions_naive({
            let mut txns = vec![];
            txns.extend(block_a.txns.iter().cloned());
            txns.push(SignatureVerifiedTransaction::Valid(Transaction::block_epilogue_v0(block_a.id, BlockEndInfo::new_empty())));
            txns.extend(block_b.txns.iter().cloned());
            txns.push(SignatureVerifiedTransaction::Valid(Transaction::block_epilogue_v0(block_b.id, BlockEndInfo::new_empty())));
            txns
        }, TEST_BLOCK_EXECUTOR_ONCHAIN_CONFIG);

        prop_assert_eq!(root_hash, expected_root_hash);
    }
}
