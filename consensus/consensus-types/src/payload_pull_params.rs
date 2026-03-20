// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    common::{Author, PayloadFilter},
    proof_of_store::BatchKind,
    utils::PayloadTxnsSize,
};
use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

/// Per-block transaction count limits, keyed by BatchKind.
/// Batches whose kind is absent from the map have no additional limit
/// beyond the global max_txns.
#[derive(Clone, Debug, Default)]
pub struct PerBatchKindTxnLimits {
    limits: HashMap<BatchKind, u64>,
}

impl PerBatchKindTxnLimits {
    pub fn new(limits: HashMap<BatchKind, u64>) -> Self {
        Self { limits }
    }

    pub fn get(&self, kind: &BatchKind) -> Option<u64> {
        self.limits.get(kind).copied()
    }

    pub fn is_empty(&self) -> bool {
        self.limits.is_empty()
    }

    /// Returns a new `PerBatchKindTxnLimits` with each limit reduced by the
    /// corresponding count already consumed.
    pub fn remaining(&self, consumed: &HashMap<BatchKind, u64>) -> Self {
        let limits = self
            .limits
            .iter()
            .map(|(kind, limit)| {
                let used = consumed.get(kind).copied().unwrap_or(0);
                (*kind, limit.saturating_sub(used))
            })
            .collect();
        Self { limits }
    }
}

#[derive(Clone)]
pub struct OptQSPayloadPullParams {
    pub exclude_authors: HashSet<Author>,
    pub minimum_batch_age_usecs: u64,
    pub enable_opt_qs_v2_payload: bool,
    pub per_kind_txn_limits: PerBatchKindTxnLimits,
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
            .field("enable_opt_qs_v2_payload", &self.enable_opt_qs_v2_payload)
            .field("per_kind_txn_limits", &self.per_kind_txn_limits)
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
