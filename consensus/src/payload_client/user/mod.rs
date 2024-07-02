// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::error::QuorumStoreError;
use aptos_consensus_types::common::{Payload, PayloadFilter};
#[cfg(test)]
use aptos_types::transaction::SignedTransaction;
use futures::future::BoxFuture;
use std::time::Duration;
#[cfg(test)]
use std::time::Instant;

/// Clients can pull information about transactions from the mempool and return
/// the retrieved information as a `Payload`.
#[async_trait::async_trait]
pub trait UserPayloadClient: Send + Sync {
    async fn pull(
        &self,
        max_poll_time: Duration,
        max_items: u64,
        max_unique_items: u64,
        max_bytes: u64,
        max_inline_items: u64,
        max_inline_bytes: u64,
        exclude: PayloadFilter,
        wait_callback: BoxFuture<'static, ()>,
        pending_ordering: bool,
        pending_uncommitted_blocks: usize,
        recent_max_fill_fraction: f32,
    ) -> anyhow::Result<Payload, QuorumStoreError>;
}

/// A dummy user payload client that pull hardcoded txns one by one.
#[cfg(test)]
pub struct DummyClient {
    pub(crate) txns: Vec<SignedTransaction>,
}

#[cfg(test)]
impl DummyClient {
    pub fn new(txns: Vec<SignedTransaction>) -> Self {
        Self { txns }
    }
}

#[cfg(test)]
#[async_trait::async_trait]
impl UserPayloadClient for DummyClient {
    async fn pull(
        &self,
        max_poll_time: Duration,
        mut max_items: u64,
        mut max_unique_items: u64,
        mut max_bytes: u64,
        _max_inline_items: u64,
        _max_inline_bytes: u64,
        _exclude: PayloadFilter,
        _wait_callback: BoxFuture<'static, ()>,
        _pending_ordering: bool,
        _pending_uncommitted_blocks: usize,
        _recent_max_fill_fraction: f32,
    ) -> anyhow::Result<Payload, QuorumStoreError> {
        let timer = Instant::now();
        let mut nxt_txn_idx = 0;
        let mut txns = vec![];
        while timer.elapsed() < max_poll_time
            && max_items >= 1
            && max_unique_items >= 1
            && max_bytes >= 1
            && nxt_txn_idx < self.txns.len()
        {
            tokio::time::sleep(Duration::from_millis(1)).await;
            let txn = self.txns[nxt_txn_idx].clone();
            let txn_size = txn.raw_txn_bytes_len() as u64;
            if txn_size > max_bytes {
                break;
            }
            max_items -= 1;
            max_unique_items -= 1;
            max_bytes -= txn_size;
            nxt_txn_idx += 1;
            txns.push(txn);
        }
        Ok(Payload::DirectMempool(txns))
    }
}

pub mod quorum_store_client;
