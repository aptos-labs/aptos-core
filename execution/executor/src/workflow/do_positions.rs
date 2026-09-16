// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Per-block extension of the native position overlay.
//!
//! Runs in the execution phase, not at state checkpoint, so a block's
//! overlay exists before its child's VM reads positions — possible only
//! because extending it reads nothing.

use crate::metrics::OTHER_TIMERS;
use anyhow::Result;
use aptos_executor_types::execution_output::ExecutionOutput;
use aptos_metrics_core::TimerHelper;
use aptos_storage_interface::state_store::{
    positions::{position_key_of, PositionOverlay, PositionWrites},
    state_with_summary::LedgerWithSummary,
};
use aptos_types::state_store::native_position::NativePosition;

pub struct DoPositions;

impl DoPositions {
    /// Pushes one layer per shard at the block's last version.
    ///
    /// Not gated on `compute_trading_native_state_roots`, unlike the
    /// JMT-side summary — that flag only affects the consensus root.
    /// `None` means the feature is off; writes are empty then, and
    /// non-empty writes without a parent is a wiring bug, so it errors.
    pub fn run(
        execution_output: &ExecutionOutput,
        parent: Option<&LedgerWithSummary<PositionOverlay>>,
    ) -> Result<Option<LedgerWithSummary<PositionOverlay>>> {
        let _timer = OTHER_TIMERS.timer_with(&["do_positions"]);

        let num_txns = execution_output.to_commit.len();
        let first_version = execution_output.first_version;

        // Per transaction so the block can be split at its state checkpoint.
        // The layer build resolves latest-wins, so only arrival order matters.
        let mut per_txn: Vec<PositionWrites> = Vec::with_capacity(num_txns);
        let mut total_writes = 0usize;
        for output in &execution_output.to_commit.transaction_outputs {
            let mut writes = PositionWrites::new();
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
            total_writes += writes.len();
            per_txn.push(writes);
        }

        // A fabricated family would be unrelated to the bundle's chain, and
        // publishing it on commit would orphan the cold-loaded data.
        let Some(parent) = parent else {
            if total_writes > 0 {
                anyhow::bail!(
                    "DoPositions: parent is None but block has {total_writes} position writes; \
                     the executor should always be given a parent when ENABLE_TRADING_NATIVE \
                     is on",
                );
            }
            return Ok(None);
        };

        let parent_latest = parent.latest().clone();
        let parent_last_checkpoint = parent.last_checkpoint().clone();

        if num_txns == 0 {
            return Ok(Some(LedgerWithSummary::from_latest_and_last_checkpoint(
                parent_latest,
                parent_last_checkpoint,
            )));
        }

        let collect = |range: std::ops::Range<usize>| -> PositionWrites {
            per_txn[range].iter().flatten().cloned().collect()
        };

        // Split at the checkpoint, as the JMT side and main state do, or
        // post-checkpoint writes fold into it.
        let last_version = first_version + num_txns as u64 - 1;
        let (new_latest, new_last_checkpoint) = match execution_output
            .to_commit
            .state_update_refs()
            .last_inner_checkpoint_index()
        {
            Some(ci) if ci + 1 == num_txns => {
                let new_ckpt = parent_latest.extend(last_version, collect(0..num_txns));
                (new_ckpt.clone(), new_ckpt)
            },
            Some(ci) => {
                let new_ckpt = parent_latest.extend(first_version + ci as u64, collect(0..ci + 1));
                let new_latest = new_ckpt.extend(last_version, collect(ci + 1..num_txns));
                (new_latest, new_ckpt)
            },
            None => (
                parent_latest.extend(last_version, collect(0..num_txns)),
                parent_last_checkpoint,
            ),
        };

        Ok(Some(LedgerWithSummary::from_latest_and_last_checkpoint(
            new_latest,
            new_last_checkpoint,
        )))
    }
}
