// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::quorum_store::counters;
use crate::quorum_store::utils::MempoolProxy;
use anyhow::Result;
use aptos_logger::prelude::*;
use aptos_mempool::QuorumStoreRequest;
use consensus_types::{
    common::{Payload, PayloadFilter},
    request_response::{ConsensusResponse, WrapperCommand},
};
use futures::{
    channel::{
        mpsc::{Receiver, Sender},
        oneshot,
    },
    StreamExt,
};
use std::time::Instant;

pub struct DirectMempoolQuorumStore {
    mempool_proxy: MempoolProxy,
}

impl DirectMempoolQuorumStore {
    pub fn new(mempool_tx: Sender<QuorumStoreRequest>, mempool_txn_pull_timeout_ms: u64) -> Self {
        Self {
            mempool_proxy: MempoolProxy::new(mempool_tx, mempool_txn_pull_timeout_ms),
        }
    }

    async fn handle_block_request(
        &self,
        max_size: u64,
        payload_filter: PayloadFilter,
        callback: oneshot::Sender<Result<ConsensusResponse>>,
    ) {
        let get_batch_start_time = Instant::now();
        let exclude_txns = match payload_filter {
            PayloadFilter::DirectMempool(exclude_txns) => exclude_txns,
            PayloadFilter::InQuorumStore(_) => {
                unreachable!("Unknown payload_filter: {}", payload_filter)
            }
            PayloadFilter::Empty => Vec::new(),
        };

        let (txns, result) = match self
            .mempool_proxy
            .pull_internal(max_size, exclude_txns)
            .await
        {
            Err(_) => {
                error!("GetBatch failed");
                (vec![], counters::REQUEST_FAIL_LABEL)
            }
            Ok(txns) => (txns, counters::REQUEST_SUCCESS_LABEL),
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

    async fn handle_consensus_request(&self, req: WrapperCommand) {
        match req {
            WrapperCommand::GetBlockRequest(_round, max_size, payload_filter, callback) => {
                self.handle_block_request(max_size, payload_filter, callback)
                    .await;
            }
            WrapperCommand::CleanRequest(..) => {
                unreachable!()
            }
        }
    }

    pub async fn start(self, mut consensus_rx: Receiver<WrapperCommand>) {
        while let Some(cmd) = consensus_rx.next().await {
            self.handle_consensus_request(cmd).await;
        }
    }
}
