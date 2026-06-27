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
    user_positions::{materialize_user_position_updates, PositionWrite, UserPositionKey, UserPositions},
};
use aptos_types::state_store::{
    native_position::NativePosition,
    state_key::inner::{StateKeyInner, TradingNativeKey},
    state_value::StateValue,
};
use std::collections::HashMap;

pub struct DoStateCheckpoint;

impl DoStateCheckpoint {
    pub fn run(
        execution_output: &ExecutionOutput,
        parent_state_summary: &LedgerStateSummary,
        persisted_state_summary: &ProvableStateSummary,
        known_state_checkpoints: Option<Vec<Option<HashValue>>>,
        known_hot_state_checkpoints: Option<Vec<Option<HashValue>>>,
        parent_position_state_summary: Option<&LedgerWithSummary<PositionStateWithSummary>>,
        persisted_position_state_summary: Option<&ProvablePositionStateSummary>,
        known_position_state_checkpoints: Option<Vec<Option<HashValue>>>,
        parent_user_positions: Option<&LedgerWithSummary<UserPositions>>,
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

        // Compute the per-account `UserPositions` index alongside the JMT
        // summary. Required for validator-side reads to see prior-block
        // writes — the bundle's reader handle is published from this on
        // commit. `parent_user_positions` is None only at genesis or on
        // restart before the executor seeds from durable storage; both
        // produce an empty starting chain.
        let user_positions = Self::compute_user_positions_checkpoint(
            execution_output,
            parent_user_positions,
        )?;

        Ok(StateCheckpointOutput::new(
            state_summary,
            state_checkpoint_hashes,
            hot_state_checkpoint_hashes,
            position_state_summary,
            position_state_checkpoint_hashes,
            user_positions,
        ))
    }

    /// Per-block extension of the `UserPositions` layered index.
    /// Decodes each tx's Position writes, materializes per-account
    /// deltas against `parent`, and pushes one layer at the block's
    /// last version. Returns the latest+last_checkpoint pair so the
    /// next block's executor can chain off it.
    ///
    /// **Not gated on `compute_trading_native_state_roots`** unlike
    /// `compute_position_checkpoint` — the consensus-state-root flag
    /// only affects the JMT-side computation. The user-positions
    /// index advances whenever the executor has a parent threaded
    /// from the bundle (i.e., `ENABLE_TRADING_NATIVE` is on at the
    /// Rust level). When that's not the case, `parent` is `None`,
    /// writes are empty, and this returns `None`.
    ///
    /// Returns `None` when there's no parent AND no writes. Errors
    /// if writes are non-empty but parent is `None` — that means the
    /// executor was reached without going through the bundle, which
    /// indicates a wiring bug.
    fn compute_user_positions_checkpoint(
        execution_output: &ExecutionOutput,
        parent: Option<&LedgerWithSummary<UserPositions>>,
    ) -> Result<Option<LedgerWithSummary<UserPositions>>> {
        let _timer = OTHER_TIMERS.timer_with(&["compute_user_positions_checkpoint"]);

        let num_txns = execution_output.to_commit.len();
        let first_version = execution_output.first_version;

        // Collect the chunk's typed PositionWrites in arrival order
        // (latest-wins per (account, market) handled inside
        // materialize_user_position_updates).
        let mut writes: Vec<PositionWrite> = Vec::new();
        for output in &execution_output.to_commit.transaction_outputs {
            for (key, op) in output.write_set().native_position_iter() {
                let (exchange, account, market) = match key.inner() {
                    StateKeyInner::TradingNative(TradingNativeKey::Position {
                        exchange,
                        account,
                        market,
                    }) => (*exchange, *account, *market),
                    other => {
                        anyhow::bail!(
                            "non-Position native StateKey in chunk write set: {other:?}"
                        );
                    },
                };
                let value = op
                    .as_write_op()
                    .as_state_value_opt()
                    .map(|sv| NativePosition::deserialize(sv.bytes()))
                    .transpose()
                    .map_err(|e| {
                        anyhow::anyhow!(
                            "native position value failed to decode at execution: {e}"
                        )
                    })?;
                writes.push(PositionWrite {
                    position_key: UserPositionKey { exchange, account },
                    market,
                    value,
                });
            }
        }

        // Refuse to fabricate a UserPositions family when there's no
        // parent: `MapLayer::new_family` would produce a fresh family
        // with a random ID, unrelated to the bundle's chain. Publishing
        // that to `bundle.user_positions` later would orphan the
        // cold-loaded data. Under correct wiring the executor always
        // gets a parent (LedgerSummary populates from the bundle), so
        // None here means the feature is off OR there's a wiring bug.
        let Some(parent) = parent else {
            if !writes.is_empty() {
                anyhow::bail!(
                    "compute_user_positions_checkpoint: parent is None but chunk has \
                     {} position writes; executor wiring should always provide a parent \
                     when ENABLE_TRADING_NATIVE is on",
                    writes.len(),
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

        let last_version = first_version + num_txns as u64 - 1;
        let updates = if writes.is_empty() {
            Vec::new()
        } else {
            materialize_user_position_updates(&parent_latest, writes)
        };
        let new_latest = parent_latest.extend(last_version, updates);
        // last_checkpoint advances with latest only when the chunk
        // crossed a state-checkpoint boundary; otherwise it stays at
        // the parent's last_checkpoint.
        let new_last_checkpoint = if execution_output
            .to_commit
            .state_update_refs()
            .last_inner_checkpoint_index()
            .is_some()
        {
            new_latest.clone()
        } else {
            parent_last_checkpoint
        };

        Ok(Some(LedgerWithSummary::from_latest_and_last_checkpoint(
            new_latest,
            new_last_checkpoint,
        )))
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
