// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::logging::{LogEntry, LogSchema};
use anyhow::{ensure, format_err, Result};
use diem_logger::prelude::*;
use diem_types::{
    contract_event::ContractEvent,
    ledger_info::LedgerInfoWithSignatures,
    protocol_spec::DpnProto,
    transaction::{
        default_protocol::{TransactionListWithProof, TransactionOutputListWithProof},
        Transaction, TransactionInfo, TransactionOutput, Version,
    },
};
use diem_vm::VMExecutor;
use executor_types::{ChunkExecutor, ProcessedVMOutput};
use fail::fail_point;

use crate::{
    metrics::{
        DIEM_EXECUTOR_APPLY_CHUNK_SECONDS, DIEM_EXECUTOR_COMMIT_CHUNK_SECONDS,
        DIEM_EXECUTOR_EXECUTE_CHUNK_SECONDS,
    },
    Executor,
};
use diem_types::transaction::TransactionToCommit;

impl<V: VMExecutor> ChunkExecutor for Executor<DpnProto, V> {
    fn execute_chunk(
        &self,
        txn_list_with_proof: TransactionListWithProof,
        // Target LI that has been verified independently: the proofs are relative to this version.
        verified_target_li: LedgerInfoWithSignatures,
    ) -> Result<(
        ProcessedVMOutput,
        Vec<TransactionToCommit>,
        Vec<ContractEvent>,
    )> {
        let _timer = DIEM_EXECUTOR_EXECUTE_CHUNK_SECONDS.start_timer();

        let num_txn = txn_list_with_proof.transactions.len();
        let first_version_in_request = txn_list_with_proof.first_transaction_version;

        // 1. Update the cache in executor to be consistent with latest synced state.
        self.reset_cache()?;
        // 2. Verify input transaction list.
        txn_list_with_proof.verify(
            verified_target_li.ledger_info(),
            txn_list_with_proof.first_transaction_version,
        )?;

        let (transactions, txn_outputs, transaction_infos) = self.filter_chunk(
            txn_list_with_proof.transactions,
            None,
            txn_list_with_proof.first_transaction_version,
            txn_list_with_proof.proof,
        )?;

        // 3. Execute transactions.
        let first_version = self
            .cache
            .read()
            .synced_trees()
            .txn_accumulator()
            .num_leaves();
        let res = self.execute_or_apply_chunk(
            first_version,
            transactions,
            txn_outputs,
            transaction_infos,
        )?;

        info!(
            LogSchema::new(LogEntry::ChunkExecutor)
                .local_synced_version(first_version.saturating_sub(1))
                .first_version_in_request(first_version_in_request)
                .num_txns_in_request(num_txn),
            "sync_request_executed",
        );

        Ok(res)
    }

    fn apply_chunk(
        &self,
        txn_output_list_with_proof: TransactionOutputListWithProof,
        // Target LI that has been verified independently: the proofs are relative to this version.
        verified_target_li: LedgerInfoWithSignatures,
    ) -> anyhow::Result<(
        ProcessedVMOutput,
        Vec<TransactionToCommit>,
        Vec<ContractEvent>,
    )> {
        let _timer = DIEM_EXECUTOR_APPLY_CHUNK_SECONDS.start_timer();
        // 1. Update the cache in executor to be consistent with latest synced state.
        self.reset_cache()?;

        let num_txn = txn_output_list_with_proof.transactions_and_outputs.len();
        let first_version_in_request = txn_output_list_with_proof.first_transaction_output_version;

        // 2. Verify input transaction list.
        txn_output_list_with_proof.verify(
            verified_target_li.ledger_info(),
            txn_output_list_with_proof.first_transaction_output_version,
        )?;

        let (unfiltered_transactions, unfiltered_txn_outputs): (Vec<_>, Vec<_>) =
            txn_output_list_with_proof
                .transactions_and_outputs
                .into_iter()
                .unzip();

        let (transactions, txn_outputs, transaction_infos) = self.filter_chunk(
            unfiltered_transactions,
            Some(unfiltered_txn_outputs),
            txn_output_list_with_proof.first_transaction_output_version,
            txn_output_list_with_proof.proof,
        )?;

        // 3. Execute transactions.
        let first_version = self
            .cache
            .read()
            .synced_trees()
            .txn_accumulator()
            .num_leaves();

        let res = self.execute_or_apply_chunk(
            first_version,
            transactions,
            txn_outputs,
            transaction_infos,
        )?;

        info!(
            LogSchema::new(LogEntry::ChunkExecutor)
                .local_synced_version(first_version.saturating_sub(1))
                .first_version_in_request(first_version_in_request)
                .num_txns_in_request(num_txn),
            "sync_request_executed",
        );

        Ok(res)
    }

    fn commit_chunk(
        &self,
        verified_target_li: LedgerInfoWithSignatures,
        epoch_change_li: Option<LedgerInfoWithSignatures>,
        output: ProcessedVMOutput,
        txns_to_commit: Vec<TransactionToCommit>,
        events: Vec<ContractEvent>,
    ) -> Result<Vec<ContractEvent>> {
        let _timer = DIEM_EXECUTOR_COMMIT_CHUNK_SECONDS.start_timer();

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

    fn execute_or_apply_chunk(
        &self,
        first_version: Version,
        transactions: Vec<Transaction>,
        transaction_outputs: Option<Vec<TransactionOutput>>,
        transaction_infos: Vec<TransactionInfo>,
    ) -> Result<(
        ProcessedVMOutput,
        Vec<TransactionToCommit>,
        Vec<ContractEvent>,
    )> {
        let num_txns = transactions.len();
        let (processed_vm_output, txns_to_commit, events, txns_to_retry, _txn_infos_to_retry) =
            self.replay_transactions_impl(
                first_version,
                transactions,
                transaction_outputs,
                transaction_infos,
            )?;

        ensure!(
            txns_to_retry.is_empty(),
            "The transaction at version {} got the status of 'Retry'",
            num_txns
                .checked_sub(txns_to_retry.len())
                .ok_or_else(|| format_err!("integer overflow occurred"))?
                .checked_add(first_version as usize)
                .ok_or_else(|| format_err!("integer overflow occurred"))?,
        );

        Ok((processed_vm_output, txns_to_commit, events))
    }
}
