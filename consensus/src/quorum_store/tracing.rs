// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::quorum_store::counters;
use aptos_consensus_types::common::Author;
use aptos_infallible::duration_since_epoch;
use aptos_short_hex_str::AsShortHexStr;
use std::time::Duration;

pub struct BatchStage;

impl BatchStage {
    pub const POS_FORMED: &'static str = "pos";
    pub const RECEIVED: &'static str = "received";
    pub const SIGNED: &'static str = "signed";
}

/// Record the time during each stage of a batch.
pub fn observe_batch(timestamp: u64, author: Author, stage: &'static str) {
    if let Some(t) = duration_since_epoch().checked_sub(Duration::from_micros(timestamp)) {
        counters::BATCH_TRACING
            .with_label_values(&[author.short_str().as_str(), stage])
            .observe(t.as_secs_f64());
    }
}

pub fn observe_batch_vote_pct(timestamp: u64, author: Author, pct: u8) {
    if let Some(t) = duration_since_epoch().checked_sub(Duration::from_micros(timestamp)) {
        let pct = (pct / 10) * 10;
        counters::BATCH_VOTE_PROGRESS
            .with_label_values(&[author.short_str().as_str(), &pct.to_string()])
            .observe(t.as_secs_f64());
    }
}
