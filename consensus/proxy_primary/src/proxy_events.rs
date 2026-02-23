// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Event types for communication between primary and proxy consensus.
//!
//! These are used by the primary RoundManager and proxy RoundManager to
//! exchange QC/TC updates and ordered proxy blocks.

use aptos_consensus_types::{
    common::Round,
    proxy_messages::OrderedProxyBlocksMsg,
    quorum_cert::QuorumCert,
    timeout_2chain::TwoChainTimeoutCertificate,
};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

/// Pipeline backpressure state from the primary execution pipeline.
///
/// This is NOT consensus-affecting data — it is used locally by the proxy
/// payload client to throttle throughput. Different validators may see
/// slightly different values, but the proxy payload client uses it only
/// for budget decisions that don't affect block content (just whether to
/// return empty payloads or apply delays, which is already the case with
/// the block-counting budget today).
#[derive(Debug, Clone, Default)]
pub struct PipelineBackpressureInfo {
    /// Gap between ordered_round and commit_round on the primary.
    /// Large values mean the execution pipeline is falling behind.
    pub pipeline_pending_round_gap: u64,
    /// Number of pending proxy block batches in the primary's
    /// `pending_proxy_blocks` vec (unconsumed by proposals).
    pub pending_proxy_batches: u64,
    /// The primary round that was most recently committed.
    pub primary_committed_round: Round,
    /// The primary round most recently ordered.
    pub primary_ordered_round: Round,
    /// Timestamp (epoch ms) when this info was generated.
    pub timestamp_ms: u64,
}

/// Atomic pipeline state shared between the proxy event loop and
/// the proxy budget payload client. Lock-free using atomics.
pub struct AtomicPipelineState {
    pipeline_pending_round_gap: AtomicU64,
    pending_proxy_batches: AtomicU64,
    primary_committed_round: AtomicU64,
    primary_ordered_round: AtomicU64,
    last_update_ms: AtomicU64,
}

impl AtomicPipelineState {
    pub fn new() -> Self {
        Self {
            pipeline_pending_round_gap: AtomicU64::new(0),
            pending_proxy_batches: AtomicU64::new(0),
            primary_committed_round: AtomicU64::new(0),
            primary_ordered_round: AtomicU64::new(0),
            last_update_ms: AtomicU64::new(0),
        }
    }

    pub fn store(&self, info: &PipelineBackpressureInfo) {
        self.pipeline_pending_round_gap
            .store(info.pipeline_pending_round_gap, Ordering::Release);
        self.pending_proxy_batches
            .store(info.pending_proxy_batches, Ordering::Release);
        self.primary_committed_round
            .store(info.primary_committed_round, Ordering::Release);
        self.primary_ordered_round
            .store(info.primary_ordered_round, Ordering::Release);
        self.last_update_ms
            .store(info.timestamp_ms, Ordering::Release);
    }

    pub fn load(&self) -> PipelineBackpressureInfo {
        PipelineBackpressureInfo {
            pipeline_pending_round_gap: self.pipeline_pending_round_gap.load(Ordering::Acquire),
            pending_proxy_batches: self.pending_proxy_batches.load(Ordering::Acquire),
            primary_committed_round: self.primary_committed_round.load(Ordering::Acquire),
            primary_ordered_round: self.primary_ordered_round.load(Ordering::Acquire),
            timestamp_ms: self.last_update_ms.load(Ordering::Acquire),
        }
    }
}

/// Events sent from primary RoundManager to proxy RoundManager.
#[derive(Debug)]
pub enum PrimaryToProxyEvent {
    /// New primary QC available - may trigger proxy block "cutting"
    NewPrimaryQC(Arc<QuorumCert>),
    /// New primary TC available - for tracking primary round
    NewPrimaryTC(Arc<TwoChainTimeoutCertificate>),
    /// Periodic pipeline state for backpressure decisions
    PipelineState(PipelineBackpressureInfo),
    /// Shutdown signal
    Shutdown,
}

/// Events sent from proxy RoundManager to primary RoundManager.
#[derive(Debug)]
pub enum ProxyToPrimaryEvent {
    /// Ordered proxy blocks ready to be aggregated into primary block
    OrderedProxyBlocks(OrderedProxyBlocksMsg),
}
