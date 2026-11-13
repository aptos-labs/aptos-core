// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

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
use aptos_executor_types::{
    BlockExecutorTrait, ChunkExecutorTrait, TransactionReplayer, VerifyExecutionMode,
};
use aptos_storage_interface::DbReaderWriter;
use aptos_types::{
    account_config::NEW_EPOCH_EVENT_V2_MOVE_TYPE_TAG,
    contract_event::ContractEvent,
    ledger_info::LedgerInfoWithSignatures,
    test_helpers::transaction_test_helpers::{block, TEST_BLOCK_EXECUTOR_ONCHAIN_CONFIG},
    transaction::{TransactionListWithProofV2, TransactionOutputListWithProofV2, Version},
};
use rand::Rng;
use std::ops::RangeInclusive;

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
fn test_finished_executor_rejects_chunks_without_panicking() {
    let batch_size: u64 = 10;
    // A single chunk of versions, not a list of versions.
    #[allow(clippy::single_range_in_vec_init)]
    let (chunks, ledger_info) = create_transaction_chunks(vec![1..=batch_size]);

    let TestExecutor {
        _path,
        db: _db,
        executor,
    } = TestExecutor::new();
    executor.reset().unwrap();
    executor.finish();

    // The outputs path used to `.expect("not reset")` here and take the node down.
    assert!(executor
        .enqueue_chunk_by_transaction_outputs(
            TransactionOutputListWithProofV2::new_empty(),
            &ledger_info,
            None,
        )
        .is_err());
    assert!(executor
        .enqueue_chunk_by_execution(chunks[0].clone(), &ledger_info, None)
        .is_err());
    assert!(executor.update_ledger().is_err());
    assert!(executor.commit_chunk().is_err());
    assert!(executor.is_empty());

    // A reset brings it back to a usable state.
    executor.reset().unwrap();
    executor
        .execute_chunk(chunks[0].clone(), &ledger_info, None)
        .unwrap();
    executor.commit_chunk().unwrap();
}

#[test]
fn test_executor_execute_and_commit_chunk_restart() {
    let (blocks, chunks) = create_blocks_and_chunks(vec![1..=20, 21..=70], vec![1..=30, 21..=70]);
    let ledger_info = blocks.last().unwrap().2.clone();

    let TestExecutor {
        _path,
        db,
        executor,
    } = TestExecutor::new();

    // First we simulate syncing the first chunk of transactions, which commits only the first
    // block.
    {
        executor
            .execute_chunk(chunks[0].clone(), &ledger_info, None)
            .unwrap();
        executor.commit_chunk().unwrap();
        assert_eq!(db.reader.expect_synced_version(), 20);
        let li = db.reader.get_latest_ledger_info().unwrap();
        assert_eq!(li.ledger_info().version(), 0);
        assert_eq!(li.ledger_info().consensus_block_id(), HashValue::zero());
    }

    // Then we restart executor, which drops the partial block held back, and resume from what's
    // committed.
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
            .map(|_| encode_mint_transaction(tests::gen_address(rng.r#gen::<u64>()), 100))
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
            .map(|_| encode_mint_transaction(tests::gen_address(rng.r#gen::<u64>()), 100))
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
#[should_panic(expected = "Hit error with pending pre-committed ledger, panicking.")]
fn test_panic_on_mismatch_with_pre_committed_after_partial_block() {
    // See comments on `commit_1_pre_commit_2_return_3()`
    let (db, _chunk3, _ledger_info2, _ledger_info3) = commit_1_pre_commit_2_return_3();

    let (bad_chunks, bad_ledger_info) = create_transaction_chunks(vec![1..=7, 8..=9, 10..=12]);

    let chunk_executor = ChunkExecutor::<MockVM>::new(db);
    // Txn 8-9 don't complete a block, so committing them verifies nothing about the pre-committed
    // txns, and the mismatch is only found with txn 10-12.
    chunk_executor
        .execute_chunk(bad_chunks[1].clone(), &bad_ledger_info, None)
        .unwrap();
    chunk_executor.commit_chunk().unwrap();
    let _res = chunk_executor.execute_chunk(bad_chunks[2].clone(), &bad_ledger_info, None);
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

/// A ledger of blocks of the given version ranges. All transactions mint to the same account, so
/// that each depends on the ones before it.
struct TestLedger {
    source: tests::TestExecutor,
    ledger_info: LedgerInfoWithSignatures,
}

impl TestLedger {
    fn new(block_ranges: &[RangeInclusive<Version>]) -> Self {
        let source = tests::TestExecutor::new();
        let account = tests::gen_address(0);

        let mut parent_block_id = source.committed_block_id();
        let mut ledger_info = None;
        for range in block_ranges {
            let version = *range.end();
            let block_id = tests::gen_block_id(version);
            // The block epilogue takes the last version.
            let txns = (*range.start()..version)
                .map(|v| encode_mint_transaction(account, v))
                .collect();
            let output = source
                .execute_block(
                    (block_id, block(txns)).into(),
                    parent_block_id,
                    TEST_BLOCK_EXECUTOR_ONCHAIN_CONFIG,
                )
                .unwrap();
            assert_eq!(output.expect_last_version(), version);
            source.pre_commit_block(block_id).unwrap();
            ledger_info = Some(tests::gen_ledger_info(
                version,
                output.root_hash(),
                block_id,
                version,
            ));
            parent_block_id = block_id;
        }
        let ledger_info = ledger_info.unwrap();
        source.commit_ledger(ledger_info.clone()).unwrap();

        Self {
            source,
            ledger_info,
        }
    }

    fn version(&self) -> Version {
        self.ledger_info.ledger_info().version()
    }

    fn transactions(&self, range: &RangeInclusive<Version>) -> TransactionListWithProofV2 {
        let num_txns = range.end() - range.start() + 1;
        self.source
            .db
            .reader
            .get_transactions(*range.start(), num_txns, self.version(), false)
            .unwrap()
    }

    fn outputs(&self, range: &RangeInclusive<Version>) -> TransactionOutputListWithProofV2 {
        let num_txns = range.end() - range.start() + 1;
        self.source
            .db
            .reader
            .get_transaction_outputs(*range.start(), num_txns, self.version())
            .unwrap()
    }

    /// Replays `range`, with the `TransactionInfo`s of `versions_to_fail` replaced by the ones before
    /// them, so that these transactions fail verification if executed.
    fn replay(
        &self,
        executor: &ChunkExecutor<MockVM>,
        range: &RangeInclusive<Version>,
        versions_to_fail: &[Version],
        verify_execution_mode: &VerifyExecutionMode,
    ) -> anyhow::Result<usize> {
        let (txn_list, persisted_aux_info) = self.transactions(range).into_parts();
        let mut txn_infos = txn_list.proof.transaction_infos;
        for version in versions_to_fail.iter().filter(|v| range.contains(*v)) {
            let idx = (version - range.start()) as usize;
            // Copying a block end, or over one, would change the blocks.
            assert!(
                !txn_infos[idx - 1].has_state_checkpoint_hash()
                    && !txn_infos[idx].has_state_checkpoint_hash(),
                "Txn {version} or the one before it ends a block."
            );
            txn_infos[idx] = txn_infos[idx - 1].clone();
        }
        let num_txns = range.end() - range.start() + 1;
        let reader = &self.source.db.reader;
        let write_sets = reader
            .get_write_set_iterator(*range.start(), num_txns)
            .unwrap()
            .map(|write_set| write_set.unwrap())
            .collect();
        let event_vecs = reader
            .get_events_iterator(*range.start(), num_txns)
            .unwrap()
            .map(|events| events.unwrap())
            .collect();
        executor.enqueue_chunks(
            txn_list.transactions,
            persisted_aux_info,
            txn_infos,
            write_sets,
            event_vecs,
            verify_execution_mode,
        )
    }
}

#[test]
fn test_executor_executes_whole_blocks() {
    let ledger = TestLedger::new(&[1..=10, 11..=25, 26..=40]);
    // Chunks and the versions synced after committing them, i.e. up to the last block end.
    let chunks = [(1..=5, 0), (6..=18, 10), (19..=22, 10), (23..=40, 40)];

    for by_execution in [
        [true, true, true, true],
        [false, false, false, false],
        [true, false, true, false],
        [false, true, false, true],
    ] {
        let TestExecutor {
            _path,
            db,
            executor,
        } = TestExecutor::new();

        // Enqueue all before updating the ledger, as state sync pipelines them.
        for ((range, _), by_execution) in chunks.iter().zip(by_execution) {
            if by_execution {
                executor
                    .enqueue_chunk_by_execution(
                        ledger.transactions(range),
                        &ledger.ledger_info,
                        None,
                    )
                    .unwrap();
            } else {
                executor
                    .enqueue_chunk_by_transaction_outputs(
                        ledger.outputs(range),
                        &ledger.ledger_info,
                        None,
                    )
                    .unwrap();
            }
        }

        let mut synced_version = 0;
        for (_, expected_synced_version) in &chunks {
            executor.update_ledger().unwrap();
            let notification = executor.commit_chunk().unwrap();
            assert_eq!(
                notification.committed_transactions.len() as Version,
                expected_synced_version - synced_version,
            );
            synced_version = db.reader.expect_synced_version();
            assert_eq!(synced_version, *expected_synced_version);
        }
        assert!(executor.is_empty());
        assert_eq!(
            db.reader.get_latest_ledger_info().unwrap(),
            ledger.ledger_info
        );
    }
}

#[test]
fn test_replayer_executes_whole_blocks() {
    let ledger = TestLedger::new(&[1..=10, 11..=25, 26..=40]);
    let TestExecutor {
        _path,
        db,
        executor,
    } = TestExecutor::new();

    // The batches split a block, which also has a transaction skipping verification.
    let verify_execution_mode = VerifyExecutionMode::verify_except(vec![15]);
    let mut num_chunks = 0;
    for range in [1..=18, 19..=40] {
        let (txn_list, persisted_aux_info) = ledger.transactions(&range).into_parts();
        let num_txns = range.end() - range.start() + 1;
        let reader = &ledger.source.db.reader;
        let write_sets = reader
            .get_write_set_iterator(*range.start(), num_txns)
            .unwrap()
            .map(|write_set| write_set.unwrap())
            .collect();
        let event_vecs = reader
            .get_events_iterator(*range.start(), num_txns)
            .unwrap()
            .map(|events| events.unwrap())
            .collect();
        num_chunks += executor
            .enqueue_chunks(
                txn_list.transactions,
                persisted_aux_info,
                txn_list.proof.transaction_infos,
                write_sets,
                event_vecs,
                &verify_execution_mode,
            )
            .unwrap();
    }

    for _ in 0..num_chunks {
        executor.update_ledger().unwrap();
        executor.commit().unwrap();
    }
    assert!(executor.is_empty());
    assert_eq!(db.reader.expect_synced_version(), ledger.version());
    assert_eq!(
        db.reader
            .get_accumulator_root_hash(ledger.version())
            .unwrap(),
        ledger
            .ledger_info
            .ledger_info()
            .transaction_accumulator_hash(),
    );
}

#[test]
fn test_replayer_verifies_whole_blocks() {
    let ledger = TestLedger::new(&[1..=10, 11..=25, 26..=40]);
    // The first batch enqueues 1-10, and 11-18 to be held back. The second one enqueues 19-40 in
    // one go, or 19-25 and 26-40 if block 11-25 isn't verified to its end.
    let batches = [1..=18, 19..=40];

    // Each batch is expected to enqueue a number of chunks, or to fail at a version.
    for (verify_execution_mode, versions_to_fail, expected_results) in [
        // Txns held back are verified once the block is complete.
        (VerifyExecutionMode::verify_all(), vec![15], [
            Ok(2),
            Err(15),
        ]),
        // The block with a txn to skip goes unverified, including the part held back.
        (VerifyExecutionMode::verify_except(vec![15]), vec![20], [
            Ok(2),
            Ok(2),
        ]),
        (VerifyExecutionMode::verify_except(vec![20]), vec![13], [
            Ok(2),
            Ok(2),
        ]),
        // Verification resumes at the next block.
        (VerifyExecutionMode::verify_except(vec![15]), vec![30], [
            Ok(2),
            Err(30),
        ]),
    ] {
        let TestExecutor {
            _path,
            db: _,
            executor,
        } = TestExecutor::new();
        for (range, expected_result) in batches.iter().zip(expected_results) {
            let result = ledger.replay(&executor, range, &versions_to_fail, &verify_execution_mode);
            match expected_result {
                Ok(num_chunks) => {
                    assert_eq!(result.unwrap(), num_chunks);
                },
                Err(version) => {
                    let error = result.unwrap_err();
                    assert!(
                        error.to_string().contains(&format!("version:{version},")),
                        "{error:?}"
                    );
                },
            }
        }
    }

    // Lazy quit doesn't stop at a mismatch, which is still found only once the block is complete.
    // Verification then resumes at the next block.
    let verify_execution_mode = VerifyExecutionMode::verify_all().set_lazy_quit(true);
    let TestExecutor {
        _path,
        db: _,
        executor,
    } = TestExecutor::new();
    for (range, (expected_num_chunks, expected_seen_error)) in
        batches.iter().zip([(2, false), (2, true)])
    {
        let num_chunks = ledger
            .replay(&executor, range, &[15], &verify_execution_mode)
            .unwrap();
        assert_eq!(num_chunks, expected_num_chunks);
        assert_eq!(verify_execution_mode.seen_error(), expected_seen_error);
    }
}

#[test]
fn test_replayer_keeps_partial_block_on_failure() {
    let ledger = TestLedger::new(&[1..=10, 11..=25, 26..=40]);
    let TestExecutor {
        _path,
        db,
        executor,
    } = TestExecutor::new();

    let reader = &ledger.source.db.reader;
    let replay = |range: RangeInclusive<Version>, forge_epoch_change: bool| {
        let (txn_list, persisted_aux_info) = ledger.transactions(&range).into_parts();
        let num_txns = range.end() - range.start() + 1;
        let write_sets = reader
            .get_write_set_iterator(*range.start(), num_txns)
            .unwrap()
            .map(|write_set| write_set.unwrap())
            .collect();
        let mut event_vecs: Vec<_> = reader
            .get_events_iterator(*range.start(), num_txns)
            .unwrap()
            .map(|events| events.unwrap())
            .collect();
        if forge_epoch_change {
            // Fails applying the outputs, since the new validator set isn't written.
            event_vecs.last_mut().unwrap().push(
                ContractEvent::new_v2(
                    NEW_EPOCH_EVENT_V2_MOVE_TYPE_TAG.clone(),
                    bcs::to_bytes(&0).unwrap(),
                )
                .unwrap(),
            );
        }
        executor.enqueue_chunks(
            txn_list.transactions,
            persisted_aux_info,
            txn_list.proof.transaction_infos,
            write_sets,
            event_vecs,
            &VerifyExecutionMode::NoVerify,
        )
    };

    // Txn 11-18 are held back.
    let mut num_chunks = replay(1..=18, false).unwrap();
    // Txn 19-25 complete the block, which fails to be applied.
    let error = replay(19..=25, true).unwrap_err();
    assert!(
        error.to_string().contains("ValidatorSet not touched"),
        "{error:?}"
    );
    // The retry still has the partial block held back.
    num_chunks += replay(19..=40, false).unwrap();

    for _ in 0..num_chunks {
        executor.update_ledger().unwrap();
        executor.commit().unwrap();
    }
    assert!(executor.is_empty());
    assert_eq!(db.reader.expect_synced_version(), ledger.version());
    assert_eq!(
        db.reader
            .get_accumulator_root_hash(ledger.version())
            .unwrap(),
        ledger
            .ledger_info
            .ledger_info()
            .transaction_accumulator_hash(),
    );
}
