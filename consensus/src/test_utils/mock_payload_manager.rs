// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::QuorumStoreError,
    payload_client::{user::quorum_store_client::QuorumStoreClient, PayloadClient},
};
use anyhow::Result;
use aptos_consensus_types::{
    block::block_test_utils::random_payload,
    common::{Payload, PayloadFilter},
    request_response::GetPayloadCommand,
};
use aptos_types::{
    transaction::{ExecutionStatus, TransactionStatus},
    validator_txn::ValidatorTransaction,
    vm_status::StatusCode,
};
use aptos_validator_transaction_pool as vtxn_pool;
use futures::{channel::mpsc, future::BoxFuture};
use rand::Rng;
use std::time::Duration;

#[allow(dead_code)]
pub struct MockPayloadManager {
    // used non-mocked PayloadClient to test interaction with shared mempool
    _quorum_store_client: Option<QuorumStoreClient>,
}

impl MockPayloadManager {
    pub fn new(consensus_to_quorum_store_sender: Option<mpsc::Sender<GetPayloadCommand>>) -> Self {
        let quorum_store_client =
            consensus_to_quorum_store_sender.map(|s| QuorumStoreClient::new(s, 1, 1.1, 100));
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
impl PayloadClient for MockPayloadManager {
    /// The returned future is fulfilled with the vector of SignedTransactions
    async fn pull_payload(
        &self,
        _max_poll_time: Duration,
        _max_size: u64,
        _max_size_after_filtering: u64,
        _soft_max_size_after_filtering: u64,
        _max_bytes: u64,
        _max_inline_size: u64,
        _max_inline_bytes: u64,
        _validator_txn_filter: vtxn_pool::TransactionFilter,
        _user_txn_filter: PayloadFilter,
        _wait_callback: BoxFuture<'static, ()>,
        _pending_ordering: bool,
        _pending_uncommitted_blocks: usize,
        _recent_fill_fraction: f32,
        _block_timestamp: Duration,
    ) -> Result<(Vec<ValidatorTransaction>, Payload), QuorumStoreError> {
        // generate 1k txn is too slow with coverage instrumentation
        Ok((
            vec![ValidatorTransaction::dummy(vec![0xFF; 1])],
            random_payload(10),
        ))
    }
}
