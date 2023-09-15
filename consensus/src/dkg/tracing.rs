// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::counters;
use aptos_infallible::duration_since_epoch;
use std::time::Duration;

pub struct DKGStage;

impl DKGStage {
    pub const DKG_NODE_READY: &'static str = "dkg_node_ready";
    pub const DKG_AGG_NODE_READY: &'static str = "dkg_agg_node_ready";
    pub const DKG_AGG_NODE_PROPOSED: &'static str = "dkg_agg_node_proposed";
    pub const DKG_FINISH: &'static str = "dkg_finish";
}

/// Record the time during each stage of the DKG process.
pub fn observe_dkg(timestamp: u64, stage: &'static str) {
    if let Some(t) = duration_since_epoch().checked_sub(Duration::from_micros(timestamp)) {
        counters::DKG_TRACING
            .with_label_values(&[stage])
            .observe(t.as_secs_f64());
    }
}
