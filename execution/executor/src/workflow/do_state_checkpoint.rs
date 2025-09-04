// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::OTHER_TIMERS;
use anyhow::{ensure, Result};
use velor_crypto::HashValue;
use velor_executor_types::{
    execution_output::ExecutionOutput, state_checkpoint_output::StateCheckpointOutput,
};
use velor_metrics_core::TimerHelper;
use velor_storage_interface::state_store::state_summary::{
    LedgerStateSummary, ProvableStateSummary,
};

pub struct DoStateCheckpoint;

impl DoStateCheckpoint {
    pub fn run(
        execution_output: &ExecutionOutput,
        parent_state_summary: &LedgerStateSummary,
        persisted_state_summary: &ProvableStateSummary,
        known_state_checkpoints: Option<Vec<Option<HashValue>>>,
    ) -> Result<StateCheckpointOutput> {
        let _timer = OTHER_TIMERS.timer_with(&["do_state_checkpoint"]);

        let state_summary = parent_state_summary.update(
            persisted_state_summary,
            execution_output.to_commit.state_update_refs(),
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
        known_state_checkpoints: Option<Vec<Option<HashValue>>>,
        state_summary: &LedgerStateSummary,
    ) -> Result<Vec<Option<HashValue>>> {
        let _timer = OTHER_TIMERS.timer_with(&["get_state_checkpoint_hashes"]);

        let num_txns = execution_output.to_commit.len();
        let last_checkpoint_index = execution_output
            .to_commit
            .state_update_refs()
            .last_inner_checkpoint_index();

        if let Some(known) = known_state_checkpoints {
            ensure!(
                known.len() == num_txns,
                "Bad number of known hashes. {} vs {}",
                known.len(),
                num_txns
            );
            if let Some(idx) = last_checkpoint_index {
                ensure!(
                    known[idx] == Some(state_summary.last_checkpoint().root_hash()),
                    "Root hash mismatch with known hashes passed in. {:?} vs {:?}",
                    known[idx],
                    Some(&state_summary.last_checkpoint().root_hash()),
                );
            }

            Ok(known)
        } else {
            if !execution_output.is_block {
                // We should enter this branch only in test.
                execution_output.to_commit.ensure_at_most_one_checkpoint()?;
            }

            let mut out = vec![None; num_txns];

            if let Some(index) = last_checkpoint_index {
                out[index] = Some(state_summary.last_checkpoint().root_hash());
            }

            Ok(out)
        }
    }
}
