// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::quorum_store::types::{BatchId, TxnData};
use aptos_crypto::{hash::DefaultHasher, HashValue};
use aptos_mempool::{QuorumStoreRequest, QuorumStoreResponse};
use aptos_metrics_core::monitor;
use aptos_types::transaction::SignedTransaction;
use bcs::to_bytes;
use chrono::Utc;
use consensus_types::common::{Round, TransactionSummary};
use futures::channel::{mpsc::Sender, oneshot};
use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashSet, VecDeque},
    hash::Hash,
    mem,
    time::Duration,
};
use tokio::time::timeout;

pub(crate) struct BatchBuilder {
    id: BatchId,
    summaries: Vec<TransactionSummary>,
    data: Vec<TxnData>,
    num_bytes: usize,
    max_bytes: usize,
}

impl BatchBuilder {
    pub(crate) fn new(batch_id: BatchId, max_bytes: usize) -> Self {
        Self {
            id: batch_id,
            summaries: Vec::new(),
            data: Vec::new(),
            num_bytes: 0,
            max_bytes,
        }
    }

    pub(crate) fn append_transaction(&mut self, txn: &SignedTransaction) -> bool {
        let bytes = to_bytes(&txn).unwrap();

        if self.num_bytes + bytes.len() <= self.max_bytes {
            self.summaries.push(TransactionSummary {
                sender: txn.sender(),
                sequence_number: txn.sequence_number(),
            });
            self.num_bytes = self.num_bytes + bytes.len();

            // TODO: check if hashing per txn is too costly (hopefully not as hashes are
            // associated with txns later in the process). Also, potentially parallelize.
            // Then, we should also probably parallelize incoming digest computation.
            let mut hasher = DefaultHasher::new(b"TxnData");
            hasher.update(&bytes);
            self.data.push(TxnData {
                txn_bytes: bytes,
                hash: hasher.finish(),
            });
            true
        } else {
            false
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub(crate) fn batch_id(&self) -> BatchId {
        self.id
    }

    /// Clears the state, increments (batch) id.
    pub(crate) fn take_batch(&mut self) -> (Vec<TransactionSummary>, Vec<TxnData>) {
        self.id = self.id + 1;
        self.num_bytes = 0;
        (mem::take(&mut self.summaries), mem::take(&mut self.data))
    }

    pub(crate) fn cloned_summaries(&self) -> Vec<TransactionSummary> {
        self.summaries.clone()
    }
}

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
            } else {
                break;
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
