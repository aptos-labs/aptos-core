// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{monitor, quorum_store::counters};
use aptos_consensus_types::{
    common::{TransactionInProgress, TransactionSummary},
    proof_of_store::{BatchId, BatchInfo, ProofOfStore},
};
use aptos_logger::prelude::*;
use aptos_mempool::{QuorumStoreRequest, QuorumStoreResponse};
use aptos_types::{transaction::SignedTransaction, PeerId};
use chrono::Utc;
use futures::channel::{mpsc::Sender, oneshot};
use move_core_types::account_address::AccountAddress;
use rand::{seq::SliceRandom, thread_rng};
use std::{
    cmp::{Ordering, Reverse},
    collections::{BTreeMap, BinaryHeap, HashMap, HashSet, VecDeque},
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

    /// Expire and return items corresponding to expiration <= given certified time.
    pub(crate) fn expire(&mut self, certified_time: u64) -> HashSet<I> {
        let mut ret = HashSet::new();
        while let Some((Reverse(t), _)) = self.expiries.peek() {
            if *t <= certified_time {
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
        exclude_transactions: BTreeMap<TransactionSummary, TransactionInProgress>,
    ) -> Result<Vec<SignedTransaction>, anyhow::Error> {
        let (callback, callback_rcv) = oneshot::channel();
        let msg = QuorumStoreRequest::GetBatchRequest(
            max_items,
            max_bytes,
            true,
            exclude_transactions,
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
                "[quorum_store] did not receive GetBatchResponse on time"
            )),
            Ok(resp) => match resp.map_err(anyhow::Error::from)?? {
                QuorumStoreResponse::GetBatchResponse(txns) => Ok(txns),
                _ => Err(anyhow::anyhow!(
                    "[quorum_store] did not receive expected GetBatchResponse"
                )),
            },
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct BatchKey {
    author: PeerId,
    batch_id: BatchId,
}

impl BatchKey {
    pub fn from_info(info: &BatchInfo) -> Self {
        Self {
            author: info.author(),
            batch_id: info.batch_id(),
        }
    }
}

#[derive(PartialEq, Eq, Clone, Hash)]
pub struct BatchSortKey {
    batch_key: BatchKey,
    gas_bucket_start: u64,
}

impl BatchSortKey {
    pub fn from_info(info: &BatchInfo) -> Self {
        Self {
            batch_key: BatchKey::from_info(info),
            gas_bucket_start: info.gas_bucket_start(),
        }
    }

    pub fn author(&self) -> PeerId {
        self.batch_key.author
    }
}

impl PartialOrd<Self> for BatchSortKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BatchSortKey {
    fn cmp(&self, other: &Self) -> Ordering {
        // ascending
        match self.gas_bucket_start.cmp(&other.gas_bucket_start) {
            Ordering::Equal => {},
            ordering => return ordering,
        }
        // descending
        other.batch_key.batch_id.cmp(&self.batch_key.batch_id)
    }
}

pub struct ProofQueue {
    my_peer_id: PeerId,
    // Queue per peer to ensure fairness between peers and priority within peer
    author_to_batches: HashMap<PeerId, BTreeMap<BatchSortKey, BatchInfo>>,
    // ProofOfStore and insertion_time. None if committed
    batch_to_proof: HashMap<BatchKey, Option<(ProofOfStore, Instant)>>,
    // Number of batches in which the txn_summary = (sender, sequence number, hash) has been included
    txn_summary_num_occurrences: HashMap<TransactionSummary, u64>,
    // List of transaction summaries for each batch
    batch_to_txn_summaries: HashMap<BatchKey, Vec<TransactionSummary>>,
    // Expiration index
    expirations: TimeExpirations<BatchSortKey>,
    latest_block_timestamp: u64,
    remaining_txns_with_duplicates: u64,
    remaining_proofs: u64,
    remaining_local_txns: u64,
    remaining_local_proofs: u64,
}

impl ProofQueue {
    pub(crate) fn new(my_peer_id: PeerId) -> Self {
        Self {
            my_peer_id,
            author_to_batches: HashMap::new(),
            batch_to_proof: HashMap::new(),
            txn_summary_num_occurrences: HashMap::new(),
            batch_to_txn_summaries: HashMap::new(),
            expirations: TimeExpirations::new(),
            latest_block_timestamp: 0,
            remaining_txns_with_duplicates: 0,
            remaining_proofs: 0,
            remaining_local_txns: 0,
            remaining_local_proofs: 0,
        }
    }

    #[inline]
    fn inc_remaining(&mut self, author: &AccountAddress, num_txns: u64) {
        self.remaining_txns_with_duplicates += num_txns;
        self.remaining_proofs += 1;
        if *author == self.my_peer_id {
            self.remaining_local_txns += num_txns;
            self.remaining_local_proofs += 1;
        }
    }

    #[inline]
    fn dec_remaining(&mut self, author: &AccountAddress, num_txns: u64) {
        self.remaining_txns_with_duplicates -= num_txns;
        self.remaining_proofs -= 1;
        if *author == self.my_peer_id {
            self.remaining_local_txns -= num_txns;
            self.remaining_local_proofs -= 1;
        }
    }

    fn remaining_txns_without_duplicates(&self) -> u64 {
        let mut remaining_txns = self.txn_summary_num_occurrences.len() as u64;

        // If a batch_key is not in batches_with_txn_summary, it means we've received the proof but haven't receive the
        // transaction summary of the batch from batch coordinator. Add the number of txns in the batch to remaining_txns.
        remaining_txns += self
            .batch_to_proof
            .iter()
            .filter_map(|(batch_key, proof)| {
                if proof.is_some() && !self.batch_to_txn_summaries.contains_key(batch_key) {
                    Some(proof.as_ref().unwrap().0.num_txns())
                } else {
                    None
                }
            })
            .sum::<u64>();

        remaining_txns
    }

    /// Add the ProofOfStore to proof queue.
    pub(crate) fn push(&mut self, proof: ProofOfStore) {
        if proof.expiration() < self.latest_block_timestamp {
            counters::inc_rejected_pos_count(counters::POS_EXPIRED_LABEL);
            return;
        }
        let batch_key = BatchKey::from_info(proof.info());
        if self.batch_to_proof.contains_key(&batch_key) {
            counters::inc_rejected_pos_count(counters::POS_DUPLICATE_LABEL);
            return;
        }
        let author = proof.author();
        let bucket = proof.gas_bucket_start();
        let num_txns = proof.num_txns();
        let expiration = proof.expiration();

        let batch_sort_key = BatchSortKey::from_info(proof.info());
        let queue = self.author_to_batches.entry(author).or_default();
        queue.insert(batch_sort_key.clone(), proof.info().clone());
        self.expirations.add_item(batch_sort_key, expiration);
        self.batch_to_proof
            .insert(batch_key, Some((proof, Instant::now())));

        if author == self.my_peer_id {
            counters::inc_local_pos_count(bucket);
        } else {
            counters::inc_remote_pos_count(bucket);
        }
        self.inc_remaining(&author, num_txns);
    }

    pub(crate) fn add_batch_summaries(
        &mut self,
        batch_summaries: Vec<(BatchInfo, Vec<TransactionSummary>)>,
    ) {
        let start = Instant::now();
        for (batch_info, txn_summaries) in batch_summaries {
            let batch_key = BatchKey::from_info(&batch_info);
            if self
                .batch_to_txn_summaries
                .insert(batch_key, txn_summaries.clone())
                .is_none()
            {
                for txn_summary in txn_summaries {
                    if let Some(count) = self.txn_summary_num_occurrences.get_mut(&txn_summary) {
                        *count += 1;
                    } else {
                        self.txn_summary_num_occurrences.insert(txn_summary, 1);
                    }
                }
            }
        }
        counters::PROOF_QUEUE_ADD_BATCH_SUMMARIES_DURATION.observe_duration(start.elapsed());
    }

    fn log_remaining_data_after_pull(
        &self,
        excluded_batches: &HashSet<BatchInfo>,
        pulled_proofs: &[ProofOfStore],
    ) {
        let mut num_proofs_remaining_after_pull = 0;
        let mut num_txns_remaining_after_pull = 0;
        let excluded_batch_keys = excluded_batches
            .iter()
            .map(BatchKey::from_info)
            .collect::<HashSet<_>>();
        let mut remaining_proofs = vec![];
        for (batch_key, proof) in &self.batch_to_proof {
            if proof.is_some()
                && !pulled_proofs
                    .iter()
                    .any(|p| BatchKey::from_info(p.info()) == *batch_key)
                && !excluded_batch_keys.contains(batch_key)
            {
                num_proofs_remaining_after_pull += 1;
                num_txns_remaining_after_pull += proof.as_ref().unwrap().0.num_txns();
                remaining_proofs.push(proof.as_ref().unwrap().0.clone());
            }
        }
        let pulled_txns = pulled_proofs.iter().map(|p| p.num_txns()).sum::<u64>();
        info!(
            "pulled_proofs: {}, pulled_txns: {}, remaining_proofs: {:?}",
            pulled_proofs.len(),
            pulled_txns,
            remaining_proofs
        );
        counters::NUM_PROOFS_IN_PROOF_QUEUE_AFTER_PULL
            .observe(num_proofs_remaining_after_pull as f64);
        counters::NUM_TXNS_IN_PROOF_QUEUE_AFTER_PULL.observe(num_txns_remaining_after_pull as f64);
    }

    // gets excluded and iterates over the vector returning non excluded or expired entries.
    // return the vector of pulled PoS, and the size of the remaining PoS
    // The flag in the second return argument is true iff the entire proof queue is fully utilized
    // when pulling the proofs. If any proof from proof queue cannot be included due to size limits,
    // this flag is set false.
    // Returns the proofs, the number of unique transactions in the proofs, and a flag indicating
    // whether the proof queue is fully utilized.
    pub(crate) fn pull_proofs(
        &mut self,
        excluded_batches: &HashSet<BatchInfo>,
        max_txns: u64,
        max_unique_txns: u64,
        max_bytes: u64,
        return_non_full: bool,
    ) -> (Vec<ProofOfStore>, u64, bool) {
        let mut ret = vec![];
        let mut cur_bytes = 0;
        let mut cur_unique_txns = 0;
        let mut cur_all_txns = 0;
        let mut excluded_txns = 0;
        let mut full = false;
        // Set of all the excluded transactions and all the transactions included in the result
        let mut filtered_txns = HashSet::new();
        for batch_info in excluded_batches {
            let batch_key = BatchKey::from_info(batch_info);
            if let Some(txn_summaries) = self.batch_to_txn_summaries.get(&batch_key) {
                for txn_summary in txn_summaries {
                    filtered_txns.insert(*txn_summary);
                }
            }
        }

        let mut iters = vec![];
        for (_, batches) in self.author_to_batches.iter() {
            iters.push(batches.iter().rev());
        }

        while !iters.is_empty() {
            iters.shuffle(&mut thread_rng());
            iters.retain_mut(|iter| {
                if let Some((sort_key, batch)) = iter.next() {
                    if excluded_batches.contains(batch) {
                        excluded_txns += batch.num_txns();
                    } else if let Some(Some((proof, insertion_time))) =
                        self.batch_to_proof.get(&sort_key.batch_key)
                    {
                        // Calculate the number of unique transactions if this batch is included in the result
                        let unique_txns = if let Some(txn_summaries) =
                            self.batch_to_txn_summaries.get(&sort_key.batch_key)
                        {
                            cur_unique_txns
                                + txn_summaries
                                    .iter()
                                    .filter(|txn_summary| !filtered_txns.contains(txn_summary))
                                    .count() as u64
                        } else {
                            cur_unique_txns + batch.num_txns()
                        };
                        if cur_bytes + batch.num_bytes() > max_bytes
                            || unique_txns > max_unique_txns
                            || cur_all_txns + batch.num_txns() > max_txns
                        {
                            // Exceeded the limit for requested bytes or number of transactions.
                            full = true;
                            return false;
                        }
                        cur_bytes += batch.num_bytes();
                        cur_all_txns += batch.num_txns();
                        // Add this batch to filtered_txns and calculate the number of
                        // unique transactions added in the result so far.
                        cur_unique_txns += self
                            .batch_to_txn_summaries
                            .get(&sort_key.batch_key)
                            .map_or(batch.num_txns(), |summaries| {
                                summaries
                                    .iter()
                                    .filter(|summary| filtered_txns.insert(**summary))
                                    .count() as u64
                            });
                        let bucket = proof.gas_bucket_start();
                        ret.push(proof.clone());
                        counters::pos_to_pull(bucket, insertion_time.elapsed().as_secs_f64());
                        if cur_bytes == max_bytes
                            || cur_all_txns == max_txns
                            || cur_unique_txns == max_unique_txns
                        {
                            full = true;
                            return false;
                        }
                    }
                    true
                } else {
                    false
                }
            })
        }
        info!(
            // before non full check
            byte_size = cur_bytes,
            block_total_txns = cur_all_txns,
            block_unique_txns = cur_unique_txns,
            batch_count = ret.len(),
            full = full,
            return_non_full = return_non_full,
            "Pull payloads from QuorumStore: internal"
        );

        if full || return_non_full {
            counters::BLOCK_SIZE_WHEN_PULL.observe(cur_unique_txns as f64);
            counters::TOTAL_BLOCK_SIZE_WHEN_PULL.observe(cur_all_txns as f64);
            counters::KNOWN_DUPLICATE_TXNS_WHEN_PULL
                .observe((cur_all_txns.saturating_sub(cur_unique_txns)) as f64);
            counters::BLOCK_BYTES_WHEN_PULL.observe(cur_bytes as f64);
            counters::PROOF_SIZE_WHEN_PULL.observe(ret.len() as f64);
            counters::EXCLUDED_TXNS_WHEN_PULL.observe(excluded_txns as f64);
            // Number of proofs remaining in proof queue after the pull
            self.log_remaining_data_after_pull(excluded_batches, &ret);
            // Stable sort, so the order of proofs within an author will not change.
            ret.sort_by_key(|proof| Reverse(proof.gas_bucket_start()));
            (ret, cur_unique_txns, !full)
        } else {
            (Vec::new(), 0, !full)
        }
    }

    pub(crate) fn handle_updated_block_timestamp(&mut self, block_timestamp: u64) {
        let start = Instant::now();
        assert!(
            self.latest_block_timestamp <= block_timestamp,
            "Decreasing block timestamp"
        );
        self.latest_block_timestamp = block_timestamp;

        let expired = self.expirations.expire(block_timestamp);
        let mut num_expired_but_not_committed = 0;
        for key in &expired {
            if let Some(mut queue) = self.author_to_batches.remove(&key.author()) {
                if let Some(batch) = queue.remove(key) {
                    if self
                        .batch_to_proof
                        .get(&key.batch_key)
                        .expect("Entry for unexpired batch must exist")
                        .is_some()
                    {
                        // non-committed proof that is expired
                        num_expired_but_not_committed += 1;
                        counters::GAP_BETWEEN_BATCH_EXPIRATION_AND_CURRENT_TIME_WHEN_COMMIT
                            .observe((block_timestamp - batch.expiration()) as f64);
                        if let Some(txn_summaries) = self.batch_to_txn_summaries.get(&key.batch_key)
                        {
                            for txn_summary in txn_summaries {
                                if let Some(count) =
                                    self.txn_summary_num_occurrences.get_mut(txn_summary)
                                {
                                    *count -= 1;
                                };
                            }
                        }
                        self.batch_to_txn_summaries.remove(&key.batch_key);
                        self.dec_remaining(&batch.author(), batch.num_txns());
                    }
                    claims::assert_some!(self.batch_to_proof.remove(&key.batch_key));
                }
                if !queue.is_empty() {
                    self.author_to_batches.insert(key.author(), queue);
                }
            }
        }
        self.txn_summary_num_occurrences
            .retain(|_, count| *count > 0);
        counters::PROOF_QUEUE_UPDATE_TIMESTAMP_DURATION.observe_duration(start.elapsed());
        counters::NUM_PROOFS_EXPIRED_WHEN_COMMIT.inc_by(num_expired_but_not_committed);
    }

    pub(crate) fn remaining_txns_and_proofs(&self) -> (u64, u64) {
        let start = Instant::now();
        counters::NUM_TOTAL_TXNS_LEFT_ON_UPDATE.observe(self.remaining_txns_with_duplicates as f64);
        counters::NUM_TOTAL_PROOFS_LEFT_ON_UPDATE.observe(self.remaining_proofs as f64);
        counters::NUM_LOCAL_TXNS_LEFT_ON_UPDATE.observe(self.remaining_local_txns as f64);
        counters::NUM_LOCAL_PROOFS_LEFT_ON_UPDATE.observe(self.remaining_local_proofs as f64);
        let remaining_txns_without_duplicates = self.remaining_txns_without_duplicates();
        counters::NUM_UNIQUE_TOTAL_TXNS_LEFT_ON_UPDATE
            .observe(remaining_txns_without_duplicates as f64);
        //count the number of transactions with more than one batches
        counters::TXNS_WITH_DUPLICATE_BATCHES.set(
            self.txn_summary_num_occurrences
                .iter()
                .filter(|(_, count)| **count > 1)
                .count() as i64,
        );

        counters::TXNS_IN_PROOF_QUEUE.set(self.txn_summary_num_occurrences.len() as i64);

        // count the number of batches with proofs but without txn summaries
        counters::PROOFS_WITHOUT_BATCH_DATA.set(
            self.batch_to_proof
                .iter()
                .map(|(batch_key, proof)| {
                    if proof.is_some() && !self.batch_to_txn_summaries.contains_key(batch_key) {
                        1
                    } else {
                        0
                    }
                })
                .sum::<i64>(),
        );

        counters::PROOFS_IN_PROOF_QUEUE.set(
            self.batch_to_proof
                .values()
                .map(|proof| if proof.is_some() { 1 } else { 0 })
                .sum::<i64>(),
        );
        counters::PROOF_QUEUE_REMAINING_TXNS_DURATION.observe_duration(start.elapsed());
        (remaining_txns_without_duplicates, self.remaining_proofs)
    }

    // Mark in the hashmap committed PoS, but keep them until they expire
    pub(crate) fn mark_committed(&mut self, batches: Vec<BatchInfo>) {
        let start = Instant::now();
        for batch in &batches {
            let batch_key = BatchKey::from_info(batch);
            if let Some(Some((proof, insertion_time))) = self.batch_to_proof.get(&batch_key) {
                counters::pos_to_commit(
                    proof.gas_bucket_start(),
                    insertion_time.elapsed().as_secs_f64(),
                );
                self.dec_remaining(&batch.author(), batch.num_txns());
            }
            self.batch_to_proof.insert(batch_key.clone(), None);
            if let Some(txn_summaries) = self.batch_to_txn_summaries.get(&batch_key) {
                for txn_summary in txn_summaries {
                    if let Some(count) = self.txn_summary_num_occurrences.get_mut(txn_summary) {
                        *count -= 1;
                    };
                }
            }
            self.batch_to_txn_summaries.remove(&batch_key);
        }
        self.txn_summary_num_occurrences
            .retain(|_, count| *count > 0);
        counters::PROOF_QUEUE_COMMIT_DURATION.observe_duration(start.elapsed());
    }
}
