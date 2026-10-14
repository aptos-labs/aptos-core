// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::{ensure, Result};
use aptos_executor_types::LedgerUpdateOutput;
use aptos_experimental_runtimes::thread_manager::THREAD_MANAGER;
use aptos_types::{
    epoch_state::EpochState, ledger_info::LedgerInfoWithSignatures,
    proof::TransactionInfoListWithProof, transaction::Version,
};

pub struct StateSyncChunkVerifier {
    pub first_version: Version,
    pub txn_infos_with_proof: TransactionInfoListWithProof,
    pub verified_target_li: LedgerInfoWithSignatures,
    pub epoch_change_li: Option<LedgerInfoWithSignatures>,
}

impl StateSyncChunkVerifier {
    pub fn verify_chunk_result(&self, ledger_update_output: &LedgerUpdateOutput) -> Result<()> {
        // In consensus-only mode, we cannot verify the proof against the executed output,
        // because the proof returned by the remote peer is an empty one.
        if cfg!(feature = "consensus-only-perf-test") {
            return Ok(());
        }

        THREAD_MANAGER.get_exe_cpu_pool().install(|| {
            // The chunk can be executed after a partial block carried over from the previous
            // chunk, so the ledger it extends is the parent with that partial block.
            let num_carried_over =
                (self.first_version - ledger_update_output.first_version()) as usize;
            let accumulator = ledger_update_output
                .parent_accumulator
                .append(&ledger_update_output.transaction_info_hashes[..num_carried_over]);

            // Verify the chunk extends the accumulator.
            let num_overlap = self.txn_infos_with_proof.verify_extends_ledger(
                accumulator.num_leaves(),
                accumulator.root_hash(),
                Some(self.first_version),
            )?;
            assert_eq!(num_overlap, 0, "overlapped chunks");

            Ok(())
        })
    }

    pub fn maybe_select_chunk_ending_ledger_info(
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
