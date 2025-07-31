// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{
    block_executor::BlockExecutor,
    chunk_executor::ChunkExecutor,
    db_bootstrapper::{generate_waypoint, maybe_bootstrap},
    tests::{
        self, create_blocks_and_chunks, create_transaction_chunks,
        mock_vm::{encode_mint_transaction, MockVM},
    },
};
use aptos_crypto::HashValue;
use aptos_db::AptosDB;
use aptos_executor_types::{BlockExecutorTrait, ChunkExecutorTrait};
use aptos_storage_interface::DbReaderWriter;
use aptos_types::{
    ledger_info::LedgerInfoWithSignatures,
    test_helpers::transaction_test_helpers::{block, TEST_BLOCK_EXECUTOR_ONCHAIN_CONFIG},
    transaction::{TransactionListWithProofV2, Version},
};
use rand::Rng;

pub struct TestExecutor {
    _path: aptos_temppath::TempPath,
    pub db: DbReaderWriter,
    pub executor: ChunkExecutor<MockVM>,
}

impl TestExecutor {
    pub fn new() -> TestExecutor {
        let path = aptos_temppath::TempPath::new();
        path.create_as_dir().unwrap();
        let db = DbReaderWriter::new(AptosDB::new_for_test(path.path()));
        let genesis = aptos_vm_genesis::test_genesis_transaction();
        let waypoint = generate_waypoint::<MockVM>(&db, &genesis).unwrap();
        maybe_bootstrap::<MockVM>(&db, &genesis, waypoint).unwrap();
        let executor = ChunkExecutor::new(db.clone());

        TestExecutor {
            _path: path,
            db,
            executor,
        }
    }
}

fn execute_and_commit_chunks(
    chunks: [TransactionListWithProofV2; 3],
    ledger_info: LedgerInfoWithSignatures,
    db: &DbReaderWriter,
    executor: &ChunkExecutor<MockVM>,
) {
    // Execute the first chunk. After that we should still get the genesis ledger info from DB.
    executor
        .execute_chunk(chunks[0].clone(), &ledger_info, None)
        .unwrap();
    executor.commit_chunk().unwrap();
    let li = db.reader.get_latest_ledger_info().unwrap();
    assert_eq!(li.ledger_info().version(), 0);
    assert_eq!(li.ledger_info().consensus_block_id(), HashValue::zero());

    // Execute the second chunk. After that we should still get the genesis ledger info from DB.
    executor
        .execute_chunk(chunks[1].clone(), &ledger_info, None)
        .unwrap();
    executor.commit_chunk().unwrap();
    let li = db.reader.get_latest_ledger_info().unwrap();
    assert_eq!(li.ledger_info().version(), 0);
    assert_eq!(li.ledger_info().consensus_block_id(), HashValue::zero());

    // Execute the third chunk. After that we should get the new ledger info.
    executor
        .execute_chunk(chunks[2].clone(), &ledger_info, None)
        .unwrap();
    executor.commit_chunk().unwrap();
    let li = db.reader.get_latest_ledger_info().unwrap();
    assert_eq!(li, ledger_info);
}

#[test]
#[cfg_attr(feature = "consensus-only-perf-test", ignore)]
fn test_executor_execute_or_apply_and_commit_chunk() {
    let first_batch_size = 30;
    let second_batch_size = 40;
    let third_batch_size = 20;

    let first_batch_start = 1;
    let second_batch_start = first_batch_start + first_batch_size;
    let third_batch_start = second_batch_start + second_batch_size;

    let (chunks, ledger_info) = {
        create_transaction_chunks(vec![
            first_batch_start..=first_batch_start + first_batch_size - 1,
            second_batch_start..=second_batch_start + second_batch_size - 1,
            third_batch_start..=third_batch_start + third_batch_size - 1,
        ])
    };
    // First test with transactions only and reset chunks to be `Vec<TransactionOutputListWithProofV2>`.
    let chunks = {
        let TestExecutor {
            _path,
            db,
            executor,
        } = TestExecutor::new();
        execute_and_commit_chunks(
            chunks.try_into().unwrap(),
            ledger_info.clone(),
            &db,
            &executor,
        );

        let ledger_version = db.reader.expect_synced_version();
        let output1 = db
            .reader
            .get_transaction_outputs(first_batch_start, first_batch_size, ledger_version)
            .unwrap();
        let output2 = db
            .reader
            .get_transaction_outputs(second_batch_start, second_batch_size, ledger_version)
            .unwrap();
        let output3 = db
            .reader
            .get_transaction_outputs(third_batch_start, third_batch_size, ledger_version)
            .unwrap();
        [output1, output2, output3]
    };

    // Test with transaction outputs.
    let TestExecutor {
        _path,
        db,
        executor,
    } = TestExecutor::new();
    // Execute the first chunk. After that we should still get the genesis ledger info from DB.
    executor.reset().unwrap();
    executor
        .apply_chunk(chunks[0].clone(), &ledger_info, None)
        .unwrap();
    executor.commit_chunk().unwrap();
    let li = db.reader.get_latest_ledger_info().unwrap();
    assert_eq!(li.ledger_info().version(), 0);
    assert_eq!(li.ledger_info().consensus_block_id(), HashValue::zero());

    // Execute the second chunk. After that we should still get the genesis ledger info from DB.
    executor
        .apply_chunk(chunks[1].clone(), &ledger_info, None)
        .unwrap();
    executor.commit_chunk().unwrap();
    let li = db.reader.get_latest_ledger_info().unwrap();
    assert_eq!(li.ledger_info().version(), 0);
    assert_eq!(li.ledger_info().consensus_block_id(), HashValue::zero());

    // Execute the third chunk. After that we should get the new ledger info.
    executor
        .apply_chunk(chunks[2].clone(), &ledger_info, None)
        .unwrap();
    executor.commit_chunk().unwrap();
    let li = db.reader.get_latest_ledger_info().unwrap();
    assert_eq!(li, ledger_info);
}

#[test]
fn test_executor_execute_and_commit_chunk_restart() {
    let first_batch_size = 30;
    let second_batch_size = 40;

    let (chunks, ledger_info) = {
        let first_batch_start = 1;
        let second_batch_start = first_batch_start + first_batch_size;
        create_transaction_chunks(vec![
            first_batch_start..=first_batch_start + first_batch_size - 1,
            second_batch_start..=second_batch_start + second_batch_size - 1,
        ])
    };

    let TestExecutor {
        _path,
        db,
        executor,
    } = TestExecutor::new();

    // First we simulate syncing the first chunk of transactions.
    {
        executor
            .execute_chunk(chunks[0].clone(), &ledger_info, None)
            .unwrap();
        executor.commit_chunk().unwrap();
        let li = db.reader.get_latest_ledger_info().unwrap();
        assert_eq!(li.ledger_info().version(), 0);
        assert_eq!(li.ledger_info().consensus_block_id(), HashValue::zero());
    }

    // Then we restart executor and resume to the next chunk.
    {
        let executor = ChunkExecutor::<MockVM>::new(db.clone());

        executor
            .execute_chunk(chunks[1].clone(), &ledger_info, None)
            .unwrap();
        executor.commit_chunk().unwrap();
        let li = db.reader.get_latest_ledger_info().unwrap();
        assert_eq!(li, ledger_info);
    }
}

#[test]
#[cfg_attr(feature = "consensus-only-perf-test", ignore)]
fn test_executor_execute_and_commit_chunk_local_result_mismatch() {
    let first_batch_size = 10;
    let second_batch_size = 10;

    let (chunks, ledger_info) = {
        let first_batch_start = 1;
        let second_batch_start = first_batch_start + first_batch_size;
        create_transaction_chunks(vec![
            first_batch_start..=first_batch_start + first_batch_size - 1,
            second_batch_start..=second_batch_start + second_batch_size - 1,
        ])
    };

    let TestExecutor {
        _path,
        db,
        executor: chunk_manager,
    } = TestExecutor::new();

    // commit 5 txns first.
    {
        let executor = BlockExecutor::<MockVM>::new(db);
        let parent_block_id = executor.committed_block_id();
        let block_id = tests::gen_block_id(1);

        let mut rng = rand::thread_rng();
        let txns = (0..5)
            .map(|_| encode_mint_transaction(tests::gen_address(rng.gen::<u64>()), 100))
            .collect::<Vec<_>>();
        let output = executor
            .execute_block(
                (block_id, block(txns)).into(),
                parent_block_id,
                TEST_BLOCK_EXECUTOR_ONCHAIN_CONFIG,
            )
            .unwrap();
        let ledger_info = tests::gen_ledger_info(5 + 1, output.root_hash(), block_id, 1);
        executor.commit_blocks(vec![block_id], ledger_info).unwrap();
    }

    // Fork starts. Should fail.
    chunk_manager.finish();
    chunk_manager.reset().unwrap();

    assert!(chunk_manager
        .execute_chunk(chunks[1].clone(), &ledger_info, None)
        .is_err());
}

#[cfg(feature = "consensus-only-perf-test")]
#[test]
fn test_executor_execute_and_commit_chunk_without_verify() {
    use aptos_types::block_executor::config::BlockExecutorConfigFromOnchain;

    let first_batch_size = 10;
    let second_batch_size = 10;

    let (chunks, ledger_info) = {
        let first_batch_start = 1;
        let second_batch_start = first_batch_start + first_batch_size;
        tests::create_transaction_chunks(vec![
            first_batch_start..first_batch_start + first_batch_size,
            second_batch_start..second_batch_start + second_batch_size,
        ])
    };

    let TestExecutor {
        _path,
        db,
        executor: chunk_manager,
    } = TestExecutor::new();

    // commit 5 txns first.
    {
        let executor = BlockExecutor::<MockVM>::new(db);
        let parent_block_id = executor.committed_block_id();
        let block_id = tests::gen_block_id(1);

        let mut rng = rand::thread_rng();
        let txns = (0..5)
            .map(|_| encode_mint_transaction(tests::gen_address(rng.gen::<u64>()), 100))
            .collect::<Vec<_>>();
        let output = executor
            .execute_block(
                (block_id, block(txns)).into(),
                parent_block_id,
                BlockExecutorConfigFromOnchain::new_no_block_limit(),
            )
            .unwrap();
        let ledger_info = tests::gen_ledger_info(6, output.root_hash(), block_id, 1);
        executor.commit_blocks(vec![block_id], ledger_info).unwrap();
    }

    // Fork starts. Should fail.
    chunk_manager.finish();
    chunk_manager.reset().unwrap();

    assert!(chunk_manager
        .execute_chunk(chunks[1].clone(), &ledger_info, None)
        .is_ok());
}

const PRE_COMMIT_TESTS_LATEST_VERSION: Version = 10;

/// commits txn 1-3, pre-commits txn 4-7, returns txn 8-10 and ledger infos at 7 and 10
fn commit_1_pre_commit_2_return_3() -> (
    DbReaderWriter,
    TransactionListWithProofV2,
    LedgerInfoWithSignatures,
    LedgerInfoWithSignatures,
) {
    let (blocks, chunks) =
        create_blocks_and_chunks(vec![1..=3, 4..=7, 8..=10], vec![1..=3, 4..=7, 8..=10]);

    let TestExecutor {
        _path,
        db,
        executor: chunk_executor,
    } = TestExecutor::new();
    drop(chunk_executor);

    let block_executor = BlockExecutor::<MockVM>::new(db.clone());
    let mut parent_block_id = block_executor.committed_block_id();
    // execute and pre-commit block 1 & 2
    for (txns, aux_info, ledger_info) in &blocks[0..=1] {
        let block_id = ledger_info.commit_info().id();
        let output = block_executor
            .execute_block(
                (block_id, block(txns.clone()), aux_info.clone()).into(),
                parent_block_id,
                TEST_BLOCK_EXECUTOR_ONCHAIN_CONFIG,
            )
            .unwrap();
        assert_eq!(
            output.root_hash(),
            ledger_info.ledger_info().transaction_accumulator_hash()
        );
        block_executor.pre_commit_block(block_id).unwrap();
        parent_block_id = block_id;
    }
    // commit till block 1
    let ledger_info1 = blocks[0].2.clone();
    let ledger_info2 = blocks[1].2.clone();
    let ledger_info3 = blocks[2].2.clone();
    block_executor.commit_ledger(ledger_info1).unwrap();
    assert_eq!(
        ledger_info3.ledger_info().version(),
        PRE_COMMIT_TESTS_LATEST_VERSION
    );

    (db, chunks[2].clone(), ledger_info2, ledger_info3)
}

#[test]
#[should_panic(expected = "Hit error with pending pre-committed ledger, panicking.")]
fn test_panic_on_mismatch_with_pre_committed() {
    // See comments on `commit_1_pre_commit_2_return_3()`
    let (db, _chunk3, _ledger_info2, _ledger_info3) = commit_1_pre_commit_2_return_3();

    let (bad_chunks, bad_ledger_info) = create_transaction_chunks(vec![1..=7, 8..=12]);
    // bad chunk has txn 8-12
    let bad_chunk = bad_chunks[1].clone();

    let chunk_executor = ChunkExecutor::<MockVM>::new(db);
    // chunk executor knows there's pre-committed txns in the DB and when a verified chunk
    // doesn't match the pre-committed root hash it panics in hope that pre-committed versions
    // get truncated on reboot
    let _res = chunk_executor.execute_chunk(bad_chunk, &bad_ledger_info, None);
}

#[test]
fn test_continue_from_pre_committed() {
    // See comments on `commit_1_pre_commit_2_return_3()`
    let (db, chunk3, _ledger_info2, ledger_info3) = commit_1_pre_commit_2_return_3();

    let (bad_chunks, bad_ledger_info) = create_transaction_chunks(vec![1..=10, 11..=15]);
    // bad chunk has txn 11-15
    let bad_chunk = bad_chunks[1].clone();

    // continue from pre-committed version
    let chunk_executor = ChunkExecutor::<MockVM>::new(db);
    chunk_executor
        .execute_chunk(chunk3, &ledger_info3, None)
        .unwrap();
    chunk_executor.commit_chunk().unwrap();
    // once pre-committed range is committed, don't panic on errors
    assert!(chunk_executor
        .execute_chunk(bad_chunk, &bad_ledger_info, None)
        .is_err());
}
