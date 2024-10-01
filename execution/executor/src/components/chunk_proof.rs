// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{ensure, Result};
use aptos_crypto::HashValue;
use aptos_executor_types::{state_checkpoint_output::StateCheckpointOutput, LedgerUpdateOutput};
use aptos_types::{
    epoch_state::EpochState,
    ledger_info::LedgerInfoWithSignatures,
    proof::{accumulator::InMemoryTransactionAccumulator, TransactionInfoListWithProof},
    transaction::Version,
};
use itertools::zip_eq;

pub struct ChunkProof {
    pub txn_infos_with_proof: TransactionInfoListWithProof,
    pub verified_target_li: LedgerInfoWithSignatures,
    pub epoch_change_li: Option<LedgerInfoWithSignatures>,
}

impl ChunkProof {
    pub fn verify_chunk(
        &self,
        parent_accumulator: &InMemoryTransactionAccumulator,
        ledger_update_output: &LedgerUpdateOutput,
    ) -> Result<()> {
        // In consensus-only mode, we cannot verify the proof against the executed output,
        // because the proof returned by the remote peer is an empty one.
        if cfg!(feature = "consensus-only-perf-test") {
            return Ok(());
        }

        // Verify the chunk extends the parent accumulator.
        let first_version = parent_accumulator.num_leaves();
        let parent_root_hash = parent_accumulator.root_hash();
        let num_overlap = self.txn_infos_with_proof.verify_extends_ledger(
            first_version,
            parent_root_hash,
            Some(first_version),
        )?;
        assert_eq!(num_overlap, 0, "overlapped chunks");

        // Verify transaction infos match
        let mut version = first_version;
        for (txn_info, expected_txn_info) in zip_eq(
            &ledger_update_output.transaction_infos,
            &self.txn_infos_with_proof.transaction_infos,
        ) {
            ensure!(
                txn_info == expected_txn_info,
                "Transaction infos don't match. version:{version}, txn_info:{txn_info}, expected_txn_info:{expected_txn_info}",
            );
            version += 1;
        }

        Ok(())
    }

    pub fn maybe_select_chunk_ending_ledger_info(
        &self,
        ledger_update_output: &LedgerUpdateOutput,
        next_epoch_state: Option<&EpochState>,
    ) -> Result<Option<LedgerInfoWithSignatures>> {
        let verified_li = self.verified_target_li.ledger_info();
        let txn_accumulator = ledger_update_output.transaction_accumulator();

        if verified_li.version() + 1 == txn_accumulator.num_leaves() {
            // If the chunk corresponds to the target LI, the target LI can be added to storage.
            ensure!(
                verified_li.transaction_accumulator_hash() == txn_accumulator.root_hash(),
                "Root hash in target ledger info does not match local computation. {:?} != {:?}",
                verified_li,
                txn_accumulator,
            );
            Ok(Some(self.verified_target_li.clone()))
        } else if let Some(epoch_change_li) = &self.epoch_change_li {
            // If the epoch change LI is present, it must match the version of the chunk:
            let epoch_change_li = epoch_change_li.ledger_info();

            // Verify that the given ledger info corresponds to the new accumulator.
            ensure!(
                epoch_change_li.transaction_accumulator_hash() == txn_accumulator.root_hash(),
                "Root hash of a given epoch LI does not match local computation. {:?} vs {:?}",
                epoch_change_li,
                txn_accumulator,
            );
            ensure!(
                epoch_change_li.version() + 1 == txn_accumulator.num_leaves(),
                "Version of a given epoch LI does not match local computation. {:?} vs {:?}",
                epoch_change_li,
                txn_accumulator,
            );
            ensure!(
                epoch_change_li.ends_epoch(),
                "Epoch change LI does not carry validator set. version:{}",
                epoch_change_li.version(),
            );
            ensure!(
                epoch_change_li.next_epoch_state() == next_epoch_state,
                "New validator set of a given epoch LI does not match local computation. {:?} vs {:?}",
                epoch_change_li.ledger_info().next_epoch_state(),
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
