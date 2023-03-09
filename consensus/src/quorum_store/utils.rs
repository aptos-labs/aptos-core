// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    monitor,
    quorum_store::{
        counters,
        types::{BatchId, SerializedTransaction},
    },
};
use aptos_consensus_types::{
    common::{Round, TransactionSummary},
    proof_of_store::{LogicalTime, ProofOfStore},
};
use aptos_crypto::HashValue;
use aptos_logger::prelude::*;
use aptos_mempool::{QuorumStoreRequest, QuorumStoreResponse};
use aptos_types::transaction::SignedTransaction;
use chrono::Utc;
use futures::channel::{mpsc::Sender, oneshot};
use std::{
    cmp::Reverse,
    collections::{
        hash_map::Entry::{Occupied, Vacant},
        BinaryHeap, HashMap, HashSet, VecDeque,
    },
    hash::Hash,
    mem,
    time::{Duration, Instant},
};
use tokio::time::timeout;

pub(crate) struct BatchBuilder {
    id: BatchId,
    summaries: Vec<TransactionSummary>,
    data: Vec<SerializedTransaction>,
    num_txns: usize,
    num_bytes: usize,
    // TODO: remove
    max_bytes: usize,
}

impl BatchBuilder {
    pub(crate) fn new(batch_id: BatchId, max_bytes: usize) -> Self {
        Self {
            id: batch_id,
            summaries: Vec::new(),
            data: Vec::new(),
            num_txns: 0,
            num_bytes: 0,
            max_bytes,
        }
    }

    pub(crate) fn append_transaction(
        &mut self,
        txn: &SignedTransaction,
        max_txns_override: usize,
    ) -> bool {
        let serialized_txn = SerializedTransaction::from_signed_txn(txn);

        if self.num_bytes + serialized_txn.len() <= self.max_bytes
            && self.num_txns < max_txns_override
        {
            self.num_txns += 1;
            self.num_bytes += serialized_txn.len();

            self.summaries.push(TransactionSummary {
                sender: txn.sender(),
                sequence_number: txn.sequence_number(),
            });
            self.data.push(serialized_txn);

            self.num_txns < max_txns_override && self.num_bytes < self.max_bytes
        } else {
            false
        }
    }

    pub(crate) fn take_serialized_txns(&mut self) -> Vec<SerializedTransaction> {
        mem::take(&mut self.data)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub(crate) fn batch_id(&self) -> BatchId {
        self.id
    }

    /// Clears the state, increments (batch) id.
    pub(crate) fn take_summaries(&mut self) -> Vec<TransactionSummary> {
        assert!(self.data.is_empty());

        self.id.increment();
        self.num_bytes = 0;
        self.num_txns = 0;
        mem::take(&mut self.summaries)
    }

    pub(crate) fn summaries(&self) -> &Vec<TransactionSummary> {
        &self.summaries
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
        trace!(
            "QS: expire cur time {} timeouts len {}",
            cur_time,
            self.timeouts.len()
        );
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
        max_items: u64,
        max_bytes: u64,
        return_non_full: bool,
        exclude_txns: Vec<TransactionSummary>,
    ) -> Result<Vec<SignedTransaction>, anyhow::Error> {
        let (callback, callback_rcv) = oneshot::channel();
        let msg = QuorumStoreRequest::GetBatchRequest(
            max_items,
            max_bytes,
            return_non_full,
            exclude_txns,
            callback,
        );
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

// TODO: unitest
pub struct ProofQueue {
    digest_queue: VecDeque<(HashValue, LogicalTime)>, // queue of all proofs
    local_digest_queue: VecDeque<(HashValue, LogicalTime)>, // queue of local proofs, to make back pressure update more efficient
    digest_proof: HashMap<HashValue, Option<ProofOfStore>>, // None means committed
    digest_insertion_time: HashMap<HashValue, Instant>,
}

impl ProofQueue {
    pub(crate) fn new() -> Self {
        Self {
            digest_queue: VecDeque::new(),
            local_digest_queue: VecDeque::new(),
            digest_proof: HashMap::new(),
            digest_insertion_time: HashMap::new(),
        }
    }

    pub(crate) fn push(&mut self, proof: ProofOfStore, local: bool) {
        match self.digest_proof.entry(*proof.digest()) {
            Vacant(entry) => {
                self.digest_queue
                    .push_back((*proof.digest(), proof.expiration()));
                entry.insert(Some(proof.clone()));
                self.digest_insertion_time
                    .insert(*proof.digest(), Instant::now());
            },
            Occupied(mut entry) => {
                if entry.get().is_some()
                    && entry.get().as_ref().unwrap().expiration() < proof.expiration()
                {
                    entry.insert(Some(proof.clone()));
                }
            },
        }
        if local {
            self.local_digest_queue
                .push_back((*proof.digest(), proof.expiration()));
        }
    }

    // gets excluded and iterates over the vector returning non excluded or expired entries.
    // return the vector of pulled PoS, and the size of the remaining PoS
    pub(crate) fn pull_proofs(
        &mut self,
        excluded_proofs: &HashSet<HashValue>,
        current_time: LogicalTime,
        max_txns: u64,
        max_bytes: u64,
        return_non_full: bool,
    ) -> Vec<ProofOfStore> {
        let num_expired = self
            .digest_queue
            .iter()
            .take_while(|(_, expiration_time)| *expiration_time < current_time)
            .count();
        let mut num_expired_but_not_committed = 0;
        for (digest, expiration_time) in self.digest_queue.drain(0..num_expired) {
            if self
                .digest_proof
                .get(&digest)
                .expect("Entry for unexpired digest must exist")
                .is_some()
            {
                // non-committed proof that is expired
                num_expired_but_not_committed += 1;
                if expiration_time.round() < current_time.round() {
                    counters::GAP_BETWEEN_BATCH_EXPIRATION_AND_CURRENT_ROUND_WHEN_PULL_PROOFS
                        .observe((current_time.round() - expiration_time.round()) as f64);
                }
            }
            claims::assert_some!(self.digest_proof.remove(&digest));
            self.digest_insertion_time.remove(&digest);
        }

        let mut ret = Vec::new();
        let mut cur_bytes = 0;
        let mut cur_txns = 0;
        let initial_size = self.digest_queue.len();
        let mut size = self.digest_queue.len();
        let mut full = false;

        for (digest, expiration) in self
            .digest_queue
            .iter()
            .filter(|(digest, _)| !excluded_proofs.contains(digest))
        {
            if let Some(proof) = self
                .digest_proof
                .get(digest)
                .expect("Entry for unexpired digest must exist")
            {
                if *expiration >= current_time {
                    // non-committed proof that has not expired
                    cur_bytes += proof.info().num_bytes;
                    cur_txns += proof.info().num_txns;
                    if cur_bytes > max_bytes || cur_txns > max_txns {
                        // Exceeded the limit for requested bytes or number of transactions.
                        full = true;
                        break;
                    }
                    ret.push(proof.clone());
                    if let Some(insertion_time) = self.digest_insertion_time.get(digest) {
                        counters::POS_TO_PULL.observe(insertion_time.elapsed().as_secs_f64());
                    }
                } else {
                    // non-committed proof that is expired
                    num_expired_but_not_committed += 1;
                    if expiration.round() < current_time.round() {
                        counters::GAP_BETWEEN_BATCH_EXPIRATION_AND_CURRENT_ROUND_WHEN_PULL_PROOFS
                            .observe((current_time.round() - expiration.round()) as f64);
                    }
                }
            }
            size -= 1;
        }
        info!(
            // before non full check
            byte_size = cur_bytes,
            block_size = cur_txns,
            batch_count = ret,
            remaining_proof_num = size,
            initial_remaining_proof_num = initial_size,
            full = full,
            return_non_full = return_non_full,
            "Pull payloads from QuorumStore: internal"
        );

        if full || return_non_full {
            counters::EXPIRED_PROOFS_WHEN_PULL.observe(num_expired_but_not_committed as f64);
            counters::BLOCK_SIZE_WHEN_PULL.observe(cur_txns as f64);
            counters::BLOCK_BYTES_WHEN_PULL.observe(cur_bytes as f64);
            counters::PROOF_SIZE_WHEN_PULL.observe(ret.len() as f64);
            ret
        } else {
            Vec::new()
        }
    }

    pub(crate) fn num_total_txns_and_proofs(&mut self, current_time: LogicalTime) -> (u64, u64) {
        let mut remaining_txns = 0;
        let mut remaining_proofs = 0;
        // TODO: if the digest_queue is large, this may be too inefficient
        for (digest, expiration) in self.digest_queue.iter() {
            // Not expired
            if *expiration >= current_time {
                // Not committed
                if let Some(Some(proof)) = self.digest_proof.get(digest) {
                    remaining_txns += proof.info().num_txns;
                    remaining_proofs += 1;
                }
            }
        }
        counters::NUM_TOTAL_TXNS_LEFT_ON_COMMIT.observe(remaining_txns as f64);
        counters::NUM_TOTAL_PROOFS_LEFT_ON_COMMIT.observe(remaining_proofs as f64);

        (remaining_txns, remaining_proofs)
    }

    // returns the number of unexpired local proofs
    pub(crate) fn clean_local_proofs(&mut self, current_time: LogicalTime) -> Option<Round> {
        let num_expired = self
            .local_digest_queue
            .iter()
            .take_while(|(_, expiration_time)| *expiration_time < current_time)
            .count();
        self.local_digest_queue.drain(0..num_expired);

        let mut remaining_local_proof_size = 0;

        for (digest, expiration) in self.local_digest_queue.iter() {
            // Not expired. It is possible that the proof entry in digest_proof was already removed
            // when draining the digest_queue but local_digest_queue is not drained yet.
            if *expiration >= current_time {
                if let Some(entry) = self.digest_proof.get(digest) {
                    // Not committed
                    if entry.is_some() {
                        remaining_local_proof_size += 1;
                    }
                }
            }
        }
        counters::NUM_LOCAL_PROOFS_LEFT_ON_COMMIT.observe(remaining_local_proof_size as f64);

        if let Some(&(_, time)) = self.local_digest_queue.iter().next() {
            Some(time.round())
        } else {
            None
        }
    }

    //mark in the hashmap committed PoS, but keep them until they expire
    pub(crate) fn mark_committed(&mut self, digests: Vec<HashValue>) {
        for digest in digests {
            self.digest_proof.insert(digest, None);
            if let Some(insertion_time) = self.digest_insertion_time.get(&digest) {
                counters::POS_TO_COMMIT.observe(insertion_time.elapsed().as_secs_f64());
            }
        }
    }
}
