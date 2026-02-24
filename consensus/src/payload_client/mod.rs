// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{block_storage::BlockReader, error::QuorumStoreError};
use aptos_config::config::ProxyBackpressureConfig;
use aptos_logger::info;
use aptos_consensus_types::{
    common::Payload, payload_pull_params::PayloadPullParameters, utils::PayloadTxnsSize,
};
use aptos_proxy_primary::{proxy_metrics, AtomicPipelineState};
use aptos_types::validator_txn::ValidatorTransaction;
use aptos_validator_transaction_pool::TransactionFilter;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};
use std::time::Duration;

pub mod mixed;
pub mod user;
pub mod validator;

#[async_trait::async_trait]
pub trait PayloadClient: Send + Sync {
    async fn pull_payload(
        &self,
        config: PayloadPullParameters,
        validator_txn_filter: TransactionFilter,
    ) -> anyhow::Result<(Vec<ValidatorTransaction>, Payload), QuorumStoreError>;
}

/// Budget-aware payload client for proxy consensus.
///
/// Wraps a real payload client (e.g. MixedPayloadClient) and enforces a
/// system-wide budget on proxy blocks carrying transactions per primary round.
///
/// Instead of a per-validator counter, the budget is determined by walking the
/// proxy block chain backwards from the highest QC block to the last cutting
/// point (a block with `primary_proof`). This count is the same across all
/// validators because they share the same certified chain view.
///
/// After `target` blocks with non-empty payloads in the current batch, returns
/// empty payloads so proxy consensus keeps running for ordering without adding
/// more transactions.
pub struct ProxyBudgetPayloadClient {
    inner: Arc<dyn PayloadClient>,
    /// Proxy BlockStore for walking the chain to count blocks with txns.
    proxy_block_store: Arc<dyn BlockReader>,
    /// Target number of proxy blocks with transactions per primary round.
    target: u64,
    quorum_store_enabled: bool,
    /// Proxy round timeout in ms; backpressure delay = round_timeout_ms / 2.
    round_timeout_ms: u64,
    /// Shared flag: true when a primary proof is pending in ProxyHooksImpl.
    /// Skip backpressure delay when true — cutting-point blocks should be
    /// ordered ASAP to unblock the primary pipeline.
    has_pending_proof: Arc<AtomicBool>,
    /// Shared pipeline state from primary, updated atomically by the proxy event loop.
    /// Used for adaptive backpressure decisions based on primary's actual congestion.
    pipeline_state: Arc<AtomicPipelineState>,
    /// Backpressure tuning parameters.
    bp_config: ProxyBackpressureConfig,
    /// Round counter for periodic logging (not consensus round, just invocation count).
    pull_count: AtomicU64,
}

impl ProxyBudgetPayloadClient {
    pub fn new(
        inner: Arc<dyn PayloadClient>,
        proxy_block_store: Arc<dyn BlockReader>,
        target: u64,
        quorum_store_enabled: bool,
        round_timeout_ms: u64,
        has_pending_proof: Arc<AtomicBool>,
        pipeline_state: Arc<AtomicPipelineState>,
        bp_config: ProxyBackpressureConfig,
    ) -> Self {
        Self {
            inner,
            proxy_block_store,
            target,
            quorum_store_enabled,
            round_timeout_ms,
            has_pending_proof,
            pipeline_state,
            bp_config,
            pull_count: AtomicU64::new(0),
        }
    }

    /// Count proxy blocks since the last cutting point.
    ///
    /// Walks backwards from the highest QC block (the parent of the block being
    /// proposed) through the proxy chain. Stops when it finds a block with
    /// `primary_proof` (a cutting point marking the end of the previous batch)
    /// or runs out of blocks in the store.
    ///
    /// Returns (blocks_with_txns, total_blocks).
    fn count_blocks_since_cutting_point(&self) -> (u64, u64) {
        let hqc = self.proxy_block_store.highest_quorum_cert();
        let mut block_id = hqc.certified_block().id();
        let mut with_txns = 0u64;
        let mut total = 0u64;

        loop {
            let block = match self.proxy_block_store.get_block(block_id) {
                Some(b) => b,
                None => break, // reached pruned history or genesis parent
            };

            // Cutting point: this block has primary_proof, belongs to previous batch
            if block.block().block_data().primary_proof().is_some() {
                break;
            }

            total += 1;
            // Count blocks with non-empty user transaction payload
            if block.payload().map_or(false, |p| !p.is_empty()) {
                with_txns += 1;
            }

            block_id = block.parent_id();
        }

        (with_txns, total)
    }
}

#[async_trait::async_trait]
impl PayloadClient for ProxyBudgetPayloadClient {
    async fn pull_payload(
        &self,
        mut config: PayloadPullParameters,
        validator_txn_filter: TransactionFilter,
    ) -> anyhow::Result<(Vec<ValidatorTransaction>, Payload), QuorumStoreError> {
        let pull_num = self.pull_count.fetch_add(1, Ordering::Relaxed);
        let (blocks_with_txns, total_blocks) = self.count_blocks_since_cutting_point();
        let pipeline_info = self.pipeline_state.load();
        let has_pending = self.has_pending_proof.load(Ordering::Acquire);
        let gap = pipeline_info.pipeline_pending_round_gap;

        proxy_metrics::PROXY_PIPELINE_PENDING_GAP.set(gap as i64);
        proxy_metrics::PROXY_PENDING_BATCHES_AT_PRIMARY
            .set(pipeline_info.pending_proxy_batches as i64);
        proxy_metrics::PROXY_BLOCKS_WITH_TXNS.set(blocks_with_txns as i64);
        proxy_metrics::PROXY_TOTAL_BLOCKS_IN_BATCH.set(total_blocks as i64);
        let batches = pipeline_info.pending_proxy_batches;

        let bp = &self.bp_config;

        // --- Adaptive budget: reduce target when primary pipeline is congested ---
        let effective_target = if gap > bp.pipeline_heavy_gap {
            // Heavy congestion: minimize budget
            1
        } else if gap > bp.pipeline_moderate_gap {
            // Moderate congestion: reduce by 50%
            (self.target / 2).max(1)
        } else {
            self.target
        };

        proxy_metrics::PROXY_TXN_BUDGET_REMAINING
            .set(effective_target.saturating_sub(blocks_with_txns) as i64);
        proxy_metrics::PROXY_EFFECTIVE_BUDGET_TARGET.set(effective_target as i64);

        // --- Adaptive delay: proportional to congestion level ---
        //
        // Two categories of delay:
        // 1. Batch delay: ALWAYS applied regardless of has_pending, because this is
        //    the primary signal that proxy is overwhelming primary. Humio logs showed
        //    has_pending=true ~68% of the time, completely defeating backpressure when
        //    all delays were gated on !has_pending.
        // 2. Budget/gap delays: only when !has_pending — cutting-point blocks should
        //    be ordered quickly, but we still slow down based on batch accumulation.
        //
        // Batch delay uses 5x round_timeout ceiling (500ms at 100ms round_timeout)
        // with max_pending_batches_for_delay=50, giving a smooth ramp:
        //   batches=2: 20ms, batches=5: 50ms, batches=10: 100ms,
        //   batches=25: 250ms, batches=50+: 500ms
        let delay_ms = {
            let mut delay = 0u64;

            // Pending batches delay: ALWAYS applied (even when has_pending=true).
            // This is the dominant backpressure signal. Uses 5x round_timeout ceiling
            // so this alone can reach 500ms under heavy batch accumulation.
            if batches >= bp.pending_batches_delay_threshold {
                let batch_delay = self
                    .round_timeout_ms
                    .saturating_mul(5)
                    .saturating_mul(batches.min(bp.max_pending_batches_for_delay))
                    / bp.max_pending_batches_for_delay;
                delay += batch_delay;
            }

            // Budget and gap delays: skip when has_pending to let cutting-point
            // blocks order quickly.
            if !has_pending {
                // Budget-based delay: half round timeout when blocks > target
                if total_blocks > effective_target {
                    delay += self.round_timeout_ms / 2;
                }

                // Pipeline gap delay: proportional, kicks in at gap > 2
                if gap > 2 {
                    let gap_delay = self
                        .round_timeout_ms
                        .saturating_mul(gap.min(bp.max_pipeline_gap_for_delay))
                        / bp.max_pipeline_gap_for_delay;
                    delay += gap_delay;
                }
            }

            delay
        };

        proxy_metrics::PROXY_BACKPRESSURE_DELAY_MS.set(delay_ms as i64);
        if delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        }

        // --- Budget check: return empty if exhausted ---
        if blocks_with_txns >= effective_target {
            proxy_metrics::PROXY_EMPTY_PAYLOAD_BUDGET.inc();
            // Log every 50th pull to avoid flooding
            if pull_num % 50 == 0 {
                info!(
                    pull_num = pull_num,
                    decision = "EMPTY_BUDGET",
                    blocks_with_txns = blocks_with_txns,
                    total_blocks = total_blocks,
                    effective_target = effective_target,
                    base_target = self.target,
                    gap = gap,
                    batches = batches,
                    has_pending = has_pending,
                    delay_ms = delay_ms,
                    "[ProxyBudget] returning empty payload (budget exhausted)"
                );
            }
            return Ok((vec![], Payload::empty(self.quorum_store_enabled, true)));
        }

        // --- Adaptive max_txns: reduce per-block size under congestion ---
        let original_max_count = config.max_txns.count();
        if gap > bp.pipeline_heavy_gap {
            // Heavy congestion: halve max_txns per block
            let reduced = PayloadTxnsSize::new(
                config.max_txns.count() / 2,
                config.max_txns.size_in_bytes() / 2,
            );
            config.max_txns = config.max_txns.minimum(reduced);
            config.max_txns_after_filtering /= 2;
            config.soft_max_txns_after_filtering /= 2;
        } else if gap > bp.pipeline_moderate_gap {
            // Moderate congestion: reduce max_txns by 25%
            let reduced = PayloadTxnsSize::new(
                config.max_txns.count() * 3 / 4,
                config.max_txns.size_in_bytes() * 3 / 4,
            );
            config.max_txns = config.max_txns.minimum(reduced);
            config.max_txns_after_filtering = config.max_txns_after_filtering * 3 / 4;
            config.soft_max_txns_after_filtering = config.soft_max_txns_after_filtering * 3 / 4;
        }

        proxy_metrics::PROXY_NONEMPTY_PAYLOAD_PULLED.inc();
        // Log every 50th pull or first 10 pulls for debugging
        if pull_num % 50 == 0 || pull_num < 10 {
            info!(
                pull_num = pull_num,
                decision = "PULLING",
                blocks_with_txns = blocks_with_txns,
                total_blocks = total_blocks,
                effective_target = effective_target,
                base_target = self.target,
                gap = gap,
                batches = batches,
                has_pending = has_pending,
                delay_ms = delay_ms,
                max_txns = config.max_txns.count(),
                original_max_txns = original_max_count,
                "[ProxyBudget] pulling payload from inner client"
            );
        }

        self.inner.pull_payload(config, validator_txn_filter).await
    }
}
