// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::types::in_memory_state_calculator_v2::InMemoryStateCalculatorV2;
use anyhow::Result;
use aptos_crypto::HashValue;
use aptos_executor_types::{
    execution_output::ExecutionOutput, state_checkpoint_output::StateCheckpointOutput,
};
use aptos_storage_interface::state_delta::StateDelta;
use std::sync::Arc;

pub struct DoStateCheckpoint;

impl DoStateCheckpoint {
    pub fn run(
        execution_output: &ExecutionOutput,
        parent_state: &Arc<StateDelta>,
        known_state_checkpoints: Option<impl IntoIterator<Item = Option<HashValue>>>,
    ) -> Result<StateCheckpointOutput> {
        // Apply the write set, get the latest state.
        InMemoryStateCalculatorV2::calculate_for_transactions(
            execution_output,
            parent_state,
            known_state_checkpoints,
        )
    }
}
