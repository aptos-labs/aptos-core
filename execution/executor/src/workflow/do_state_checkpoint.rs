// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::metrics::OTHER_TIMERS;
use anyhow::{ensure, Result};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_executor_types::{
    execution_output::ExecutionOutput, state_checkpoint_output::StateCheckpointOutput,
};
use aptos_metrics_core::TimerHelper;
use aptos_storage_interface::state_store::{
    sharded_jmt_state::{PositionSlot, PositionStateWithSummary},
    state_summary::{LedgerStateSummary, ProvablePositionStateSummary, ProvableStateSummary},
    state_with_summary::LedgerWithSummary,
};
use aptos_types::state_store::state_value::StateValue;
use std::collections::HashMap;

pub struct DoStateCheckpoint;

#[bon::bon]
impl DoStateCheckpoint {
    #[builder(finish_fn = build)]
    pub fn run<'a, 'db>(
        execution_output: &'a ExecutionOutput,
        parent_state_summary: &'a LedgerStateSummary,
        persisted_state_summary: &'a ProvableStateSummary<'db>,
        known_state_checkpoints: Option<Vec<Option<HashValue>>>,
        known_hot_state_checkpoints: Option<Vec<Option<HashValue>>>,
        parent_position_state_summary: Option<&'a LedgerWithSummary<PositionStateWithSummary>>,
        persisted_position_state_summary: Option<&'a ProvablePositionStateSummary<'db>>,
        known_position_state_checkpoints: Option<Vec<Option<HashValue>>>,
    ) -> Result<StateCheckpointOutput> {
        let _timer = OTHER_TIMERS.timer_with(&["do_state_checkpoint"]);

        let state_summary = parent_state_summary.update(
            persisted_state_summary,
            &execution_output.hot_state_updates,
            execution_output.to_commit.state_update_refs(),
        )?;

        let last_checkpoint = state_summary.last_checkpoint();

        let state_checkpoint_hashes = Self::get_state_checkpoint_hashes(
            execution_output,
            known_state_checkpoints,
            last_checkpoint.root_hash(),
            "state",
        )?;
        let hot_state_checkpoint_hashes = execution_output
            .transaction_info_v1
            .then(|| {
                Self::get_state_checkpoint_hashes(
                    execution_output,
                    known_hot_state_checkpoints,
                    last_checkpoint.hot_root_hash()?,
                    "hot_state",
                )
            })
            .transpose()?;

        let (position_state_summary, position_state_checkpoint_hashes) =
            if execution_output.compute_trading_native_state_roots {
                let persisted = persisted_position_state_summary
                    .expect("persisted position summary required when feature on");
                let (summary, hashes) = Self::compute_position_checkpoint(
                    execution_output,
                    parent_position_state_summary,
                    persisted,
                    known_position_state_checkpoints,
                )?;
                (Some(summary), Some(hashes))
            } else {
                (None, None)
            };

        Ok(StateCheckpointOutput::builder()
            .state_summary(state_summary)
            .state_checkpoint_hashes(state_checkpoint_hashes)
            .maybe_hot_state_checkpoint_hashes(hot_state_checkpoint_hashes)
            .maybe_position_state_summary(position_state_summary)
            .maybe_position_state_checkpoint_hashes(position_state_checkpoint_hashes)
            .build())
    }

    /// Computes the position summary (latest + last_checkpoint) and per-txn
    /// position root for this chunk by extending the parent on the persisted
    /// base. The root depends only on the position writes, not on the base, so
    /// it's deterministic across nodes.
    fn compute_position_checkpoint(
        execution_output: &ExecutionOutput,
        parent: Option<&LedgerWithSummary<PositionStateWithSummary>>,
        persisted: &ProvablePositionStateSummary,
        known_position_state_checkpoints: Option<Vec<Option<HashValue>>>,
    ) -> Result<(
        LedgerWithSummary<PositionStateWithSummary>,
        Vec<Option<HashValue>>,
    )> {
        let _timer = OTHER_TIMERS.timer_with(&["get_position_checkpoint_hashes"]);

        let num_txns = execution_output.to_commit.len();
        let first_version = execution_output.first_version;
        let last_checkpoint_index = execution_output
            .to_commit
            .state_update_refs()
            .last_inner_checkpoint_index();
        let base_summary = persisted.summary();
        // No in-memory parent at genesis / first block after enabling: seed
        // from the pre-committed position tip (covers committed writes the
        // merklized snapshot may lag).
        let parent_latest =
            parent.map_or_else(|| persisted.base().latest().clone(), |p| p.latest().clone());
        let parent_last_checkpoint = parent.map_or_else(
            || persisted.base().last_checkpoint().clone(),
            |p| p.last_checkpoint().clone(),
        );

        // Empty chunk: nothing to extend (avoids the `num_txns - 1` underflow).
        if num_txns == 0 {
            let summary = LedgerWithSummary::from_latest_and_last_checkpoint(
                parent_latest,
                parent_last_checkpoint,
            );
            return Ok((summary, vec![]));
        }

        // Collapse position writes (latest-per-key) over a version range into
        // SMT leaf updates.
        let collect = |range: std::ops::Range<usize>| -> Vec<(HashValue, PositionSlot)> {
            let mut latest: HashMap<HashValue, PositionSlot> = HashMap::new();
            for i in range {
                for (key, op) in execution_output.to_commit.transaction_outputs[i]
                    .write_set()
                    .native_position_iter()
                {
                    let value_hash = op.as_write_op().as_state_value_opt().map(StateValue::hash);
                    latest.insert(key.hash(), PositionSlot {
                        state_key: key.clone(),
                        value_hash,
                        value: None,
                    });
                }
            }
            latest.into_iter().collect()
        };

        let (new_latest, new_last_checkpoint) = if let Some(ci) = last_checkpoint_index {
            let checkpoint_version = first_version + ci as u64;
            let new_ckpt = parent_latest.extend(
                checkpoint_version,
                collect(0..ci + 1),
                base_summary,
                persisted,
            )?;
            if ci + 1 == num_txns {
                (new_ckpt.clone(), new_ckpt)
            } else {
                let last_version = first_version + num_txns as u64 - 1;
                let new_latest = new_ckpt.extend(
                    last_version,
                    collect(ci + 1..num_txns),
                    base_summary,
                    persisted,
                )?;
                (new_latest, new_ckpt)
            }
        } else {
            // No checkpoint in this chunk: only the latest advances.
            let last_version = first_version + num_txns as u64 - 1;
            let new_latest = parent_latest.extend(
                last_version,
                collect(0..num_txns),
                base_summary,
                persisted,
            )?;
            (new_latest, parent_last_checkpoint)
        };

        // Per-tx hash vector + known-hash validation (shared with main/hot state).
        let hashes = Self::get_state_checkpoint_hashes(
            execution_output,
            known_position_state_checkpoints,
            new_last_checkpoint.root_hash(),
            "position_state",
        )?;

        let summary =
            LedgerWithSummary::from_latest_and_last_checkpoint(new_latest, new_last_checkpoint);
        Ok((summary, hashes))
    }

    fn get_state_checkpoint_hashes(
        execution_output: &ExecutionOutput,
        known_state_checkpoints: Option<Vec<Option<HashValue>>>,
        computed_last_checkpoint_hash: HashValue,
        label: &str,
    ) -> Result<Vec<Option<HashValue>>> {
        let _timer = OTHER_TIMERS.timer_with(&[&format!("get_{label}_checkpoint_hashes")]);

        let num_txns = execution_output.to_commit.len();
        let last_checkpoint_index = execution_output
            .to_commit
            .state_update_refs()
            .last_inner_checkpoint_index();

        if let Some(known) = known_state_checkpoints {
            ensure!(
                known.len() == num_txns,
                "Bad number of known {label} hashes. {} vs {}",
                known.len(),
                num_txns,
            );
            if let Some(idx) = last_checkpoint_index {
                ensure!(
                    known[idx] == Some(computed_last_checkpoint_hash),
                    "{label} root hash mismatch with known hashes passed in. {:?} vs {:?}",
                    known[idx],
                    Some(computed_last_checkpoint_hash),
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
                out[index] = Some(computed_last_checkpoint_hash);
            }
            Ok(out)
        }
    }
}
