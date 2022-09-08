// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::block_executor::BlockExecutor;
use anyhow::Result;
use aptos_crypto::{hash::SPARSE_MERKLE_PLACEHOLDER_HASH, HashValue};
use aptos_state_view::StateView;
use aptos_types::{
    ledger_info::LedgerInfoWithSignatures,
    transaction::{Transaction, TransactionOutput, TransactionToCommit, Version},
    vm_status::VMStatus,
};
use aptos_vm::VMExecutor;
use executor_types::BlockExecutorTrait;
use storage_interface::{state_delta::StateDelta, DbReader, DbReaderWriter, DbWriter};

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
        let _execution_results = executor.execute_block(block, parent_block_id);
        parent_block_id = block_id;
        block_ids.push(block_id);
    }
    let _res = executor.commit_blocks(block_ids, ledger_info_with_sigs);
}

/// A fake VM implementing VMExecutor
pub struct FakeVM;

impl VMExecutor for FakeVM {
    fn execute_block(
        _transactions: Vec<Transaction>,
        _state_view: &impl StateView,
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
