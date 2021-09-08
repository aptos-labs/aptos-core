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
use executor_types::ChunkExecutor;
use fail::fail_point;

use crate::{metrics::DIEM_EXECUTOR_EXECUTE_AND_COMMIT_CHUNK_SECONDS, Executor};

impl<V: VMExecutor> ChunkExecutor for Executor<DpnProto, V> {
    fn execute_and_commit_chunk(
        &self,
        txn_list_with_proof: TransactionListWithProof,
        // Target LI that has been verified independently: the proofs are relative to this version.
        verified_target_li: LedgerInfoWithSignatures,
        // An optional end of epoch LedgerInfo. We do not allow chunks that end epoch without
        // carrying any epoch change LI.
        epoch_change_li: Option<LedgerInfoWithSignatures>,
    ) -> anyhow::Result<Vec<ContractEvent>> {
        let _timer = DIEM_EXECUTOR_EXECUTE_AND_COMMIT_CHUNK_SECONDS.start_timer();
        // 1. Update the cache in executor to be consistent with latest synced state.
        self.reset_cache()?;
        let read_lock = self.cache.read();

        info!(
            LogSchema::new(LogEntry::ChunkExecutor)
                .local_synced_version(read_lock.synced_trees().txn_accumulator().num_leaves() - 1)
                .first_version_in_request(txn_list_with_proof.first_transaction_version)
                .num_txns_in_request(txn_list_with_proof.transactions.len()),
            "sync_request_received",
        );

        // 2. Verify input transaction list.
        let (transactions, transaction_infos) =
            self.verify_chunk(txn_list_with_proof, &verified_target_li)?;

        // 3. Execute transactions.
        let first_version = read_lock.synced_trees().txn_accumulator().num_leaves();
        drop(read_lock);
        let (output, txns_to_commit, events) =
            self.execute_chunk(first_version, transactions, transaction_infos)?;

        // 4. Commit to DB.
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
            "sync_finished",
        );

        Ok(events)
    }
}
