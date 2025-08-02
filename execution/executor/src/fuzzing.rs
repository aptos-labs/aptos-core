// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::block_executor::BlockExecutor;
use anyhow::Result;
use aptos_block_executor::txn_provider::default::DefaultTxnProvider;
use aptos_crypto::{hash::SPARSE_MERKLE_PLACEHOLDER_HASH, HashValue};
use aptos_executor_types::BlockExecutorTrait;
use aptos_storage_interface::{chunk_to_commit::ChunkToCommit, DbReader, DbReaderWriter, DbWriter};
use aptos_types::{
    block_executor::{
        config::BlockExecutorConfigFromOnchain, partitioner::PartitionedTransactions,
        transaction_slice_metadata::TransactionSliceMetadata,
    },
    ledger_info::LedgerInfoWithSignatures,
    state_store::{state_key::StateKey, StateView},
    test_helpers::transaction_test_helpers::TEST_BLOCK_EXECUTOR_ONCHAIN_CONFIG,
    transaction::{
        signature_verified_transaction::{
            into_signature_verified_block, SignatureVerifiedTransaction,
        },
        BlockOutput, Transaction, TransactionOutput, Version,
    },
    vm_status::VMStatus,
};
use aptos_vm::{
    sharded_block_executor::{executor_client::ExecutorClient, ShardedBlockExecutor},
    VMBlockExecutor,
};
use std::{collections::BTreeMap, sync::Arc};

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
        let sig_verified_block = into_signature_verified_block(block.1);
        let _execution_results = executor.execute_block(
            (block_id, sig_verified_block).into(),
            parent_block_id,
            TEST_BLOCK_EXECUTOR_ONCHAIN_CONFIG,
        );
        parent_block_id = block_id;
        block_ids.push(block_id);
    }
    let _res = executor.commit_blocks(block_ids, ledger_info_with_sigs);
}

/// A fake VM implementing VMBlockExecutor
pub struct FakeVM;

impl VMBlockExecutor for FakeVM {
    fn new() -> Self {
        Self
    }

    fn execute_block_sharded<S: StateView + Send + Sync, E: ExecutorClient<S>>(
        _sharded_block_executor: &ShardedBlockExecutor<S, E>,
        _transactions: PartitionedTransactions,
        _state_view: Arc<S>,
        _onchain_config: BlockExecutorConfigFromOnchain,
    ) -> Result<Vec<TransactionOutput>, VMStatus> {
        Ok(Vec::new())
    }

    fn execute_block(
        &self,
        _txn_provider: &DefaultTxnProvider<SignatureVerifiedTransaction>,
        _state_view: &impl StateView,
        _onchain_config: BlockExecutorConfigFromOnchain,
        _transaction_slice_metadata: TransactionSliceMetadata,
    ) -> Result<BlockOutput<StateKey, TransactionOutput>, VMStatus> {
        Ok(BlockOutput::new(vec![], None, BTreeMap::new()))
    }
}

/// A fake database implementing DbReader and DbWriter
pub struct FakeDb;

impl DbReader for FakeDb {
    fn get_latest_ledger_info_version(&self) -> aptos_storage_interface::Result<Version> {
        Ok(self.get_latest_ledger_info()?.ledger_info().version())
    }

    fn get_latest_commit_metadata(&self) -> aptos_storage_interface::Result<(Version, u64)> {
        let ledger_info_with_sig = self.get_latest_ledger_info()?;
        let ledger_info = ledger_info_with_sig.ledger_info();
        Ok((ledger_info.version(), ledger_info.timestamp_usecs()))
    }
}

impl DbWriter for FakeDb {
    fn pre_commit_ledger(
        &self,
        _chunk: ChunkToCommit,
        _sync_commit: bool,
    ) -> aptos_storage_interface::Result<()> {
        Ok(())
    }

    fn commit_ledger(
        &self,
        _version: Version,
        _ledger_info_with_sigs: Option<&LedgerInfoWithSignatures>,
        _chunk: Option<ChunkToCommit>,
    ) -> aptos_storage_interface::Result<()> {
        Ok(())
    }
}
