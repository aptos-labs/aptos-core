// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::batch_generator::BackPressure;
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

#[derive(Debug)]
pub enum ProofQueueCommand {
    // Proof manager sends this command to add the proofs to the proof queue
    AddProofs(Vec<ProofOfStore>),
    // Batch coordinator sends this command to add the received batches to the proof queue.
    // For each transaction, the proof queue stores the list of batches containing the transaction.
    AddBatches(Vec<(BatchInfo, Vec<TransactionSummary>)>),
    // Proof manager sends this command to pull proofs from the proof queue to
    // include in the block proposal.
    PullProofs {
        excluded_batches: HashSet<BatchInfo>,
        max_txns: u64,
        max_bytes: u64,
        return_non_full: bool,
        response_sender: oneshot::Sender<(Vec<ProofOfStore>, bool)>,
    },
    // Proof manager sends this command to mark these batches as committed and
    // update the block timestamp.
    MarkCommitted(Vec<BatchInfo>, u64),
}

pub struct ProofQueue {
    my_peer_id: PeerId,
    // Queue per peer to ensure fairness between peers and priority within peer
    author_to_batches: HashMap<PeerId, BTreeMap<BatchSortKey, BatchInfo>>,
    // ProofOfStore and insertion_time. None if committed
    batch_to_proof: HashMap<BatchKey, Option<(ProofOfStore, Instant)>>,
    // Map of txn_summary = (sender, sequence number, hash) to all the batches that contain
    // the transaction. This helps in counting the number of unique transactions in the pipeline.
    txn_summary_to_batches: HashMap<TransactionSummary, HashSet<BatchKey>>,
    // List of batches for which we received txn summaries from the batch coordinator
    batches_with_txn_summary: HashSet<BatchKey>,
    // Expiration index
    expirations: TimeExpirations<BatchSortKey>,
    latest_block_timestamp: u64,
    back_pressure_total_txn_limit: u64,
    back_pressure_total_proof_limit: u64,
    remaining_txns_with_duplicates: u64,
    remaining_proofs: u64,
    remaining_local_txns: u64,
    remaining_local_proofs: u64,
}

impl ProofQueue {
    pub(crate) fn new(
        my_peer_id: PeerId,
        back_pressure_total_txn_limit: u64,
        back_pressure_total_proof_limit: u64,
    ) -> Self {
        Self {
            my_peer_id,
            author_to_batches: HashMap::new(),
            batch_to_proof: HashMap::new(),
            txn_summary_to_batches: HashMap::new(),
            batches_with_txn_summary: HashSet::new(),
            expirations: TimeExpirations::new(),
            latest_block_timestamp: 0,
            back_pressure_total_txn_limit,
            back_pressure_total_proof_limit,
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
        // All the batch keys for which batch_to_proof is not None. This is the set of unexpired and uncommitted proofs.
        let unexpired_batch_keys = self
            .batch_to_proof
            .iter()
            .filter(|(_, proof)| proof.is_some())
            .map(|(batch_key, _)| batch_key)
            .collect::<HashSet<_>>();
        let mut remaining_txns = self
            .txn_summary_to_batches
            .iter()
            .filter(|(_, batches)| {
                batches
                    .iter()
                    .any(|batch_key| unexpired_batch_keys.contains(batch_key))
            })
            .count() as u64;

        // If a batch_key is not in batches_with_txn_summary, it means we've received the proof but haven't receive the
        // transaction summary of the batch from batch coordinator. Add the number of txns in the batch to remaining_txns.
        remaining_txns += self
            .batch_to_proof
            .iter()
            .filter_map(|(batch_key, proof)| {
                if proof.is_some() && !self.batches_with_txn_summary.contains(batch_key) {
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
        for (batch_info, txn_summaries) in batch_summaries {
            let batch_key = BatchKey::from_info(&batch_info);
            for txn_summary in txn_summaries {
                self.txn_summary_to_batches
                    .entry(txn_summary)
                    .or_default()
                    .insert(batch_key.clone());
            }
            self.batches_with_txn_summary.insert(batch_key);
        }
    }

    /// return true when quorum store is back pressured
    pub(crate) fn qs_back_pressure(&self) -> BackPressure {
        let (remaining_total_txn_num, remaining_total_proof_num) = self.remaining_txns_and_proofs();
        BackPressure {
            txn_count: remaining_total_txn_num > self.back_pressure_total_txn_limit,
            proof_count: remaining_total_proof_num > self.back_pressure_total_proof_limit,
        }
    }

    // gets excluded and iterates over the vector returning non excluded or expired entries.
    // return the vector of pulled PoS, and the size of the remaining PoS
    // The flag in the second return argument is true iff the entire proof queue is fully utilized
    // when pulling the proofs. If any proof from proof queue cannot be included due to size limits,
    // this flag is set false.
    pub(crate) fn pull_proofs(
        &mut self,
        excluded_batches: &HashSet<BatchInfo>,
        max_txns: u64,
        max_bytes: u64,
        return_non_full: bool,
    ) -> (Vec<ProofOfStore>, bool) {
        let mut ret = vec![];
        let mut cur_bytes = 0;
        let mut cur_txns = 0;
        let mut excluded_txns = 0;
        let mut full = false;

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
                        if cur_bytes + batch.num_bytes() > max_bytes
                            || cur_txns + batch.num_txns() > max_txns
                        {
                            full = true;
                            return false;
                        }
                        cur_bytes += batch.num_bytes();
                        cur_txns += batch.num_txns();
                        let bucket = proof.gas_bucket_start();
                        ret.push(proof.clone());
                        counters::pos_to_pull(bucket, insertion_time.elapsed().as_secs_f64());
                        if cur_bytes == max_bytes || cur_txns == max_txns {
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

            // Number of proofs remaining in proof queue after the pull
            let mut num_proofs_remaining_after_pull = 0;
            let mut num_txns_remaining_after_pull = 0;
            let excluded_batch_keys = excluded_batches
                .iter()
                .map(BatchKey::from_info)
                .collect::<HashSet<_>>();
            let mut remaining_proofs = vec![];
            for (batch_key, proof) in &self.batch_to_proof {
                if proof.is_some()
                    && !ret
                        .iter()
                        .any(|p| BatchKey::from_info(p.info()) == *batch_key)
                    && !excluded_batch_keys.contains(batch_key)
                {
                    num_proofs_remaining_after_pull += 1;
                    num_txns_remaining_after_pull += proof.as_ref().unwrap().0.num_txns();
                    remaining_proofs.push(proof.as_ref().unwrap().0.clone());
                }
            }
            info!(
                "cur_txns: {}, remaining_proofs: {:?}",
                cur_txns, remaining_proofs
            );
            counters::NUM_PROOFS_IN_PROOF_QUEUE_AFTER_PULL
                .observe(num_proofs_remaining_after_pull as f64);
            counters::NUM_TXNS_IN_PROOF_QUEUE_AFTER_PULL
                .observe(num_txns_remaining_after_pull as f64);

            // Stable sort, so the order of proofs within an author will not change.
            ret.sort_by_key(|proof| Reverse(proof.gas_bucket_start()));
            (ret, !full)
        } else {
            (Vec::new(), !full)
        }
    }

    fn handle_updated_block_timestamp(&mut self, block_timestamp: u64) {
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
                        self.txn_summary_to_batches.retain(|_, batches| {
                            batches.remove(&key.batch_key);
                            !batches.is_empty()
                        });
                        self.batches_with_txn_summary.remove(&key.batch_key);
                        self.dec_remaining(&batch.author(), batch.num_txns());
                    }
                    claims::assert_some!(self.batch_to_proof.remove(&key.batch_key));
                }
                if !queue.is_empty() {
                    self.author_to_batches.insert(key.author(), queue);
                }
            }
        }
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
            self.txn_summary_to_batches
                .iter()
                .filter(|(_, batches)| batches.len() > 1)
                .count() as i64,
        );

        counters::TXNS_IN_PROOF_QUEUE.set(self.txn_summary_to_batches.len() as i64);

        // count the number of batches with proofs but without txn summaries
        counters::PROOFS_WITHOUT_BATCH_DATA.set(
            self.batch_to_proof
                .iter()
                .map(|(batch_key, proof)| {
                    if proof.is_some() && !self.batches_with_txn_summary.contains(batch_key) {
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
    fn mark_committed(&mut self, batches: Vec<BatchInfo>) {
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
            self.batches_with_txn_summary.remove(&batch_key);
        }
        let batch_keys = batches
            .iter()
            .map(BatchKey::from_info)
            .collect::<HashSet<_>>();
        self.txn_summary_to_batches.retain(|_, batches| {
            for batch_key in &batch_keys {
                batches.remove(batch_key);
            }
            !batches.is_empty()
        });
    }

    pub async fn start(
        mut self,
        back_pressure_tx: tokio::sync::mpsc::Sender<BackPressure>,
        mut command_rx: tokio::sync::mpsc::Receiver<ProofQueueCommand>,
    ) {
        let mut back_pressure = BackPressure {
            txn_count: false,
            proof_count: false,
        };

        loop {
            let _timer = counters::PROOF_QUEUE_MAIN_LOOP.start_timer();
            if let Some(msg) = command_rx.recv().await {
                match msg {
                    ProofQueueCommand::AddProofs(proofs) => {
                        let start = Instant::now();
                        for proof in proofs {
                            self.push(proof);
                        }
                        counters::PROOF_QUEUE_ADD_PROOFS_DURATION.observe_duration(start.elapsed());

                        let updated_back_pressure = self.qs_back_pressure();
                        if updated_back_pressure != back_pressure {
                            back_pressure = updated_back_pressure;
                            if back_pressure_tx.send(back_pressure).await.is_err() {
                                debug!("Failed to send back_pressure for proposal");
                            }
                        }
                    },
                    ProofQueueCommand::PullProofs {
                        excluded_batches,
                        max_txns,
                        max_bytes,
                        return_non_full,
                        response_sender,
                    } => {
                        let (proofs, full) = self.pull_proofs(
                            &excluded_batches,
                            max_txns,
                            max_bytes,
                            return_non_full,
                        );
                        if let Err(e) = response_sender.send((proofs, full)) {
                            warn!("Failed to send response to PullProofs: {:?}", e);
                        }
                    },
                    ProofQueueCommand::MarkCommitted(batches, block_timestamp) => {
                        let start = Instant::now();
                        self.mark_committed(batches);
                        counters::PROOF_QUEUE_COMMIT_DURATION.observe_duration(start.elapsed());

                        let start = Instant::now();
                        self.handle_updated_block_timestamp(block_timestamp);
                        counters::PROOF_QUEUE_UPDATE_TIMESTAMP_DURATION
                            .observe_duration(start.elapsed());

                        let updated_back_pressure = self.qs_back_pressure();
                        if updated_back_pressure != back_pressure {
                            back_pressure = updated_back_pressure;
                            if back_pressure_tx.send(back_pressure).await.is_err() {
                                debug!("Failed to send back_pressure for proposal");
                            }
                        }
                    },
                    ProofQueueCommand::AddBatches(batch_summaries) => {
                        let start = Instant::now();
                        self.add_batch_summaries(batch_summaries);
                        counters::PROOF_QUEUE_ADD_BATCH_SUMMARIES_DURATION
                            .observe_duration(start.elapsed());
                    },
                }
            }
        }
    }
}
