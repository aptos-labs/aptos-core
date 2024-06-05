// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::utils::TimeExpirations;
use crate::quorum_store::counters;
use aptos_consensus_types::proof_of_store::{BatchId, BatchInfo, ProofOfStore};
use aptos_logger::prelude::*;
use aptos_types::PeerId;
use futures::channel::oneshot;
use move_core_types::account_address::AccountAddress;
use rand::{seq::SliceRandom, thread_rng};
use std::{
    cmp::{Ordering, Reverse},
    collections::{BTreeMap, HashMap, HashSet},
    time::Instant,
};

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
    // We send back (remaining_txns, remaining_proofs) to the proof manager
    AddProofs(Vec<ProofOfStore>, oneshot::Sender<(u64, u64)>),
    // Batch coordinator sends this command to add the received batches to the proof queue.
    // For each transaction, the proof queue stores the list of batches containing the transaction.
    AddBatches(Vec<(BatchInfo, Vec<(PeerId, u64)>)>),
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
    // We send back the (remaining_txns, remaining_proofs) to the proof manager
    MarkCommitted(Vec<BatchInfo>, u64, oneshot::Sender<(u64, u64)>),
}

pub struct ProofQueue {
    my_peer_id: PeerId,
    // Queue per peer to ensure fairness between peers and priority within peer
    author_to_batches: HashMap<PeerId, BTreeMap<BatchSortKey, BatchInfo>>,
    // ProofOfStore and insertion_time. None if committed
    batch_to_proof: HashMap<BatchKey, Option<(ProofOfStore, Instant)>>,
    // Map of txn_summary = (sender, sequence number) to all the batches that contain
    // the transaction. This helps in counting the number of unique transactions in the pipeline.
    txn_summary_to_batches: HashMap<(PeerId, u64), HashSet<BatchKey>>,
    // List of batches for which we received txn summaries from the batch coordinator
    batches_with_txn_summary: HashSet<BatchKey>,
    // Expiration index
    expirations: TimeExpirations<BatchSortKey>,
    latest_block_timestamp: u64,
    remaining_txns: u64,
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
            txn_summary_to_batches: HashMap::new(),
            batches_with_txn_summary: HashSet::new(),
            expirations: TimeExpirations::new(),
            latest_block_timestamp: 0,
            remaining_txns: 0,
            remaining_proofs: 0,
            remaining_local_txns: 0,
            remaining_local_proofs: 0,
        }
    }

    #[inline]
    fn inc_remaining(&mut self, author: &AccountAddress, num_txns: u64) {
        self.remaining_txns += num_txns;
        self.remaining_proofs += 1;
        if *author == self.my_peer_id {
            self.remaining_local_txns += num_txns;
            self.remaining_local_proofs += 1;
        }
    }

    #[inline]
    fn dec_remaining(&mut self, author: &AccountAddress, num_txns: u64) {
        self.remaining_txns -= num_txns;
        self.remaining_proofs -= 1;
        if *author == self.my_peer_id {
            self.remaining_local_txns -= num_txns;
            self.remaining_local_proofs -= 1;
        }
    }

    fn remaining_txns(&self) -> u64 {
        // All the bath keys for which batch_to_proof is not None. This is the set of unexpired and uncommitted proofs.
        let batch_keys = self
            .batch_to_proof
            .iter()
            .filter_map(|(batch_key, proof)| proof.as_ref().map(|_| batch_key))
            .collect::<HashSet<_>>();
        let mut remaining_txns = self
            .txn_summary_to_batches
            .iter()
            .filter(|(_, batches)| {
                batches
                    .iter()
                    .any(|batch_key| batch_keys.contains(batch_key))
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
        if self.batch_to_proof.get(&batch_key).is_some() {
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
                if full {
                    return false;
                }
                if let Some((sort_key, batch)) = iter.next() {
                    if excluded_batches.contains(batch) {
                        excluded_txns += batch.num_txns();
                    } else if let Some(Some((proof, insertion_time))) =
                        self.batch_to_proof.get(&sort_key.batch_key)
                    {
                        cur_bytes += batch.num_bytes();
                        cur_txns += batch.num_txns();
                        if cur_bytes > max_bytes || cur_txns > max_txns {
                            // Exceeded the limit for requested bytes or number of transactions.
                            full = true;
                            return false;
                        }
                        let bucket = proof.gas_bucket_start();
                        ret.push(proof.clone());
                        counters::pos_to_pull(bucket, insertion_time.elapsed().as_secs_f64());
                        if cur_bytes == max_bytes || cur_txns == max_txns {
                            // Exactly the limit for requested bytes or number of transactions.
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
        counters::NUM_TOTAL_TXNS_LEFT_ON_UPDATE.observe(self.remaining_txns as f64);
        counters::NUM_TOTAL_PROOFS_LEFT_ON_UPDATE.observe(self.remaining_proofs as f64);
        counters::NUM_LOCAL_TXNS_LEFT_ON_UPDATE.observe(self.remaining_local_txns as f64);
        counters::NUM_LOCAL_PROOFS_LEFT_ON_UPDATE.observe(self.remaining_local_proofs as f64);
        let remaining_txns_without_duplicates = self.remaining_txns();
        counters::NUM_TOTAL_TXNS_LEFT_ON_UPDATE_WITHOUT_DUPLICATES
            .observe(remaining_txns_without_duplicates as f64);
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
            self.txn_summary_to_batches.retain(|_, batches| {
                batches.remove(&batch_key);
                !batches.is_empty()
            });
        }
    }

    pub async fn start(mut self, mut command_rx: tokio::sync::mpsc::Receiver<ProofQueueCommand>) {
        loop {
            let _timer = counters::PROOF_MANAGER_MAIN_LOOP.start_timer();
            if let Some(msg) = command_rx.recv().await {
                match msg {
                    ProofQueueCommand::AddProofs(proofs, response_sender) => {
                        for proof in proofs {
                            self.push(proof);
                        }
                        if let Err(e) = response_sender.send(self.remaining_txns_and_proofs()) {
                            warn!("Failed to send response to AddProofs: {:?}", e);
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
                    ProofQueueCommand::MarkCommitted(batches, block_timestamp, response_sender) => {
                        self.mark_committed(batches);
                        self.handle_updated_block_timestamp(block_timestamp);
                        if let Err(e) = response_sender.send(self.remaining_txns_and_proofs()) {
                            error!("Failed to send response to MarkCommitted: {:?}", e);
                        }
                    },
                    ProofQueueCommand::AddBatches(batch_summaries) => {
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
                    },
                }
            }
        }
    }
}
