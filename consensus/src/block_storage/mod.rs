// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use velor_consensus_types::{
    pipelined_block::{ExecutionSummary, PipelinedBlock},
    quorum_cert::QuorumCert,
    sync_info::SyncInfo,
    timeout_2chain::TwoChainTimeoutCertificate,
    wrapped_ledger_info::WrappedLedgerInfo,
};
use velor_crypto::HashValue;
pub use block_store::{
    sync_manager::{BlockRetriever, NeedFetchResult},
    BlockStore,
};
use std::{sync::Arc, time::Duration};

mod block_store;
mod block_tree;
mod execution_pool;
pub mod pending_blocks;
pub mod tracing;

pub trait BlockReader: Send + Sync {
    /// Check if a block with the block_id exist in the BlockTree.
    fn block_exists(&self, block_id: HashValue) -> bool;

    /// Try to get a block with the block_id, return an Arc of it if found.
    fn get_block(&self, block_id: HashValue) -> Option<Arc<PipelinedBlock>>;

    /// Get the current ordered root block of the BlockTree.
    fn ordered_root(&self) -> Arc<PipelinedBlock>;

    /// Get the current commit root block of the BlockTree.
    fn commit_root(&self) -> Arc<PipelinedBlock>;

    fn get_quorum_cert_for_block(&self, block_id: HashValue) -> Option<Arc<QuorumCert>>;

    /// Returns all the blocks between the ordered/commit root and the given block, including the given block
    /// but excluding the root.
    /// In case a given block is not the successor of the root, return None.
    /// For example if a tree is b0 <- b1 <- b2 <- b3, then
    /// path_from_root(b2) -> Some([b2, b1])
    /// path_from_root(b0) -> Some([])
    /// path_from_root(a) -> None
    fn path_from_ordered_root(&self, block_id: HashValue) -> Option<Vec<Arc<PipelinedBlock>>>;

    fn path_from_commit_root(&self, block_id: HashValue) -> Option<Vec<Arc<PipelinedBlock>>>;

    /// Return the certified block with the highest round.
    #[cfg(test)]
    fn highest_certified_block(&self) -> Arc<PipelinedBlock>;

    /// Return the quorum certificate with the highest round
    fn highest_quorum_cert(&self) -> Arc<QuorumCert>;

    /// Return the wrapped ledger info that carries ledger info with the highest round
    fn highest_ordered_cert(&self) -> Arc<WrappedLedgerInfo>;

    /// Return the highest timeout certificate if available.
    fn highest_2chain_timeout_cert(&self) -> Option<Arc<TwoChainTimeoutCertificate>>;

    /// Return the highest commit decision wrapped ledger info.
    fn highest_commit_cert(&self) -> Arc<WrappedLedgerInfo>;

    /// Return the combination of highest quorum cert, timeout cert and commit cert.
    fn sync_info(&self) -> SyncInfo;

    /// Return if the consensus is backpressured
    fn vote_back_pressure(&self) -> bool;

    // Return time difference between last committed block and new proposal
    fn pipeline_pending_latency(&self, proposal_timestamp: Duration) -> Duration;

    fn get_recent_block_execution_times(&self, num_blocks: usize) -> Vec<ExecutionSummary>;
}
