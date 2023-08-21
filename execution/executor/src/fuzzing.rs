// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block_executor::{BlockExecutor, TransactionBlockExecutor},
    components::chunk_output::ChunkOutput,
};
use anyhow::Result;
use aptos_crypto::{hash::SPARSE_MERKLE_PLACEHOLDER_HASH, HashValue};
use aptos_executor_types::BlockExecutorTrait;
use aptos_state_view::StateView;
use aptos_storage_interface::{
    cached_state_view::CachedStateView, state_delta::StateDelta, DbReader, DbReaderWriter, DbWriter,
};
use aptos_types::{
    block_executor::partitioner::{ExecutableTransactions, PartitionedTransactions},
    ledger_info::LedgerInfoWithSignatures,
    test_helpers::transaction_test_helpers::BLOCK_GAS_LIMIT,
    transaction::{Transaction, TransactionOutput, TransactionToCommit, Version},
    vm_status::VMStatus,
};
use aptos_vm::{
    sharded_block_executor::{executor_client::ExecutorClient, ShardedBlockExecutor},
    VMExecutor,
};
use std::sync::Arc;

fn create_test_executor() -> BlockExecutor<FakeVM> {
    // setup fake db
    let fake_db = FakeDb {};
    let db_reader_writer = DbReaderWriter::new(fake_db);
    BlockExecutor::<FakeVM>::new(db_reader_writer)
}

pub fn fuzz_execute_and_commit_blocks(
    blocks: Vec<(HashValue, Vec<Transaction>)>,
    ledger_info_with_sigs: LedgerInfoWithSignatures,
) {
    let executor = create_test_executor();

    let mut parent_block_id = *SPARSE_MERKLE_PLACEHOLDER_HASH;
    let mut block_ids = vec![];
    for block in blocks {
        let block_id = block.0;
        let _execution_results =
            executor.execute_block(block.into(), parent_block_id, BLOCK_GAS_LIMIT);
        parent_block_id = block_id;
        block_ids.push(block_id);
    }
    let _res = executor.commit_blocks(block_ids, ledger_info_with_sigs);
}

/// A fake VM implementing VMExecutor
pub struct FakeVM;

impl TransactionBlockExecutor for FakeVM {
    fn execute_transaction_block(
        transactions: ExecutableTransactions,
        state_view: CachedStateView,
        maybe_block_gas_limit: Option<u64>,
    ) -> Result<ChunkOutput> {
        ChunkOutput::by_transaction_execution::<FakeVM>(
            transactions,
            state_view,
            maybe_block_gas_limit,
        )
    }
}

impl VMExecutor for FakeVM {
    fn execute_block_sharded<S: StateView + Send + Sync, E: ExecutorClient<S>>(
        _sharded_block_executor: &ShardedBlockExecutor<S, E>,
        _transactions: PartitionedTransactions,
        _state_view: Arc<S>,
        _maybe_block_gas_limit: Option<u64>,
    ) -> Result<Vec<TransactionOutput>, VMStatus> {
        Ok(Vec::new())
    }

    fn execute_block(
        _transactions: Vec<Transaction>,
        _state_view: &impl StateView,
        _maybe_block_gas_limit: Option<u64>,
    ) -> Result<Vec<TransactionOutput>, VMStatus> {
        Ok(Vec::new())
    }
}

/// A fake database implementing DbReader and DbWriter
pub struct FakeDb;

impl DbReader for FakeDb {
    fn get_latest_version(&self) -> Result<Version> {
        Ok(self.get_latest_ledger_info()?.ledger_info().version())
    }

    fn get_latest_commit_metadata(&self) -> Result<(Version, u64)> {
        let ledger_info_with_sig = self.get_latest_ledger_info()?;
        let ledger_info = ledger_info_with_sig.ledger_info();
        Ok((ledger_info.version(), ledger_info.timestamp_usecs()))
    }
}

impl DbWriter for FakeDb {
    fn save_transactions(
        &self,
        _txns_to_commit: &[TransactionToCommit],
        _first_version: Version,
        _base_state_version: Option<Version>,
        _ledger_info_with_sigs: Option<&LedgerInfoWithSignatures>,
        _sync_commit: bool,
        _in_memory_state: StateDelta,
    ) -> Result<()> {
        Ok(())
    }
}
