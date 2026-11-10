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
    sharded_jmt_state::{NativeSlot, NativeStateWithSummary},
    state_summary::{
        LedgerStateSummary, ProvableCollateralStateSummary, ProvableNativeStateSummary,
        ProvableOrderStateSummary, ProvablePositionStateSummary, ProvableStateSummary,
    },
    state_with_summary::LedgerWithSummary,
};
use aptos_types::{
    state_store::{state_key::StateKey, state_value::StateValue},
    write_set::{WriteOp, WriteSet},
};
use std::collections::HashMap;

pub struct DoStateCheckpoint;

/// A trading-native tree whose root `TransactionInfoV1` commits.
#[derive(Clone, Copy)]
enum NativeTree {
    Position,
    Order,
    Collateral,
}

impl NativeTree {
    fn label(self) -> &'static str {
        match self {
            NativeTree::Position => "position_state",
            NativeTree::Order => "order_state",
            NativeTree::Collateral => "collateral_state",
        }
    }

    fn for_each_write(self, write_set: &WriteSet, mut f: impl FnMut(&StateKey, &WriteOp)) {
        match self {
            NativeTree::Position => write_set
                .native_position_iter()
                .for_each(|(key, op)| f(key, op.as_write_op())),
            NativeTree::Order => write_set
                .native_order_iter()
                .for_each(|(key, op)| f(key, op.as_write_op())),
            NativeTree::Collateral => write_set
                .native_collateral_iter()
                .for_each(|(key, op)| f(key, op.as_write_op())),
        }
    }
}

#[bon::bon]
impl DoStateCheckpoint {
    #[builder(finish_fn = build)]
    pub fn run<'a, 'db>(
        execution_output: &'a ExecutionOutput,
        parent_state_summary: &'a LedgerStateSummary,
        persisted_state_summary: &'a ProvableStateSummary<'db>,
        known_state_checkpoints: Option<Vec<Option<HashValue>>>,
        known_hot_state_checkpoints: Option<Vec<Option<HashValue>>>,
        parent_position_state_summary: Option<&'a LedgerWithSummary<NativeStateWithSummary>>,
        persisted_position_state_summary: Option<&'a ProvablePositionStateSummary<'db>>,
        known_position_state_checkpoints: Option<Vec<Option<HashValue>>>,
        parent_order_state_summary: Option<&'a LedgerWithSummary<NativeStateWithSummary>>,
        persisted_order_state_summary: Option<&'a ProvableOrderStateSummary<'db>>,
        known_order_state_checkpoints: Option<Vec<Option<HashValue>>>,
        parent_collateral_state_summary: Option<&'a LedgerWithSummary<NativeStateWithSummary>>,
        persisted_collateral_state_summary: Option<&'a ProvableCollateralStateSummary<'db>>,
        known_collateral_state_checkpoints: Option<Vec<Option<HashValue>>>,
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
            .hot_state_root_in_txn_info
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
                let (summary, hashes) = Self::compute_native_checkpoint(
                    NativeTree::Position,
                    execution_output,
                    parent_position_state_summary,
                    persisted,
                    known_position_state_checkpoints,
                )?;
                (Some(summary), Some(hashes))
            } else {
                (None, None)
            };
        let (order_state_summary, order_state_checkpoint_hashes) =
            if execution_output.compute_trading_native_state_roots {
                let persisted = persisted_order_state_summary
                    .expect("persisted order summary required when feature on");
                let (summary, hashes) = Self::compute_native_checkpoint(
                    NativeTree::Order,
                    execution_output,
                    parent_order_state_summary,
                    persisted,
                    known_order_state_checkpoints,
                )?;
                (Some(summary), Some(hashes))
            } else {
                (None, None)
            };
        let (collateral_state_summary, collateral_state_checkpoint_hashes) =
            if execution_output.compute_trading_native_state_roots {
                let persisted = persisted_collateral_state_summary
                    .expect("persisted collateral summary required when feature on");
                let (summary, hashes) = Self::compute_native_checkpoint(
                    NativeTree::Collateral,
                    execution_output,
                    parent_collateral_state_summary,
                    persisted,
                    known_collateral_state_checkpoints,
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
            .maybe_order_state_summary(order_state_summary)
            .maybe_order_state_checkpoint_hashes(order_state_checkpoint_hashes)
            .maybe_collateral_state_summary(collateral_state_summary)
            .maybe_collateral_state_checkpoint_hashes(collateral_state_checkpoint_hashes)
            .build())
    }

    /// Computes `tree`'s summary and per-txn root
    /// for this chunk by extending the parent on the persisted base. The root
    /// depends only on the tree's writes, not on the base, so it's
    /// deterministic across nodes.
    fn compute_native_checkpoint(
        tree: NativeTree,
        execution_output: &ExecutionOutput,
        parent: Option<&LedgerWithSummary<NativeStateWithSummary>>,
        persisted: &ProvableNativeStateSummary,
        known_checkpoints: Option<Vec<Option<HashValue>>>,
    ) -> Result<(
        LedgerWithSummary<NativeStateWithSummary>,
        Vec<Option<HashValue>>,
    )> {
        let _timer = OTHER_TIMERS.timer_with(&[&format!("compute_{}_checkpoint", tree.label())]);

        let num_txns = execution_output.to_commit.len();
        let first_version = execution_output.first_version;
        // No in-memory parent at genesis / first block after enabling: seed
        // from the pre-committed tip (covers committed writes the merklized
        // snapshot may lag).
        let parent = parent.map_or_else(
            || persisted.base().last_checkpoint().clone(),
            |p| p.last_checkpoint().clone(),
        );

        // Empty chunk: nothing to extend (avoids the `num_txns - 1` underflow).
        if num_txns == 0 {
            return Ok((LedgerWithSummary::new_at_checkpoint(parent), vec![]));
        }

        // Collapse the tree's writes (latest-per-key) over a version range into
        // SMT leaf updates.
        let collect = |range: std::ops::Range<usize>| -> Vec<(HashValue, NativeSlot)> {
            let mut latest: HashMap<HashValue, NativeSlot> = HashMap::new();
            for i in range {
                tree.for_each_write(
                    execution_output.to_commit.transaction_outputs[i].write_set(),
                    |key, op| {
                        let value_hash = op.as_state_value_opt().map(StateValue::hash);
                        latest.insert(key.hash(), NativeSlot {
                            state_key: key.clone(),
                            value_hash,
                            value: None,
                        });
                    },
                );
            }
            latest.into_iter().collect()
        };

        // The chunk ends at a checkpoint.
        let last_version = first_version + num_txns as u64 - 1;
        let new_ckpt = parent.extend(
            last_version,
            collect(0..num_txns),
            persisted.persisted(),
            persisted,
        )?;

        // Per-tx hash vector + known-hash validation (shared with main/hot state).
        let hashes = Self::get_state_checkpoint_hashes(
            execution_output,
            known_checkpoints,
            new_ckpt.root_hash(),
            tree.label(),
        )?;

        Ok((LedgerWithSummary::new_at_checkpoint(new_ckpt), hashes))
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
