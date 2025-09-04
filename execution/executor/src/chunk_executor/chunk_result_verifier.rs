// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{ensure, Result};
use velor_executor_types::LedgerUpdateOutput;
use velor_experimental_runtimes::thread_manager::THREAD_MANAGER;
use velor_types::{
    epoch_state::EpochState,
    ledger_info::LedgerInfoWithSignatures,
    proof::{accumulator::InMemoryTransactionAccumulator, TransactionInfoListWithProof},
    transaction::TransactionInfo,
};

pub trait ChunkResultVerifier {
    fn verify_chunk_result(
        &self,
        parent_accumulator: &InMemoryTransactionAccumulator,
        ledger_update_output: &LedgerUpdateOutput,
    ) -> Result<()>;

    fn transaction_infos(&self) -> &[TransactionInfo];

    fn maybe_select_chunk_ending_ledger_info(
        &self,
        ledger_update_output: &LedgerUpdateOutput,
        next_epoch_state: Option<&EpochState>,
    ) -> Result<Option<LedgerInfoWithSignatures>>;
}

pub struct StateSyncChunkVerifier {
    pub txn_infos_with_proof: TransactionInfoListWithProof,
    pub verified_target_li: LedgerInfoWithSignatures,
    pub epoch_change_li: Option<LedgerInfoWithSignatures>,
}

impl ChunkResultVerifier for StateSyncChunkVerifier {
    fn verify_chunk_result(
        &self,
        parent_accumulator: &InMemoryTransactionAccumulator,
        ledger_update_output: &LedgerUpdateOutput,
    ) -> Result<()> {
        // In consensus-only mode, we cannot verify the proof against the executed output,
        // because the proof returned by the remote peer is an empty one.
        if cfg!(feature = "consensus-only-perf-test") {
            return Ok(());
        }

        THREAD_MANAGER.get_exe_cpu_pool().install(|| {
            let first_version = parent_accumulator.num_leaves();

            // Verify the chunk extends the parent accumulator.
            let parent_root_hash = parent_accumulator.root_hash();
            let num_overlap = self.txn_infos_with_proof.verify_extends_ledger(
                first_version,
                parent_root_hash,
                Some(first_version),
            )?;
            assert_eq!(num_overlap, 0, "overlapped chunks");

            // Verify transaction infos match
            ledger_update_output
                .ensure_transaction_infos_match(&self.txn_infos_with_proof.transaction_infos)?;

            Ok(())
        })
    }

    fn transaction_infos(&self) -> &[TransactionInfo] {
        &self.txn_infos_with_proof.transaction_infos
    }

    fn maybe_select_chunk_ending_ledger_info(
        &self,
        ledger_update_output: &LedgerUpdateOutput,
        next_epoch_state: Option<&EpochState>,
    ) -> Result<Option<LedgerInfoWithSignatures>> {
        let li = self.verified_target_li.ledger_info();
        let txn_accumulator = &ledger_update_output.transaction_accumulator;

        if li.version() + 1 == txn_accumulator.num_leaves() {
            // If the chunk corresponds to the target LI, the target LI can be added to storage.
            ensure!(
                li.transaction_accumulator_hash() == txn_accumulator.root_hash(),
                "Root hash in target ledger info does not match local computation. {:?} != {:?}",
                li,
                txn_accumulator,
            );
            Ok(Some(self.verified_target_li.clone()))
        } else if let Some(epoch_change_li) = &self.epoch_change_li {
            // If the epoch change LI is present, it must match the version of the chunk:
            let li = epoch_change_li.ledger_info();

            // Verify that the given ledger info corresponds to the new accumulator.
            ensure!(
                li.transaction_accumulator_hash() == txn_accumulator.root_hash(),
                "Root hash of a given epoch LI does not match local computation. {:?} vs {:?}",
                li,
                txn_accumulator,
            );
            ensure!(
                li.version() + 1 == txn_accumulator.num_leaves(),
                "Version of a given epoch LI does not match local computation. {:?} vs {:?}",
                li,
                txn_accumulator,
            );
            ensure!(
                li.ends_epoch(),
                "Epoch change LI does not carry validator set. version:{}",
                li.version(),
            );
            ensure!(
                li.next_epoch_state() == next_epoch_state,
                "New validator set of a given epoch LI does not match local computation. {:?} vs {:?}",
                li.next_epoch_state(),
                next_epoch_state,
            );
            Ok(Some(epoch_change_li.clone()))
        } else {
            ensure!(
                next_epoch_state.is_none(),
                "End of epoch chunk based on local computation but no EoE LedgerInfo provided. version: {:?}",
                txn_accumulator.num_leaves().checked_sub(1),
            );
            Ok(None)
        }
    }
}

pub struct ReplayChunkVerifier {
    pub transaction_infos: Vec<TransactionInfo>,
}

impl ChunkResultVerifier for ReplayChunkVerifier {
    fn verify_chunk_result(
        &self,
        _parent_accumulator: &InMemoryTransactionAccumulator,
        ledger_update_output: &LedgerUpdateOutput,
    ) -> Result<()> {
        ledger_update_output.ensure_transaction_infos_match(&self.transaction_infos)
    }

    fn transaction_infos(&self) -> &[TransactionInfo] {
        &self.transaction_infos
    }

    fn maybe_select_chunk_ending_ledger_info(
        &self,
        _ledger_update_output: &LedgerUpdateOutput,
        _next_epoch_state: Option<&EpochState>,
    ) -> Result<Option<LedgerInfoWithSignatures>> {
        Ok(None)
    }
}
