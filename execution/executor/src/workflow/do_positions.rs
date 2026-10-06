// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Per-chunk extension of the native position index.
//!
//! Called from `DoGetExecutionOutput` beside `result_state`, the main-state
//! value it is the counterpart of, so a chunk's overlay lands in
//! `ExecutionOutput` and exists before the next chunk's VM reads positions.

use crate::metrics::OTHER_TIMERS;
use anyhow::Result;
use aptos_executor_types::transactions_with_output::TransactionsToKeep;
use aptos_metrics_core::TimerHelper;
use aptos_storage_interface::state_store::positions::{
    position_key_of, PositionOverlay, PositionParent, PositionWrites,
};
use aptos_types::{state_store::native_position::NativePosition, transaction::Version};

pub struct DoPositions;

impl DoPositions {
    /// Pushes one layer per shard at the chunk's last version, over the
    /// base layers the parent was checked against.
    ///
    /// No last-checkpoint overlay alongside it, unlike main state: the
    /// position root is hashed from the write sets by `DoStateCheckpoint`,
    /// not from the index. Not gated on
    /// `compute_trading_native_state_roots`, which only affects that root.
    pub fn run(
        to_commit: &TransactionsToKeep,
        first_version: Version,
        parent: Option<PositionParent<'_>>,
    ) -> Result<Option<PositionOverlay>> {
        let _timer = OTHER_TIMERS.timer_with(&["do_positions"]);

        let num_txns = to_commit.len();

        // One layer for the chunk; the layer build resolves latest-wins.
        let mut writes = PositionWrites::new();
        for output in &to_commit.transaction_outputs {
            for (key, op) in output.write_set().native_position_iter() {
                let position_key = position_key_of(key).map_err(anyhow::Error::from)?;
                let value = op
                    .as_write_op()
                    .as_state_value_opt()
                    .map(|sv| NativePosition::deserialize(sv.bytes()))
                    .transpose()
                    .map_err(|e| {
                        anyhow::anyhow!("native position value failed to decode at execution: {e}")
                    })?;
                writes.push((position_key, value));
            }
        }

        // No parent means the feature is off, or the caller only verifies and
        // never commits. Fabricating a family would orphan the cold-loaded
        // data; `commit_native_position` rebuilds from the chunk's own writes
        // if a chunk ever reaches it without one.
        let Some(parent) = parent else {
            return Ok(None);
        };

        if num_txns == 0 {
            return Ok(Some(parent.overlay.clone()));
        }

        let last_version = first_version + num_txns as u64 - 1;
        Ok(Some(parent.overlay.extend(
            parent.floor,
            last_version,
            writes,
        )))
    }
}
