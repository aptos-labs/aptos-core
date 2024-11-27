// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_crypto::HashValue;
use aptos_executor_types::{
    execution_output::ExecutionOutput, state_checkpoint_output::StateCheckpointOutput,
};
use aptos_storage_interface::state_store::state_summary::StateSummary;

pub struct DoStateCheckpoint;

impl DoStateCheckpoint {
    pub fn run(
        _execution_output: &ExecutionOutput,
        _parent_state_summary: StateSummary,
        _known_state_checkpoints: Option<impl IntoIterator<Item = Option<HashValue>>>,
    ) -> Result<StateCheckpointOutput> {
        /* FIXME(aldenhu):
        // Apply the write set, get the latest state.
        InMemoryStateCalculatorV2::calculate_for_transactions(
            execution_output,
            known_state_checkpoints,
        )
         */
        todo!()
    }
}
