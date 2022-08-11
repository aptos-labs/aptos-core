// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::QuorumStoreError, payload_manager::QuorumStoreClient, state_replication::PayloadManager,
};
use anyhow::Result;
use aptos_types::{
    transaction::{ExecutionStatus, TransactionStatus},
    vm_status::StatusCode,
};
use consensus_types::{
    block::block_test_utils::random_payload,
    common::{Payload, PayloadFilter},
    request_response::ConsensusRequest,
};
use futures::{channel::mpsc, future::BoxFuture};
use rand::Rng;

pub struct MockPayloadManager {
    // used non-mocked TxnManager to test interaction with shared mempool
    _quorum_store_client: Option<QuorumStoreClient>,
}

impl MockPayloadManager {
    pub fn new(consensus_to_quorum_store_sender: Option<mpsc::Sender<ConsensusRequest>>) -> Self {
        let quorum_store_client =
            consensus_to_quorum_store_sender.map(|s| QuorumStoreClient::new(s, 1, 1));
        Self {
            _quorum_store_client: quorum_store_client,
        }
    }
}

// mock transaction status on the fly
fn _mock_transaction_status(count: usize) -> Vec<TransactionStatus> {
    let mut statuses = vec![];
    // generate count + 1 status to mock the block metadata txn in mempool proxy
    for _ in 0..=count {
        let random_status = match rand::thread_rng().gen_range(0, 1000) {
            0 => TransactionStatus::Discard(StatusCode::UNKNOWN_VALIDATION_STATUS),
            _ => TransactionStatus::Keep(ExecutionStatus::Success),
        };
        statuses.push(random_status);
    }
    statuses
}

#[async_trait::async_trait]
impl PayloadManager for MockPayloadManager {
    /// The returned future is fulfilled with the vector of SignedTransactions
    async fn pull_payload(
        &self,
        _max_size: u64,
        _max_bytes: u64,
        _exclude: PayloadFilter,
        _wait_callback: BoxFuture<'static, ()>,
        _pending_ordering: bool,
    ) -> Result<Payload, QuorumStoreError> {
        // generate 1k txn is too slow with coverage instrumentation
        Ok(random_payload(10))
    }
}
