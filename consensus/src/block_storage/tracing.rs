// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::counters;
use aptos_infallible::duration_since_epoch;
use std::time::Duration;

pub struct BlockStage;

impl BlockStage {
    pub const SIGNED: &'static str = "signed";
    pub const NETWORK_RECEIVED: &'static str = "network_received";
    pub const EPOCH_MANAGER_RECEIVED: &'static str = "epoch_manager_received";
    pub const EPOCH_MANAGER_VERIFIED: &'static str = "epoch_manager_verified";
    pub const ROUND_MANAGER_RECEIVED: &'static str = "round_manager_received";
    pub const SYNCED: &'static str = "synced";
    pub const VOTED: &'static str = "voted";
    pub const QC_AGGREGATED: &'static str = "qc_aggregated";
    pub const QC_ADDED: &'static str = "qc_added";
    pub const ORDERED: &'static str = "ordered";
    pub const EXECUTED: &'static str = "executed";
    pub const COMMIT_CERTIFIED: &'static str = "commit_certified";
    pub const COMMITTED: &'static str = "committed";
}

/// Record the time during each stage of a block.
pub fn observe_block(timestamp: u64, stage: &'static str) {
    if let Some(t) = duration_since_epoch().checked_sub(Duration::from_micros(timestamp)) {
        counters::BLOCK_TRACING
            .with_label_values(&[stage])
            .observe(t.as_secs_f64());
    }
}
