// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_bitvec::BitVec;
use aptos_config::config::BlockTransactionFilterConfig;
use aptos_consensus_types::{
    block::Block,
    common::{Author, Payload},
};
use aptos_executor_types::*;
use aptos_types::transaction::SignedTransaction;
use async_trait::async_trait;

mod co_payload_manager;
mod direct_mempool_payload_manager;
mod quorum_store_payload_manager;

pub use co_payload_manager::ConsensusObserverPayloadManager;
pub use direct_mempool_payload_manager::DirectMempoolPayloadManager;
#[cfg(test)]
pub use quorum_store_payload_manager::TQuorumStoreCommitNotifier;
pub use quorum_store_payload_manager::{QuorumStoreCommitNotifier, QuorumStorePayloadManager};

/// A trait that defines the interface for a payload manager. The payload manager is responsible for
/// resolving the transactions in a block's payload.
#[async_trait]
pub trait TPayloadManager: Send + Sync {
    /// Notify the payload manager that a block has been committed. This indicates that the
    /// transactions in the block's payload are no longer required for consensus.
    fn notify_commit(&self, block_timestamp: u64, payloads: Vec<Payload>);

    /// Prefetch the data for a payload. This is used to ensure that the data for a payload is
    /// available when block is executed.
    fn prefetch_payload_data(&self, payload: &Payload, author: Author, timestamp: u64);

    /// Check if the block contains any inline transactions that need
    /// to be denied (e.g., due to block transaction filtering).
    /// This is only used when processing block proposals.
    fn check_denied_inline_transactions(
        &self,
        block: &Block,
        block_txn_filter_config: &BlockTransactionFilterConfig,
    ) -> anyhow::Result<()>;

    /// Check if the transactions corresponding are available. This is specific to payload
    /// manager implementations. For optimistic quorum store, we only check if optimistic
    /// batches are available locally.
    fn check_payload_availability(&self, block: &Block) -> Result<(), BitVec>;

    /// Get the transactions in a block's payload. This function returns a vector of transactions.
    async fn get_transactions(
        &self,
        block: &Block,
        block_voters: Option<BitVec>,
    ) -> ExecutorResult<(Vec<SignedTransaction>, Option<u64>, Option<u64>)>;
}
