// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::{Author, PayloadFilter},
    utils::PayloadTxnsSize,
};
use std::{collections::HashSet, time::Duration};

#[derive(Clone)]
pub struct OptQSPayloadPullParams {
    pub exclude_authors: HashSet<Author>,
    pub minimum_batch_age_usecs: u64,
}

pub struct PayloadPullParameters {
    pub max_poll_time: Duration,
    pub max_txns: PayloadTxnsSize,
    pub max_txns_after_filtering: u64,
    pub soft_max_txns_after_filtering: u64,
    pub max_inline_txns: PayloadTxnsSize,
    pub user_txn_filter: PayloadFilter,
    pub pending_ordering: bool,
    pub pending_uncommitted_blocks: usize,
    pub recent_max_fill_fraction: f32,
    pub block_timestamp: Duration,
    pub maybe_optqs_payload_pull_params: Option<OptQSPayloadPullParams>,
}

impl std::fmt::Debug for OptQSPayloadPullParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OptQSPayloadPullParams")
            .field("exclude_authors", &self.exclude_authors)
            .field("minimum_batch_age_useds", &self.minimum_batch_age_usecs)
            .finish()
    }
}

impl PayloadPullParameters {
    pub fn new_for_test(
        max_poll_time: Duration,
        max_txns: u64,
        max_txns_bytes: u64,
        max_txns_after_filtering: u64,
        soft_max_txns_after_filtering: u64,
        max_inline_txns: u64,
        max_inline_txns_bytes: u64,
        user_txn_filter: PayloadFilter,
        pending_ordering: bool,
        pending_uncommitted_blocks: usize,
        recent_max_fill_fraction: f32,
        block_timestamp: Duration,
    ) -> Self {
        Self {
            max_poll_time,
            max_txns: PayloadTxnsSize::new(max_txns, max_txns_bytes),
            max_txns_after_filtering,
            soft_max_txns_after_filtering,
            max_inline_txns: PayloadTxnsSize::new(max_inline_txns, max_inline_txns_bytes),
            user_txn_filter,
            pending_ordering,
            pending_uncommitted_blocks,
            recent_max_fill_fraction,
            block_timestamp,
            maybe_optqs_payload_pull_params: None,
        }
    }
}

impl std::fmt::Debug for PayloadPullParameters {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PayloadPullParameters")
            .field("max_poll_time", &self.max_poll_time)
            .field("max_items", &self.max_txns)
            .field("max_unique_items", &self.max_txns_after_filtering)
            .field(
                "soft_max_txns_after_filtering",
                &self.soft_max_txns_after_filtering,
            )
            .field("max_inline_items", &self.max_inline_txns)
            .field("pending_ordering", &self.pending_ordering)
            .field(
                "pending_uncommitted_blocks",
                &self.pending_uncommitted_blocks,
            )
            .field("recent_max_fill_fraction", &self.recent_max_fill_fraction)
            .field("block_timestamp", &self.block_timestamp)
            .field("optqs_params", &self.maybe_optqs_payload_pull_params)
            .finish()
    }
}
