// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Per-chunk extension of the native order index, beside `DoPositions`.

use crate::metrics::OTHER_TIMERS;
use anyhow::Result;
use aptos_executor_types::transactions_with_output::TransactionsToKeep;
use aptos_metrics_core::TimerHelper;
use aptos_storage_interface::state_store::orders::{
    decode_order_writes, OrderOverlay, OrderParent,
};
use aptos_types::transaction::Version;

pub struct DoOrders;

impl DoOrders {
    /// Pushes one layer per shard at the chunk's last version, over the base
    /// layers the parent was checked against.
    pub fn run(
        to_commit: &TransactionsToKeep,
        first_version: Version,
        parent: Option<OrderParent<'_>>,
    ) -> Result<Option<OrderOverlay>> {
        let _timer = OTHER_TIMERS.timer_with(&["do_orders"]);

        // One layer for the chunk; the layer build resolves latest-wins.
        let writes =
            decode_order_writes(to_commit.transaction_outputs.iter().flat_map(|output| {
                output
                    .write_set()
                    .native_order_iter()
                    .map(|(key, op)| (key, op.as_write_op().as_state_value_opt()))
            }))?;

        // No parent means storage is off, or the caller only verifies and
        // never commits.
        let Some(parent) = parent else {
            return Ok(None);
        };

        if to_commit.is_empty() {
            return Ok(Some(parent.overlay.clone()));
        }

        let last_version = first_version + to_commit.len() as u64 - 1;
        Ok(Some(parent.overlay.extend(
            parent.floor,
            last_version,
            writes,
        )))
    }
}
