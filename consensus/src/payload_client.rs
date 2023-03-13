// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    counters::WAIT_FOR_FULL_BLOCKS_TRIGGERED, error::QuorumStoreError, monitor,
    state_replication::PayloadClient,
};
use anyhow::Result;
use aptos_consensus_types::{
    common::{Payload, PayloadFilter, Round},
    request_response::{GetPayloadCommand, GetPayloadResponse},
};
use aptos_logger::prelude::*;
use fail::fail_point;
use futures::{
    channel::{mpsc, oneshot},
    future::BoxFuture,
};
use std::time::{Duration, Instant};
use tokio::time::{sleep, timeout};

const NO_TXN_DELAY: u64 = 30; // TODO: consider moving to a config

/// Client that pulls blocks from Quorum Store
#[derive(Clone)]
pub struct QuorumStoreClient {
    consensus_to_quorum_store_sender: mpsc::Sender<GetPayloadCommand>,
    poll_count: u64,
    /// Timeout for consensus to pull transactions from quorum store and get a response (in milliseconds)
    pull_timeout_ms: u64,
    wait_for_full_blocks_above_recent_fill_threshold: f32,
    wait_for_full_blocks_above_pending_blocks: usize,
}

impl QuorumStoreClient {
    pub fn new(
        consensus_to_quorum_store_sender: mpsc::Sender<GetPayloadCommand>,
        poll_count: u64,
        pull_timeout_ms: u64,
        wait_for_full_blocks_above_recent_fill_threshold: f32,
        wait_for_full_blocks_above_pending_blocks: usize,
    ) -> Self {
        assert!(
            poll_count > 0,
            "poll_count = 0 won't pull any txns from quorum store"
        );
        Self {
            consensus_to_quorum_store_sender,
            poll_count,
            pull_timeout_ms,
            wait_for_full_blocks_above_recent_fill_threshold,
            wait_for_full_blocks_above_pending_blocks,
        }
    }

    async fn pull_internal(
        &self,
        round: Round,
        max_items: u64,
        max_bytes: u64,
        return_non_full: bool,
        exclude_payloads: PayloadFilter,
    ) -> Result<Payload, QuorumStoreError> {
        let (callback, callback_rcv) = oneshot::channel();
        let req = GetPayloadCommand::GetPayloadRequest(
            round,
            max_items,
            max_bytes,
            return_non_full,
            exclude_payloads.clone(),
            callback,
        );
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
impl PayloadClient for QuorumStoreClient {
    async fn pull_payload(
        &self,
        round: Round,
        max_items: u64,
        max_bytes: u64,
        exclude_payloads: PayloadFilter,
        wait_callback: BoxFuture<'static, ()>,
        pending_ordering: bool,
        pending_uncommitted_blocks: usize,
        recent_max_fill_fraction: f32,
    ) -> Result<Payload, QuorumStoreError> {
        let return_non_full = recent_max_fill_fraction
            < self.wait_for_full_blocks_above_recent_fill_threshold
            && pending_uncommitted_blocks < self.wait_for_full_blocks_above_pending_blocks;
        let return_empty = pending_ordering && return_non_full;

        WAIT_FOR_FULL_BLOCKS_TRIGGERED.observe(u64::from(!return_non_full));

        fail_point!("consensus::pull_payload", |_| {
            Err(anyhow::anyhow!("Injected error in pull_payload").into())
        });
        let mut callback_wrapper = Some(wait_callback);
        // keep polling QuorumStore until there's payloads available or there's still pending payloads
        let mut count = self.poll_count;
        let start_time = Instant::now();
        let max_duration = (self.poll_count.saturating_sub(1) * NO_TXN_DELAY) as u128;
        let payload = loop {
            count -= 1;
            // Make sure we don't wait more than expected, due to thread scheduling delays/processing time consumed
            let done = count == 0 || start_time.elapsed().as_millis() >= max_duration;
            let payload = self
                .pull_internal(
                    round,
                    max_items,
                    max_bytes,
                    return_non_full || return_empty || done || self.poll_count == u64::MAX,
                    exclude_payloads.clone(),
                )
                .await?;
            if payload.is_empty() && !return_empty && !done {
                if let Some(callback) = callback_wrapper.take() {
                    callback.await;
                }
                sleep(Duration::from_millis(NO_TXN_DELAY)).await;
                continue;
            }
            break payload;
        };
        info!(
            poll_count = self.poll_count - count,
            max_poll_count = self.poll_count,
            payload_len = payload.len(),
            max_items = max_items,
            max_bytes = max_bytes,
            pending_ordering = pending_ordering,
            return_empty = return_empty,
            return_non_full = return_non_full,
            duration = start_time.elapsed().as_secs_f32(),
            "Pull payloads from QuorumStore: proposal"
        );
        Ok(payload)
    }
}
