// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{monitor, quorum_store::counters};
use aptos_consensus_types::{
    common::TransactionSummary,
    proof_of_store::{BatchInfo, ProofOfStore},
};
use aptos_logger::prelude::*;
use aptos_mempool::{QuorumStoreRequest, QuorumStoreResponse};
use aptos_types::{transaction::SignedTransaction, PeerId};
use chrono::Utc;
use futures::channel::{mpsc::Sender, oneshot};
use rand::{seq::SliceRandom, thread_rng};
use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashMap, HashSet, VecDeque},
    hash::Hash,
    time::{Duration, Instant},
};
use tokio::time::timeout;

pub(crate) struct Timeouts<T> {
    timeouts: VecDeque<(i64, T)>,
}

impl<T> Timeouts<T> {
    pub(crate) fn new() -> Self {
        Self {
            timeouts: VecDeque::new(),
        }
    }

    pub(crate) fn add(&mut self, value: T, timeout: usize) {
        let expiry = Utc::now().naive_utc().timestamp_millis() + timeout as i64;
        self.timeouts.push_back((expiry, value));
    }

    pub(crate) fn expire(&mut self) -> Vec<T> {
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

pub(crate) struct TimeExpirations<I: Ord> {
    expiries: BinaryHeap<(Reverse<u64>, I)>,
}

impl<I: Ord + Hash> TimeExpirations<I> {
    pub(crate) fn new() -> Self {
        Self {
            expiries: BinaryHeap::new(),
        }
    }

    pub(crate) fn add_item(&mut self, item: I, expiry_time: u64) {
        self.expiries.push((Reverse(expiry_time), item));
    }

    /// Expire and return items corresponding to round <= given (expired) round.
    pub(crate) fn expire(&mut self, expiry_time: u64) -> HashSet<I> {
        let mut ret = HashSet::new();
        while let Some((Reverse(t), _)) = self.expiries.peek() {
            if *t <= expiry_time {
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

pub struct ProofQueue {
    my_peer_id: PeerId,
    // Queue per peer to ensure fairness, includes insertion time
    author_to_batches: HashMap<PeerId, Vec<BatchInfo>>,
    // ProofOfStore and insertion_time. None if committed
    batch_to_proof: HashMap<BatchInfo, Option<(ProofOfStore, Instant)>>,
    latest_block_timestamp: u64,
    max_in_future_usecs: u64,
    max_per_author: usize,
}

impl ProofQueue {
    pub(crate) fn new(my_peer_id: PeerId, max_in_future_usecs: u64, max_per_author: u64) -> Self {
        Self {
            my_peer_id,
            author_to_batches: HashMap::new(),
            batch_to_proof: HashMap::new(),
            latest_block_timestamp: 0,
            max_in_future_usecs,
            max_per_author: max_per_author as usize,
        }
    }

    pub(crate) fn push(&mut self, proof: ProofOfStore) {
        if proof.expiration() < self.latest_block_timestamp {
            counters::REJECTED_POS_COUNT
                .with_label_values(&["expired"])
                .inc();
            return;
        }
        if proof.expiration()
            > aptos_infallible::duration_since_epoch().as_micros() as u64 + self.max_in_future_usecs
        {
            counters::REJECTED_POS_COUNT
                .with_label_values(&["too_far_in_future"])
                .inc();
            return;
        }
        if self.batch_to_proof.get(proof.info()).is_some() {
            counters::REJECTED_POS_COUNT
                .with_label_values(&["duplicate"])
                .inc();
            return;
        }

        if let Some(queue) = self.author_to_batches.get(&proof.author()) {
            if queue.len() >= self.max_per_author {
                sample!(
                    SampleRate::Duration(Duration::from_secs(10)),
                    warn!("Queue full for author {}", proof.author())
                );
                counters::REJECTED_POS_COUNT
                    .with_label_values(&["queue_full"])
                    .inc();
                return;
            }
        }

        let author = proof.author();
        let queue = self.author_to_batches.entry(author).or_default();
        queue.push(proof.info().clone());
        self.batch_to_proof
            .insert(proof.info().clone(), Some((proof, Instant::now())));

        if author == self.my_peer_id {
            counters::LOCAL_POS_COUNT.inc();
        } else {
            counters::REMOTE_POS_COUNT.inc();
        }
    }

    // gets excluded and iterates over the vector returning non excluded or expired entries.
    // return the vector of pulled PoS, and the size of the remaining PoS
    pub(crate) fn pull_proofs(
        &mut self,
        excluded_batches: &HashSet<BatchInfo>,
        max_txns: u64,
        max_bytes: u64,
        return_non_full: bool,
    ) -> Vec<ProofOfStore> {
        let mut ret = vec![];
        let mut cur_bytes = 0;
        let mut cur_txns = 0;
        let mut excluded_txns = 0;
        let mut full = false;

        let mut author_to_num_remaining = HashMap::new();
        for (author, batches) in self.author_to_batches.iter() {
            author_to_num_remaining.insert(*author, batches.len());
        }

        'outer: while !author_to_num_remaining.is_empty() {
            let shuffled_peers: Vec<_> = {
                let mut peers: Vec<_> = author_to_num_remaining.keys().cloned().collect();
                peers.shuffle(&mut thread_rng());
                peers
            };

            for peer in shuffled_peers {
                let queue = self.author_to_batches.get(&peer).unwrap();
                let num_remaining = author_to_num_remaining.remove(&peer).unwrap();

                let mut num_read = 0;
                for i in (queue.len() - num_remaining)..queue.len() {
                    let batch = queue.get(i).unwrap();
                    num_read += 1;
                    if excluded_batches.contains(batch) {
                        excluded_txns += batch.num_txns();
                    } else if self.batch_to_proof.get(batch).unwrap().is_some() {
                        cur_bytes += batch.num_bytes();
                        cur_txns += batch.num_txns();
                        if cur_bytes > max_bytes || cur_txns > max_txns {
                            // Exceeded the limit for requested bytes or number of transactions.
                            full = true;
                            break 'outer;
                        }
                        let (proof, insertion_time) =
                            self.batch_to_proof.get(batch).unwrap().clone().unwrap();
                        ret.push(proof);
                        counters::POS_TO_PULL.observe(insertion_time.elapsed().as_secs_f64());
                        break;
                    }
                }
                if num_remaining != num_read {
                    author_to_num_remaining.insert(peer, num_remaining - num_read);
                }
            }
        }
        info!(
            // before non full check
            byte_size = cur_bytes,
            block_size = cur_txns,
            batch_count = ret.len(),
            full = full,
            return_non_full = return_non_full,
            "Pull payloads from QuorumStore: internal"
        );

        if full || return_non_full {
            counters::BLOCK_SIZE_WHEN_PULL.observe(cur_txns as f64);
            counters::BLOCK_BYTES_WHEN_PULL.observe(cur_bytes as f64);
            counters::PROOF_SIZE_WHEN_PULL.observe(ret.len() as f64);
            counters::EXCLUDED_TXNS_WHEN_PULL.observe(excluded_txns as f64);
            ret
        } else {
            Vec::new()
        }
    }

    pub(crate) fn handle_updated_block_timestamp(&mut self, block_timestamp: u64) {
        assert!(
            self.latest_block_timestamp <= block_timestamp,
            "Decreasing block timestamp"
        );
        self.latest_block_timestamp = block_timestamp;

        let peers: Vec<_> = self.author_to_batches.keys().cloned().collect();
        let mut num_expired_but_not_committed = 0;

        for peer in peers {
            let mut queue = self.author_to_batches.remove(&peer).unwrap();
            let num_expired = queue
                .iter()
                .take_while(|batch| batch.expiration() < block_timestamp)
                .count();

            for batch in queue.drain(0..num_expired) {
                if self
                    .batch_to_proof
                    .get(&batch)
                    .expect("Entry for unexpired batch must exist")
                    .is_some()
                {
                    // non-committed proof that is expired
                    num_expired_but_not_committed += 1;
                    if batch.expiration() < block_timestamp {
                        counters::GAP_BETWEEN_BATCH_EXPIRATION_AND_CURRENT_TIME_WHEN_COMMIT
                            .observe((block_timestamp - batch.expiration()) as f64);
                    }
                }
                claims::assert_some!(self.batch_to_proof.remove(&batch));
            }

            if !queue.is_empty() {
                self.author_to_batches.insert(peer, queue);
            }
        }
        counters::NUM_PROOFS_EXPIRED_WHEN_COMMIT.inc_by(num_expired_but_not_committed);
    }

    pub(crate) fn num_total_txns_and_proofs(&mut self, current_block_timestamp: u64) -> (u64, u64) {
        let mut remaining_txns = 0;
        let mut remaining_proofs = 0;
        let mut remaining_local_txns = 0;
        let mut remaining_local_proofs = 0;

        // TODO: if the digest_queue is large, this may be too inefficient
        let peers: Vec<_> = self.author_to_batches.keys().cloned().collect();
        for peer in peers {
            // TODO: queue direction!
            let queue = self.author_to_batches.get(&peer).unwrap();
            for batch in queue.iter() {
                // Not expired
                if batch.expiration() >= current_block_timestamp {
                    // Not committed
                    if let Some(Some((proof, _))) = self.batch_to_proof.get(batch) {
                        remaining_txns += proof.num_txns();
                        remaining_proofs += 1;
                        if proof.author() == self.my_peer_id {
                            remaining_local_txns += proof.num_txns();
                            remaining_local_proofs += 1;
                        }
                    }
                }
            }
        }
        counters::NUM_TOTAL_TXNS_LEFT_ON_COMMIT.observe(remaining_txns);
        counters::NUM_TOTAL_PROOFS_LEFT_ON_COMMIT.observe(remaining_proofs);
        counters::NUM_LOCAL_TXNS_LEFT_ON_COMMIT.observe(remaining_local_txns);
        counters::NUM_LOCAL_PROOFS_LEFT_ON_COMMIT.observe(remaining_local_proofs);

        (remaining_txns, remaining_proofs)
    }

    // Mark in the hashmap committed PoS, but keep them until they expire
    pub(crate) fn mark_committed(&mut self, batches: Vec<BatchInfo>) {
        for batch in batches {
            if let Some(Some((_, insertion_time))) = self.batch_to_proof.get(&batch) {
                counters::POS_TO_COMMIT.observe(insertion_time.elapsed().as_secs_f64());
            }
            self.batch_to_proof.insert(batch, None);
        }
    }
}
