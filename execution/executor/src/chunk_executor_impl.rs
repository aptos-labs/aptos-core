// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::logging::{LogEntry, LogSchema};
use diem_logger::prelude::*;
use diem_types::{
    contract_event::ContractEvent, ledger_info::LedgerInfoWithSignatures, protocol_spec::DpnProto,
    transaction::default_protocol::TransactionListWithProof,
};
use diem_vm::VMExecutor;
use executor_types::{ChunkExecutor, ProcessedVMOutput};
use fail::fail_point;

use crate::{metrics::DIEM_EXECUTOR_EXECUTE_AND_COMMIT_CHUNK_SECONDS, Executor};
use diem_types::transaction::TransactionToCommit;

impl<V: VMExecutor> ChunkExecutor for Executor<DpnProto, V> {
    fn execute_chunk(
        &self,
        txn_list_with_proof: TransactionListWithProof,
        // Target LI that has been verified independently: the proofs are relative to this version.
        verified_target_li: LedgerInfoWithSignatures,
    ) -> anyhow::Result<(
        ProcessedVMOutput,
        Vec<TransactionToCommit>,
        Vec<ContractEvent>,
    )> {
        let _timer = DIEM_EXECUTOR_EXECUTE_AND_COMMIT_CHUNK_SECONDS.start_timer();
        // 1. Update the cache in executor to be consistent with latest synced state.
        self.reset_cache()?;
        let read_lock = self.cache.read();

        // 2. Verify input transaction list.
        let num_txn = txn_list_with_proof.transactions.len();
        let first_version_in_request = txn_list_with_proof.first_transaction_version;
        let (transactions, transaction_infos) =
            self.verify_chunk(txn_list_with_proof, &verified_target_li)?;

        // 3. Execute transactions.
        let first_version = read_lock.synced_trees().txn_accumulator().num_leaves();
        drop(read_lock);
        let (output, txns_to_commit, events) =
            self.execute_chunk(first_version, transactions, transaction_infos)?;

        info!(
            LogSchema::new(LogEntry::ChunkExecutor)
                .local_synced_version(first_version.saturating_sub(1))
                .first_version_in_request(first_version_in_request)
                .num_txns_in_request(num_txn),
            "sync_request_executed",
        );
        Ok((output, txns_to_commit, events))
    }

    fn commit_chunk(
        &self,
        verified_target_li: LedgerInfoWithSignatures,
        epoch_change_li: Option<LedgerInfoWithSignatures>,
        output: ProcessedVMOutput,
        txns_to_commit: Vec<TransactionToCommit>,
        events: Vec<ContractEvent>,
    ) -> anyhow::Result<Vec<ContractEvent>> {
        // 4. Commit to DB.
        let first_version = self
            .cache
            .read()
            .synced_trees()
            .txn_accumulator()
            .num_leaves();
        let ledger_info_to_commit =
            Self::find_chunk_li(verified_target_li, epoch_change_li, &output)?;
        if ledger_info_to_commit.is_none() && txns_to_commit.is_empty() {
            return Ok(events);
        }
        fail_point!("executor::commit_chunk", |_| {
            Err(anyhow::anyhow!("Injected error in commit_chunk"))
        });
        self.db.writer.save_transactions(
            &txns_to_commit,
            first_version,
            ledger_info_to_commit.as_ref(),
        )?;

        // 5. Cache maintenance.
        let mut write_lock = self.cache.write();
        let output_trees = output.executed_trees().clone();
        if let Some(ledger_info_with_sigs) = &ledger_info_to_commit {
            write_lock.update_block_tree_root(output_trees, ledger_info_with_sigs.ledger_info());
        } else {
            write_lock.update_synced_trees(output_trees);
        }
        write_lock.reset();

        info!(
            LogSchema::new(LogEntry::ChunkExecutor)
                .synced_to_version(
                    write_lock
                        .synced_trees()
                        .version()
                        .expect("version must exist")
                )
                .committed_with_ledger_info(ledger_info_to_commit.is_some()),
            "sync_request_committed",
        );

        Ok(events)
    }
}
