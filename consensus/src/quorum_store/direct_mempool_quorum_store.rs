// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{monitor, quorum_store::counters};
use anyhow::Result;
use aptos_logger::prelude::*;
use aptos_mempool::{QuorumStoreRequest, QuorumStoreResponse};
use aptos_types::transaction::SignedTransaction;
use consensus_types::{
    common::{Payload, PayloadFilter, TransactionSummary},
    request_response::{ConsensusRequest, ConsensusResponse},
};
use futures::{
    channel::{
        mpsc::{Receiver, Sender},
        oneshot,
    },
    StreamExt,
};
use std::time::{Duration, Instant};
use tokio::time::timeout;

pub struct DirectMempoolQuorumStore {
    consensus_receiver: Receiver<ConsensusRequest>,
    mempool_sender: Sender<QuorumStoreRequest>,
    mempool_txn_pull_timeout_ms: u64,
}

impl DirectMempoolQuorumStore {
    pub fn new(
        consensus_receiver: Receiver<ConsensusRequest>,
        mempool_sender: Sender<QuorumStoreRequest>,
        mempool_txn_pull_timeout_ms: u64,
    ) -> Self {
        Self {
            consensus_receiver,
            mempool_sender,
            mempool_txn_pull_timeout_ms,
        }
    }

    async fn pull_internal(
        &self,
        max_items: u64,
        max_bytes: u64,
        exclude_txns: Vec<TransactionSummary>,
    ) -> Result<Vec<SignedTransaction>, anyhow::Error> {
        let (callback, callback_rcv) = oneshot::channel();
        let msg = QuorumStoreRequest::GetBatchRequest(max_items, max_bytes, exclude_txns, callback);
        self.mempool_sender
            .clone()
            .try_send(msg)
            .map_err(anyhow::Error::from)?;
        // wait for response
        match monitor!(
            "pull_txn",
            timeout(
                Duration::from_millis(self.mempool_txn_pull_timeout_ms),
                callback_rcv
            )
            .await
        ) {
            Err(_) => Err(anyhow::anyhow!(
                "[direct_mempool_quorum_store] did not receive GetBatchResponse on time"
            )),
            Ok(resp) => match resp.map_err(anyhow::Error::from)?? {
                QuorumStoreResponse::GetBatchResponse(txns) => Ok(txns),
                _ => Err(anyhow::anyhow!(
                    "[direct_mempool_quorum_store] did not receive expected GetBatchResponse"
                )),
            },
        }
    }

    async fn handle_block_request(
        &self,
        max_txns: u64,
        max_bytes: u64,
        payload_filter: PayloadFilter,
        callback: oneshot::Sender<Result<ConsensusResponse>>,
    ) {
        let get_batch_start_time = Instant::now();
        let (txns, result) = match payload_filter {
            PayloadFilter::DirectMempool(exclude_txns) => {
                match self.pull_internal(max_txns, max_bytes, exclude_txns).await {
                    Err(_) => {
                        error!("GetBatch failed");
                        (vec![], counters::REQUEST_FAIL_LABEL)
                    }
                    Ok(txns) => (txns, counters::REQUEST_SUCCESS_LABEL),
                }
            }
            _ => {
                panic!("Unknown payload_filter: {}", payload_filter)
            }
        };
        counters::quorum_store_service_latency(
            counters::GET_BATCH_LABEL,
            result,
            get_batch_start_time.elapsed(),
        );

        let get_block_response_start_time = Instant::now();
        let payload = Payload::DirectMempool(txns);
        let result = match callback.send(Ok(ConsensusResponse::GetBlockResponse(payload))) {
            Err(_) => {
                error!("Callback failed");
                counters::CALLBACK_FAIL_LABEL
            }
            Ok(_) => counters::CALLBACK_SUCCESS_LABEL,
        };
        counters::quorum_store_service_latency(
            counters::GET_BLOCK_RESPONSE_LABEL,
            result,
            get_block_response_start_time.elapsed(),
        );
    }

    async fn handle_clean_request(&self, callback: oneshot::Sender<Result<ConsensusResponse>>) {
        match callback.send(Ok(ConsensusResponse::CleanResponse())) {
            Err(_) => {
                error!("Callback failed");
                counters::CALLBACK_FAIL_LABEL
            }
            Ok(_) => counters::CALLBACK_SUCCESS_LABEL,
        };
    }

    async fn handle_consensus_request(&self, req: ConsensusRequest) {
        match req {
            ConsensusRequest::GetBlockRequest(max_txns, max_bytes, payload_filter, callback) => {
                self.handle_block_request(max_txns, max_bytes, payload_filter, callback)
                    .await;
            }
            ConsensusRequest::CleanRequest(_, _, callback) => {
                self.handle_clean_request(callback).await;
            }
        }
    }

    pub async fn start(mut self) {
        loop {
            let _timer = counters::MAIN_LOOP.start_timer();
            ::futures::select! {
                msg = self.consensus_receiver.select_next_some() => {
                    self.handle_consensus_request(msg).await;
                },
                complete => break,
            }
        }
    }
}
