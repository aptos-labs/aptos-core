// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::components::in_memory_state_calculator_v2::InMemoryStateCalculatorV2;
use anyhow::Result;
use aptos_crypto::HashValue;
use aptos_executor_types::{
    chunk_output::ChunkOutput, state_checkpoint_output::StateCheckpointOutput,
};
use aptos_storage_interface::state_delta::StateDelta;

pub struct MakeStateCheckpoint;

impl MakeStateCheckpoint {
    pub fn make(
        chunk_output: &ChunkOutput,
        parent_state: &StateDelta,
        known_state_checkpoints: Option<Vec<Option<HashValue>>>,
        is_block: bool,
    ) -> Result<StateCheckpointOutput> {
        // Apply the write set, get the latest state.
        let mut res = InMemoryStateCalculatorV2::calculate_for_transactions(
            parent_state,
            chunk_output,
            is_block,
        )?;

        // On state sync/replay, we generate state checkpoints only periodically, for the
        // last state checkpoint of each chunk.
        // A mismatch in the SMT will be detected at that occasion too. Here we just copy
        // in the state root from the TxnInfo in the proof.
        if let Some(state_checkpoint_hashes) = known_state_checkpoints {
            res.check_and_update_state_checkpoint_hashes(state_checkpoint_hashes)?;
        }

        Ok(res)
    }
}
