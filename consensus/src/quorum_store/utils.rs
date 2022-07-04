// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::HashValue;
use aptos_mempool::{QuorumStoreRequest, QuorumStoreResponse};
use aptos_metrics_core::monitor;
use aptos_types::transaction::SignedTransaction;
use chrono::Utc;
use consensus_types::common::{Round, TransactionSummary};
use futures::channel::{mpsc::Sender, oneshot};
use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashSet, VecDeque},
    hash::Hash,
    time::Duration,
};
use tokio::time::timeout;

pub(crate) struct DigestTimeouts {
    timeouts: VecDeque<(i64, HashValue)>,
}

impl DigestTimeouts {
    pub(crate) fn new() -> Self {
        Self {
            timeouts: VecDeque::new(),
        }
    }

    pub(crate) fn add_digest(&mut self, digest: HashValue, timeout: usize) {
        let expiry = Utc::now().naive_utc().timestamp_millis() + timeout as i64;
        self.timeouts.push_back((expiry, digest));
    }

    pub(crate) fn expire(&mut self) -> Vec<HashValue> {
        let cur_time = chrono::Utc::now().naive_utc().timestamp_millis();
        let num_expired = self
            .timeouts
            .iter()
            .take_while(|(expiration_time, _)| cur_time >= *expiration_time)
            .count();

        self.timeouts
            .drain(0..num_expired)
            .map(|(_, h)| h)
            .collect()
    }
}

pub(crate) struct RoundExpirations<I: Ord> {
    expiries: BinaryHeap<(Reverse<Round>, I)>,
}

impl<I: Ord + Hash> RoundExpirations<I> {
    pub(crate) fn new() -> Self {
        Self {
            expiries: BinaryHeap::new(),
        }
    }

    pub(crate) fn add_item(&mut self, item: I, expiry_round: Round) {
        self.expiries.push((Reverse(expiry_round), item));
    }

    /// Expire and return items corresponding to round <= given (expired) round.
    pub(crate) fn expire(&mut self, round: Round) -> HashSet<I> {
        let mut ret = HashSet::new();
        while let Some((Reverse(r), _)) = self.expiries.peek() {
            if *r <= round {
                let (_, item) = self.expiries.pop().unwrap();
                ret.insert(item);
            }
        }
        ret
    }
}

pub struct MempoolProxy {
    mempool_tx: Sender<QuorumStoreRequest>,
    mempool_txn_pull_timeout_ms: u64,
}

impl MempoolProxy {
    pub fn new(mempool_tx: Sender<QuorumStoreRequest>, mempool_txn_pull_timeout_ms: u64) -> Self {
        Self {
            mempool_tx,
            mempool_txn_pull_timeout_ms,
        }
    }

    pub async fn pull_internal(
        &self,
        max_size: u64,
        exclude_txns: Vec<TransactionSummary>,
    ) -> Result<Vec<SignedTransaction>, anyhow::Error> {
        let (callback, callback_rcv) = oneshot::channel();
        let msg = QuorumStoreRequest::GetBatchRequest(max_size, exclude_txns, callback);
        self.mempool_tx
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
}
