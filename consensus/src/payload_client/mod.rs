// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{block_storage::BlockReader, error::QuorumStoreError};
use aptos_consensus_types::{common::Payload, payload_pull_params::PayloadPullParameters};
use aptos_proxy_primary::proxy_metrics;
use aptos_types::validator_txn::ValidatorTransaction;
use aptos_validator_transaction_pool::TransactionFilter;
use std::sync::{
    atomic::{AtomicBool, Ordering},
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
}

impl ProxyBudgetPayloadClient {
    pub fn new(
        inner: Arc<dyn PayloadClient>,
        proxy_block_store: Arc<dyn BlockReader>,
        target: u64,
        quorum_store_enabled: bool,
        round_timeout_ms: u64,
        has_pending_proof: Arc<AtomicBool>,
    ) -> Self {
        Self {
            inner,
            proxy_block_store,
            target,
            quorum_store_enabled,
            round_timeout_ms,
            has_pending_proof,
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
        config: PayloadPullParameters,
        validator_txn_filter: TransactionFilter,
    ) -> anyhow::Result<(Vec<ValidatorTransaction>, Payload), QuorumStoreError> {
        let (blocks_with_txns, total_blocks) = self.count_blocks_since_cutting_point();
        proxy_metrics::PROXY_TXN_BUDGET_REMAINING
            .set(self.target.saturating_sub(blocks_with_txns) as i64);

        // Apply constant backpressure delay (half round timeout) when total
        // blocks exceed target. Skip delay if a primary proof is pending —
        // cutting-point blocks must be ordered ASAP to unblock the primary.
        if total_blocks > self.target
            && !self.has_pending_proof.load(Ordering::Acquire)
        {
            let delay_ms = self.round_timeout_ms / 2;
            proxy_metrics::PROXY_BACKPRESSURE_DELAY_MS.set(delay_ms as i64);
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        } else {
            proxy_metrics::PROXY_BACKPRESSURE_DELAY_MS.set(0);
        }

        if blocks_with_txns >= self.target {
            // Budget exhausted: return empty so proxy keeps running for ordering
            return Ok((vec![], Payload::empty(self.quorum_store_enabled, true)));
        }
        self.inner.pull_payload(config, validator_txn_filter).await
    }
}
