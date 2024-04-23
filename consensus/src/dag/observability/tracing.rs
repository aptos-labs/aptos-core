// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::dag::observability::counters;
use aptos_infallible::duration_since_epoch;
use aptos_metrics_core::HistogramVec;
use std::time::Duration;

#[derive(strum_macros::AsRefStr)]
pub enum NodeStage {
    NodeFirstReceived,
    NodeReceived,
    CertAggregated,
    CertifiedNodeReceived,
    AnchorOrdered,
    NodeOrdered,
}

#[derive(strum_macros::AsRefStr)]
pub enum RoundStage {
    NodeBroadcastedQuorum,
    NodeBroadcastedAll,
    CertifiedNodeBroadcasted,
    StrongLinkReceived,
    VotingPowerMet,
    Finished,
}

fn observe(counter: &HistogramVec, timestamp: u64, name: &str, dag_id: u8) {
    if let Some(t) = duration_since_epoch().checked_sub(Duration::from_micros(timestamp)) {
        counter
            .with_label_values(&[&dag_id.to_string(), name])
            .observe(t.as_secs_f64());
    }
}

/// Record the time during each stage of a node.
pub fn observe_node(dag_id: u8, timestamp: u64, stage: NodeStage) {
    observe(&counters::NODE_TRACING, timestamp, stage.as_ref(), dag_id);
}

/// Record the time during each stage of a round.
pub fn observe_round(dag_id: u8, timestamp: u64, stage: RoundStage) {
    observe(&counters::ROUND_TRACING, timestamp, stage.as_ref(), dag_id);
}
