// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::counters;
use aptos_infallible::duration_since_epoch;
use std::time::Duration;

pub struct BlockStage;

impl BlockStage {
    pub const COMMITTED: &'static str = "committed";
    pub const COMMIT_CERTIFIED: &'static str = "commit_certified";
    pub const EPOCH_MANAGER_RECEIVED: &'static str = "epoch_manager_received";
    pub const EPOCH_MANAGER_VERIFIED: &'static str = "epoch_manager_verified";
    pub const EXECUTED: &'static str = "executed";
    pub const EXECUTION_PIPELINE_INSERTED: &'static str = "execution_pipeline_inserted";
    pub const NETWORK_RECEIVED: &'static str = "network_received";
    pub const OC_ADDED: &'static str = "ordered_cert_created";
    pub const ORDERED: &'static str = "ordered";
    pub const ORDER_VOTED: &'static str = "order_voted";
    pub const QC_ADDED: &'static str = "qc_added";
    pub const QC_AGGREGATED: &'static str = "qc_aggregated";
    pub const RAND_ADD_DECISION: &'static str = "rand_add_decision";
    pub const RAND_ADD_ENOUGH_SHARE_FAST: &'static str = "rand_add_enough_share_fast";
    pub const RAND_ADD_ENOUGH_SHARE_SLOW: &'static str = "rand_add_enough_share_slow";
    pub const RAND_ENTER: &'static str = "rand_enter";
    pub const RAND_READY: &'static str = "rand_ready";
    pub const ROUND_MANAGER_RECEIVED: &'static str = "round_manager_received";
    pub const SIGNED: &'static str = "signed";
    pub const SYNCED: &'static str = "synced";
    pub const VOTED: &'static str = "voted";
}

/// Record the time during each stage of a block.
pub fn observe_block(timestamp: u64, stage: &'static str) {
    if let Some(t) = duration_since_epoch().checked_sub(Duration::from_micros(timestamp)) {
        counters::BLOCK_TRACING
            .with_label_values(&[stage])
            .observe(t.as_secs_f64());
    }
}
