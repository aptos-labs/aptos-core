// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    counters::WAIT_FOR_FULL_BLOCKS_TRIGGERED, error::QuorumStoreError, monitor,
    payload_client::user::UserPayloadClient,
};
use aptos_consensus_types::{
    common::{Payload, PayloadFilter},
    payload_pull_params::{OptQSPayloadPullParams, PayloadPullParameters},
    request_response::{GetPayloadCommand, GetPayloadRequest, GetPayloadResponse},
    utils::PayloadTxnsSize,
};
use aptos_logger::info;
use fail::fail_point;
use futures_channel::{mpsc, oneshot};
use std::time::{Duration, Instant};
use tokio::time::{sleep, timeout};

const NO_TXN_DELAY: u64 = 30;

/// Client that pulls blocks from Quorum Store
#[derive(Clone)]
pub struct QuorumStoreClient {
    consensus_to_quorum_store_sender: mpsc::Sender<GetPayloadCommand>,
    /// Timeout for consensus to pull transactions from quorum store and get a response (in milliseconds)
    pull_timeout_ms: u64,
    wait_for_full_blocks_above_recent_fill_threshold: f32,
    wait_for_full_blocks_above_pending_blocks: usize,
}

impl QuorumStoreClient {
    pub fn new(
        consensus_to_quorum_store_sender: mpsc::Sender<GetPayloadCommand>,
        pull_timeout_ms: u64,
        wait_for_full_blocks_above_recent_fill_threshold: f32,
        wait_for_full_blocks_above_pending_blocks: usize,
    ) -> Self {
        Self {
            consensus_to_quorum_store_sender,
            pull_timeout_ms,
            wait_for_full_blocks_above_recent_fill_threshold,
            wait_for_full_blocks_above_pending_blocks,
        }
    }

    async fn pull_internal(
        &self,
        max_txns: PayloadTxnsSize,
        max_txns_after_filtering: u64,
        soft_max_txns_after_filtering: u64,
        max_inline_txns: PayloadTxnsSize,
        maybe_optqs_payload_pull_params: Option<OptQSPayloadPullParams>,
        return_non_full: bool,
        exclude_payloads: PayloadFilter,
        block_timestamp: Duration,
    ) -> anyhow::Result<Payload, QuorumStoreError> {
        let (callback, callback_rcv) = oneshot::channel();
        let req = GetPayloadCommand::GetPayloadRequest(GetPayloadRequest {
            max_txns,
            max_txns_after_filtering,
            soft_max_txns_after_filtering,
            maybe_optqs_payload_pull_params,
            max_inline_txns,
            filter: exclude_payloads,
            return_non_full,
            callback,
            block_timestamp,
        });
        // send to shared mempool
        self.consensus_to_quorum_store_sender
            .clone()
            .try_send(req)
            .map_err(anyhow::Error::from)?;
        // wait for response
        match monitor!(
            "pull_payload",
            timeout(Duration::from_millis(self.pull_timeout_ms), callback_rcv).await
        ) {
            Err(_) => {
                Err(anyhow::anyhow!("[consensus] did not receive GetBlockResponse on time").into())
            },
            Ok(resp) => match resp.map_err(anyhow::Error::from)?? {
                GetPayloadResponse::GetPayloadResponse(payload) => Ok(payload),
            },
        }
    }
}

#[async_trait::async_trait]
impl UserPayloadClient for QuorumStoreClient {
    async fn pull(
        &self,
        params: PayloadPullParameters,
    ) -> anyhow::Result<Payload, QuorumStoreError> {
        let return_non_full = params.recent_max_fill_fraction
            < self.wait_for_full_blocks_above_recent_fill_threshold
            && params.pending_uncommitted_blocks < self.wait_for_full_blocks_above_pending_blocks;
        let return_empty = params.pending_ordering && return_non_full;

        WAIT_FOR_FULL_BLOCKS_TRIGGERED.observe(if !return_non_full { 1.0 } else { 0.0 });

        fail_point!("consensus::pull_payload", |_| {
            Err(anyhow::anyhow!("Injected error in pull_payload").into())
        });
        // keep polling QuorumStore until there's payloads available or there's still pending payloads
        let start_time = Instant::now();

        let payload = loop {
            // Make sure we don't wait more than expected, due to thread scheduling delays/processing time consumed
            let done = start_time.elapsed() >= params.max_poll_time;
            let payload = self
                .pull_internal(
                    params.max_txns,
                    params.max_txns_after_filtering,
                    params.soft_max_txns_after_filtering,
                    params.max_inline_txns,
                    params.maybe_optqs_payload_pull_params.clone(),
                    return_non_full || return_empty || done,
                    params.user_txn_filter.clone(),
                    params.block_timestamp,
                )
                .await?;
            if payload.is_empty() && !return_empty && !done {
                sleep(Duration::from_millis(NO_TXN_DELAY)).await;
                continue;
            }
            break payload;
        };
        info!(
            pull_params = ?params,
            elapsed_time_ms = start_time.elapsed().as_millis() as u64,
            payload_len = payload.len(),
            return_empty = return_empty,
            return_non_full = return_non_full,
            duration_ms = start_time.elapsed().as_millis() as u64,
            "Pull payloads from QuorumStore: proposal"
        );

        Ok(payload)
    }
}
