// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::OTHER_TIMERS;
use anyhow::{ensure, Result};
use aptos_crypto::HashValue;
use aptos_executor_types::{
    execution_output::ExecutionOutput, state_checkpoint_output::StateCheckpointOutput,
};
use aptos_metrics_core::TimerHelper;
use aptos_scratchpad::ProofRead;
use aptos_storage_interface::state_store::state_summary::{
    LedgerStateSummary, ProvableStateSummary, StateSummary,
};
use itertools::Itertools;

pub struct DoStateCheckpoint;

impl DoStateCheckpoint {
    pub fn run(
        execution_output: &ExecutionOutput,
        parent_state_summary: &LedgerStateSummary,
        persisted_state_summary: &ProvableStateSummary,
        known_state_checkpoints: Option<impl IntoIterator<Item = Option<HashValue>>>,
    ) -> Result<StateCheckpointOutput> {
        let _timer = OTHER_TIMERS.timer_with(&["do_state_checkpoint"]);

        let state_summary = parent_state_summary.update(
            persisted_state_summary,
            execution_output
                .to_commit
                .state_update_refs_for_last_checkpoint(),
            execution_output.to_commit.state_update_refs_for_latest(),
        )?;

        let state_checkpoint_hashes = Self::get_state_checkpoint_hashes(
            execution_output,
            known_state_checkpoints,
            &state_summary,
        )?;

        Ok(StateCheckpointOutput::new(
            state_summary,
            state_checkpoint_hashes,
        ))
    }

    fn get_state_checkpoint_hashes(
        execution_output: &ExecutionOutput,
        known_state_checkpoints: Option<impl IntoIterator<Item = Option<HashValue>>>,
        state_summary: &LedgerStateSummary,
    ) -> Result<Vec<Option<HashValue>>> {
        let num_txns = execution_output.to_commit.len();

        if let Some(known) = known_state_checkpoints {
            let out = known.into_iter().collect_vec();
            ensure!(
                out.len() == num_txns,
                "Bad number of known hashes. {} vs {}",
                out.len(),
                num_txns
            );
            ensure!(
                out.last() == Some(&Some(state_summary.root_hash())),
                "Root hash mismatch. {:?} vs {:?}",
                out.last(),
                state_summary.root_hash()
            );

            Ok(out)
        } else {
            /* FIXME(aldenhu): relex for tests
            // We don't bother to deal with the case where the known hashes are not passed in while
            // there are potentially multiple state checkpoints in the output.
            ensure!(execution_output.is_block || num_txns == 1);
             */

            let mut out = vec![None; num_txns];

            if let Some(updates) = execution_output
                .to_commit
                .state_update_refs_for_last_checkpoint()
            {
                let index = updates.num_versions - 1;
                out[index] = Some(state_summary.last_checkpoint().root_hash());
            }

            Ok(out)
        }
    }
}
