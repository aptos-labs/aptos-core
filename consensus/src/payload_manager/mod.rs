// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_bitvec::BitVec;
use aptos_consensus_types::{
    block::Block,
    common::{Author, Payload},
    payload::RaptrPayload,
};
use aptos_executor_types::*;
use aptos_types::transaction::SignedTransaction;
use async_trait::async_trait;
use std::time::Duration;

mod co_payload_manager;
mod direct_mempool_payload_manager;
mod quorum_store_payload_manager;

pub use co_payload_manager::ConsensusObserverPayloadManager;
pub use direct_mempool_payload_manager::DirectMempoolPayloadManager;
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
    fn prefetch_payload_data(
        &self,
        payload: &Payload,
        author: Author,
        timestamp: u64,
        block_voters: Option<BitVec>,
    );

    /// Check if the transactions corresponding are available. This is specific to payload
    /// manager implementations. For optimistic quorum store, we only check if optimistic
    /// batches are available locally.
    fn check_payload_availability(&self, payload: &Payload) -> Result<(), BitVec> {
        todo!()
    }

    fn available_prefix(&self, payload: &RaptrPayload, cached_value: usize) -> (usize, BitVec) {
        todo!()
    }

    async fn wait_for_payload(
        &self,
        payload: &Payload,
        block_author: Option<Author>,
        block_timestamp: u64,
        timeout: Duration,
        wait_for_proof: bool,
    ) -> anyhow::Result<()> {
        todo!()
    }

    /// Get the transactions in a block's payload. This function returns a vector of transactions.
    async fn get_transactions(
        &self,
        block: &Block,
        block_voters: Option<BitVec>,
    ) -> ExecutorResult<(Vec<SignedTransaction>, Option<u64>)>;
}
