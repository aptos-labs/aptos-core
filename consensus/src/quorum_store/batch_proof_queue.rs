// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{
    batch_store::BatchStore,
    utils::{BatchKey, BatchSortKey, TimeExpirations},
};
use crate::quorum_store::counters;
use aptos_consensus_types::{
    common::{Author, TxnSummaryWithExpiration},
    payload::TDataInfo,
    proof_of_store::{BatchInfo, ProofOfStore},
    utils::PayloadTxnsSize,
};
use aptos_logger::{info, sample, sample::SampleRate, warn};
use aptos_metrics_core::TimerHelper;
use aptos_short_hex_str::AsShortHexStr;
use aptos_types::{transaction::SignedTransaction, PeerId};
use rand::{prelude::SliceRandom, thread_rng};
use raptr::raptr::duration_since_epoch;
use rayon::prelude::*;
use std::{
    cmp::Reverse,
    collections::{hash_map::Entry, BTreeMap, HashMap, HashSet},
    ops::Bound,
    sync::Arc,
    time::{Duration, Instant},
};

/// QueueItem represents an item in the ProofBatchQueue.
/// It stores the transaction summaries and proof associated with the
/// batch.
struct QueueItem {
    /// The info of the Batch this item stores
    info: BatchInfo,
    /// Contains the summary of transactions in the batch.
    /// It is optional as the summary can be updated after the proof.
    txn_summaries: Option<Vec<TxnSummaryWithExpiration>>,

    /// Contains the proof associated with the batch.
    /// It is optional as the proof can be updated after the summary.
    proof: Option<ProofOfStore>,
    /// The time when the proof is inserted into this item.
    proof_insertion_time: Option<Instant>,
    batch_insert_time: Option<Instant>,
}

impl QueueItem {
    fn is_committed(&self) -> bool {
        self.proof.is_none() && self.proof_insertion_time.is_none() && self.txn_summaries.is_none()
    }

    fn mark_committed(&mut self) {
        self.proof = None;
        self.proof_insertion_time = None;
        self.txn_summaries = None;
    }
}

pub struct BatchProofQueue {
    my_peer_id: PeerId,
    // Queue per peer to ensure fairness between peers and priority within peer
    author_to_batches: HashMap<PeerId, BTreeMap<BatchSortKey, BatchInfo>>,
    // Map of Batch key to QueueItem containing Batch data and proofs
    items: HashMap<BatchKey, QueueItem>,
    // Number of unexpired and uncommitted proofs in which the txn_summary = (sender, sequence number, hash, expiration)
    // has been included. We only count those batches that are in both author_to_batches and items along with proofs.
    txn_summary_num_occurrences: HashMap<TxnSummaryWithExpiration, u64>,
    // Expiration index
    expirations: TimeExpirations<BatchSortKey>,
    batch_store: Arc<BatchStore>,

    latest_block_timestamp: u64,
    remaining_txns_with_duplicates: u64,
    remaining_proofs: u64,
    remaining_local_txns: u64,
    remaining_local_proofs: u64,

    batch_expiry_gap_when_init_usecs: u64,
    max_batches_per_pull: usize,
}

impl BatchProofQueue {
    pub(crate) fn new(
        my_peer_id: PeerId,
        batch_store: Arc<BatchStore>,
        batch_expiry_gap_when_init_usecs: u64,
        max_batches_per_pull: usize,
    ) -> Self {
        Self {
            my_peer_id,
            author_to_batches: HashMap::with_capacity(200),
            items: HashMap::with_capacity(50_000),
            txn_summary_num_occurrences: HashMap::new(),
            expirations: TimeExpirations::with_capacity(50_000),
            batch_store,
            latest_block_timestamp: 0,
            remaining_txns_with_duplicates: 0,
            remaining_proofs: 0,
            remaining_local_txns: 0,
            remaining_local_proofs: 0,
            batch_expiry_gap_when_init_usecs,
            max_batches_per_pull,
        }
    }

    #[inline]
    fn inc_remaining_proofs(&mut self, author: &PeerId, num_txns: u64) {
        self.remaining_txns_with_duplicates += num_txns;
        self.remaining_proofs += 1;
        if *author == self.my_peer_id {
            self.remaining_local_txns += num_txns;
            self.remaining_local_proofs += 1;
        }
    }

    #[inline]
    fn dec_remaining_proofs(&mut self, author: &PeerId, num_txns: u64) {
        self.remaining_txns_with_duplicates -= num_txns;
        self.remaining_proofs -= 1;
        if *author == self.my_peer_id {
            self.remaining_local_txns -= num_txns;
            self.remaining_local_proofs -= 1;
        }
    }

    #[cfg(test)]
    pub(crate) fn batch_summaries_len(&self) -> usize {
        self.items
            .iter()
            .filter(|(_, item)| item.txn_summaries.is_some())
            .count()
    }

    pub(crate) fn num_batches_without_proof(&self) -> usize {
        self.items
            .iter()
            .filter(|(_, item)| item.proof.is_none())
            .count()
    }

    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.items.is_empty()
            && self.author_to_batches.is_empty()
            && self.expirations.is_empty()
            && self.txn_summary_num_occurrences.is_empty()
    }

    fn remaining_txns_without_duplicates(&self) -> u64 {
        // txn_summary_num_occurrences counts all the unexpired and uncommitted proofs that have txn summaries
        // in batch_summaries.
        let mut remaining_txns = self.txn_summary_num_occurrences.len() as u64;

        // For the unexpired and uncommitted proofs that don't have transaction summaries in batch_summaries,
        // we need to add the proof.num_txns() to the remaining_txns.
        remaining_txns += self
            .author_to_batches
            .values()
            .map(|batches| {
                batches
                    .keys()
                    .map(|batch_sort_key| {
                        if let Some(item) = self.items.get(&batch_sort_key.batch_key) {
                            if item.txn_summaries.is_none() {
                                if let Some(ref proof) = item.proof {
                                    // The batch has a proof but not txn summaries
                                    return proof.num_txns();
                                }
                            }
                        }
                        0
                    })
                    .sum::<u64>()
            })
            .sum::<u64>();

        remaining_txns
    }

    /// Add the ProofOfStore to proof queue.
    pub(crate) fn insert_proof(&mut self, proof: ProofOfStore) {
        if proof.expiration() <= self.latest_block_timestamp {
            counters::inc_rejected_pos_count(counters::POS_EXPIRED_LABEL);
            return;
        }
        let batch_key = BatchKey::from_info(proof.info());
        if self
            .items
            .get(&batch_key)
            .is_some_and(|item| item.proof.is_some() || item.is_committed())
        {
            counters::inc_rejected_pos_count(counters::POS_DUPLICATE_LABEL);
            return;
        }

        let author = proof.author();
        let bucket = proof.gas_bucket_start();
        let num_txns = proof.num_txns();
        let expiration = proof.expiration();

        let batch_sort_key = BatchSortKey::from_info(proof.info());
        let batches_for_author = self.author_to_batches.entry(author).or_default();
        batches_for_author.insert(batch_sort_key.clone(), proof.info().clone());

        // Check if a batch with a higher batch Id (reverse sorted) exists
        // if let Some((prev_batch_key, _)) = batches_for_author
        //     .range((Bound::Unbounded, Bound::Excluded(batch_sort_key.clone())))
        //     .next_back()
        // {
        //     if prev_batch_key.gas_bucket_start() == batch_sort_key.gas_bucket_start() {
        //         counters::PROOF_MANAGER_OUT_OF_ORDER_PROOF_INSERTION
        //             .with_label_values(&[author.short_str().as_str()])
        //             .inc();
        //     }
        // }

        self.expirations.add_item(batch_sort_key, expiration);

        // If we are here, then proof is added for the first time. Otherwise, we will
        // return early. We only count when proof is added for the first time and txn
        // summary exists.
        if let Some(txn_summaries) = self
            .items
            .get(&batch_key)
            .and_then(|item| item.txn_summaries.as_ref())
        {
            for txn_summary in txn_summaries {
                *self
                    .txn_summary_num_occurrences
                    .entry(*txn_summary)
                    .or_insert(0) += 1;
            }
        }

        match self.items.entry(batch_key) {
            Entry::Occupied(mut entry) => {
                let item = entry.get_mut();
                item.proof = Some(proof);
                item.proof_insertion_time = Some(Instant::now());
            },
            Entry::Vacant(entry) => {
                entry.insert(QueueItem {
                    info: proof.info().clone(),
                    proof: Some(proof),
                    proof_insertion_time: Some(Instant::now()),
                    txn_summaries: None,
                    batch_insert_time: None,
                });
            },
        }

        if author == self.my_peer_id {
            counters::inc_local_pos_count(bucket);
        } else {
            counters::inc_remote_pos_count(bucket);
        }
        self.inc_remaining_proofs(&author, num_txns);

        // sample!(
        //     SampleRate::Duration(Duration::from_millis(500)),
        //     self.gc_expired_batch_summaries_without_proofs()
        // );
    }

    pub fn insert_batches(
        &mut self,
        batches_with_txn_summaries: Vec<(BatchInfo, Vec<TxnSummaryWithExpiration>)>,
    ) {
        let start = Instant::now();

        for (batch_info, txn_summaries) in batches_with_txn_summaries.into_iter() {
            let batch_sort_key = BatchSortKey::from_info(&batch_info);
            let batch_key = BatchKey::from_info(&batch_info);

            assert!(txn_summaries.is_empty());

            // If the batch is either committed or the txn summary already exists, skip
            // inserting this batch.
            if self
                .items
                .get(&batch_key)
                .is_some_and(|item| item.is_committed() || item.txn_summaries.is_some())
            {
                continue;
            }

            self.author_to_batches
                .entry(batch_info.author())
                .or_default()
                .insert(batch_sort_key.clone(), batch_info.clone());
            self.expirations
                .add_item(batch_sort_key, batch_info.expiration());

            // We only count txn summaries first time it is added to the queue
            // and only if the proof already exists.
            if self
                .items
                .get(&batch_key)
                .is_some_and(|item| item.proof.is_some())
            {
                for txn_summary in &txn_summaries {
                    *self
                        .txn_summary_num_occurrences
                        .entry(*txn_summary)
                        .or_insert(0) += 1;
                }
            }

            match self.items.entry(batch_key) {
                Entry::Occupied(mut entry) => {
                    entry.get_mut().txn_summaries = Some(txn_summaries);
                },
                Entry::Vacant(entry) => {
                    entry.insert(QueueItem {
                        info: batch_info,
                        proof: None,
                        proof_insertion_time: None,
                        txn_summaries: Some(txn_summaries),
                        batch_insert_time: Some(Instant::now()),
                    });
                },
            }
        }

        // sample!(
        //     SampleRate::Duration(Duration::from_millis(500)),
        //     self.gc_expired_batch_summaries_without_proofs()
        // );
        counters::PROOF_QUEUE_ADD_BATCH_SUMMARIES_DURATION.observe_duration(start.elapsed());
    }

    // If the validator receives the batch from batch coordinator, but doesn't receive the corresponding
    // proof before the batch expires, the batch summary will be garbage collected.
    fn gc_expired_batch_summaries_without_proofs(&mut self) {
        let timestamp = aptos_infallible::duration_since_epoch().as_micros() as u64;
        let mut count = 0;
        self.items.retain(|_, item| {
            if item.is_committed() || item.proof.is_some() || item.info.expiration() > timestamp {
                true
            } else {
                self.author_to_batches
                    .get_mut(&item.info.author())
                    .map(|queue| queue.remove(&BatchSortKey::from_info(&item.info)));
                count += 1;
                false
            }
        });
        counters::GARBAGE_COLLECTED_IN_PROOF_QUEUE_COUNTER
            .with_label_values(&["expired_batch_without_proof"])
            .inc_by(count);
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

        let remaining_batches = self
            .author_to_batches
            .iter()
            .flat_map(|(_, batches)| batches)
            .filter(|(batch_sort_key, _)| {
                !excluded_batch_keys.contains(&batch_sort_key.batch_key)
                    && !pulled_proofs
                        .iter()
                        .any(|p| BatchKey::from_info(p.info()) == batch_sort_key.batch_key)
            })
            .filter_map(|(batch_sort_key, batch)| {
                if self
                    .items
                    .get(&batch_sort_key.batch_key)
                    .is_some_and(|item| item.proof.is_some())
                {
                    Some(batch)
                } else {
                    None
                }
            });

        for batch in remaining_batches {
            num_proofs_remaining_after_pull += 1;
            num_txns_remaining_after_pull += batch.num_txns();
        }

        let pulled_txns = pulled_proofs.iter().map(|p| p.num_txns()).sum::<u64>();
        info!(
            "pulled_proofs: {}, pulled_txns: {}, remaining_proofs: {:?}, remaining_txns: {:?}",
            pulled_proofs.len(),
            pulled_txns,
            num_proofs_remaining_after_pull,
            num_txns_remaining_after_pull,
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
        max_txns: PayloadTxnsSize,
        max_txns_after_filtering: u64,
        soft_max_txns_after_filtering: u64,
        return_non_full: bool,
        block_timestamp: Duration,
    ) -> (Vec<ProofOfStore>, PayloadTxnsSize, u64, bool) {
        let (result, all_txns, unique_txns, is_full) = self.pull_internal(
            false,
            excluded_batches,
            &HashSet::new(),
            max_txns,
            max_txns_after_filtering,
            soft_max_txns_after_filtering,
            return_non_full,
            block_timestamp,
            None,
        );
        let proof_of_stores: Vec<_> = result
            .into_iter()
            .map(|item| {
                let proof = item.proof.clone().expect("proof must exist due to filter");
                let bucket = proof.gas_bucket_start();
                counters::pos_to_pull(
                    bucket,
                    item.proof_insertion_time
                        .expect("proof must exist due to filter")
                        .elapsed()
                        .as_secs_f64(),
                );
                proof
            })
            .collect();

        if is_full || return_non_full {
            counters::CONSENSUS_PULL_NUM_UNIQUE_TXNS.observe_with(&["proof"], unique_txns as f64);
            counters::CONSENSUS_PULL_NUM_TXNS.observe_with(&["proof"], all_txns.count() as f64);
            counters::CONSENSUS_PULL_SIZE_IN_BYTES
                .observe_with(&["proof"], all_txns.size_in_bytes() as f64);

            counters::BLOCK_SIZE_WHEN_PULL.observe(unique_txns as f64);
            counters::TOTAL_BLOCK_SIZE_WHEN_PULL.observe(all_txns.count() as f64);
            counters::KNOWN_DUPLICATE_TXNS_WHEN_PULL
                .observe((all_txns.count().saturating_sub(unique_txns)) as f64);
            counters::BLOCK_BYTES_WHEN_PULL.observe(all_txns.size_in_bytes() as f64);

            counters::PROOF_SIZE_WHEN_PULL.observe(proof_of_stores.len() as f64);
            // Number of proofs remaining in proof queue after the pull
            // self.log_remaining_data_after_pull(excluded_batches, &proof_of_stores);
        }

        (proof_of_stores, all_txns, unique_txns, !is_full)
    }

    pub fn pull_batches(
        &mut self,
        excluded_batches: &HashSet<BatchInfo>,
        exclude_authors: &HashSet<Author>,
        max_txns: PayloadTxnsSize,
        max_txns_after_filtering: u64,
        soft_max_txns_after_filtering: u64,
        return_non_full: bool,
        block_timestamp: Duration,
        minimum_batch_age_usecs: Option<u64>,
    ) -> (Vec<BatchInfo>, PayloadTxnsSize, u64) {
        let (result, pulled_txns, unique_txns, is_full) = self.pull_batches_internal(
            excluded_batches,
            exclude_authors,
            max_txns,
            max_txns_after_filtering,
            soft_max_txns_after_filtering,
            return_non_full,
            block_timestamp,
            minimum_batch_age_usecs,
        );

        if is_full || return_non_full {
            counters::CONSENSUS_PULL_NUM_UNIQUE_TXNS
                .observe_with(&["optbatch"], unique_txns as f64);
            counters::CONSENSUS_PULL_NUM_TXNS
                .observe_with(&["optbatch"], pulled_txns.count() as f64);
            counters::CONSENSUS_PULL_SIZE_IN_BYTES
                .observe_with(&["optbatch"], pulled_txns.size_in_bytes() as f64);
        }
        (result, pulled_txns, unique_txns)
    }

    pub fn pull_batches_internal(
        &mut self,
        excluded_batches: &HashSet<BatchInfo>,
        exclude_authors: &HashSet<Author>,
        max_txns: PayloadTxnsSize,
        max_txns_after_filtering: u64,
        soft_max_txns_after_filtering: u64,
        return_non_full: bool,
        block_timestamp: Duration,
        minimum_batch_age_usecs: Option<u64>,
    ) -> (Vec<BatchInfo>, PayloadTxnsSize, u64, bool) {
        let (result, all_txns, unique_txns, is_full) = self.pull_internal(
            true,
            excluded_batches,
            exclude_authors,
            max_txns,
            max_txns_after_filtering,
            soft_max_txns_after_filtering,
            return_non_full,
            block_timestamp,
            minimum_batch_age_usecs,
        );
        let batches = result
            .into_iter()
            .map(|item| {
                let bucket = item.info.gas_bucket_start;
                let duration_since_creation = duration_since_epoch().saturating_sub(
                    Duration::from_micros(item.info.expiration)
                        .saturating_sub(Duration::from_secs(60)),
                );
                counters::batch_to_pull(bucket, duration_since_creation.as_secs_f64());
                if let Some(instant) = item.batch_insert_time {
                    counters::batch_insert_to_pull(bucket, instant.elapsed().as_secs_f64());
                }
                item.info.clone()
            })
            .collect();
        (batches, all_txns, unique_txns, is_full)
    }

    pub fn pull_batches_with_transactions(
        &mut self,
        excluded_batches: &HashSet<BatchInfo>,
        max_txns: PayloadTxnsSize,
        max_txns_after_filtering: u64,
        soft_max_txns_after_filtering: u64,
        return_non_full: bool,
        block_timestamp: Duration,
    ) -> (
        Vec<(BatchInfo, Vec<SignedTransaction>)>,
        PayloadTxnsSize,
        u64,
    ) {
        let (batches, pulled_txns, unique_txns, is_full) = self.pull_batches_internal(
            excluded_batches,
            &HashSet::new(),
            max_txns,
            max_txns_after_filtering,
            soft_max_txns_after_filtering,
            return_non_full,
            block_timestamp,
            None,
        );
        let mut result = Vec::new();
        for batch in batches.into_iter() {
            if let Ok(mut persisted_value) = self.batch_store.get_batch_from_local(batch.digest()) {
                if let Some(txns) = persisted_value.take_payload() {
                    result.push((batch, txns));
                }
            } else {
                warn!(
                    "Couldn't find a batch in local storage while creating inline block: {:?}",
                    batch.digest()
                );
            }
        }

        if is_full || return_non_full {
            counters::CONSENSUS_PULL_NUM_UNIQUE_TXNS.observe_with(&["inline"], unique_txns as f64);
            counters::CONSENSUS_PULL_NUM_TXNS.observe_with(&["inline"], pulled_txns.count() as f64);
            counters::CONSENSUS_PULL_SIZE_IN_BYTES
                .observe_with(&["inline"], pulled_txns.size_in_bytes() as f64);
        }
        (result, pulled_txns, unique_txns)
    }

    fn pull_internal(
        &mut self,
        batches_without_proofs: bool,
        excluded_batches: &HashSet<BatchInfo>,
        exclude_authors: &HashSet<Author>,
        max_txns: PayloadTxnsSize,
        max_txns_after_filtering: u64,
        soft_max_txns_after_filtering: u64,
        return_non_full: bool,
        block_timestamp: Duration,
        min_batch_age_usecs: Option<u64>,
    ) -> (Vec<&QueueItem>, PayloadTxnsSize, u64, bool) {
        let mut result = Vec::new();
        let mut cur_unique_txns = 0;
        let mut cur_all_txns = PayloadTxnsSize::zero();
        let mut excluded_txns = 0;
        let mut full = false;
        // Set of all the excluded transactions and all the transactions included in the result
        let mut filtered_txns = HashSet::new();
        for batch_info in excluded_batches {
            let batch_key = BatchKey::from_info(batch_info);
            if let Some(txn_summaries) = self
                .items
                .get(&batch_key)
                .and_then(|item| item.txn_summaries.as_ref())
            {
                for txn_summary in txn_summaries {
                    filtered_txns.insert(*txn_summary);
                }
            }
        }

        let max_batch_creation_ts_usecs = min_batch_age_usecs
            .map(|min_age| aptos_infallible::duration_since_epoch().as_micros() as u64 - min_age);
        let mut iters = vec![];
        for (_, batches) in self
            .author_to_batches
            .iter()
            .filter(|(author, _)| !exclude_authors.contains(author))
        {
            let batch_iter = batches.iter().rev().filter_map(|(sort_key, info)| {
                if let Some(item) = self.items.get(&sort_key.batch_key) {
                    let batch_create_ts_usecs =
                        item.info.expiration() - self.batch_expiry_gap_when_init_usecs;

                    // Ensure that the batch was created at least `min_batch_age_usecs` ago to
                    // reduce the chance of inline fetches.
                    if max_batch_creation_ts_usecs
                        .is_some_and(|max_create_ts| batch_create_ts_usecs > max_create_ts)
                    {
                        return None;
                    }

                    if item.is_committed() {
                        return None;
                    }
                    if !(batches_without_proofs ^ item.proof.is_none()) {
                        return Some((info, item));
                    }
                }
                None
            });
            iters.push(batch_iter);
        }

        iters.shuffle(&mut thread_rng());
        while !iters.is_empty() {
            iters.retain_mut(|iter| {
                if full {
                    return false;
                }

                if let Some((batch, item)) = iter.next() {
                    if excluded_batches.contains(batch) {
                        excluded_txns += batch.num_txns();
                    } else {
                        // Calculate the number of unique transactions if this batch is included in the result
                        let unique_txns = if let Some(ref txn_summaries) = item.txn_summaries {
                            cur_unique_txns
                                + txn_summaries
                                    .iter()
                                    .filter(|txn_summary| {
                                        !filtered_txns.contains(txn_summary)
                                            && block_timestamp.as_secs()
                                                < txn_summary.expiration_timestamp_secs
                                    })
                                    .count() as u64
                        } else {
                            cur_unique_txns + batch.num_txns()
                        };
                        if cur_all_txns + batch.size() > max_txns
                            || unique_txns > max_txns_after_filtering
                            || result.len() > self.max_batches_per_pull
                        {
                            // Exceeded the limit for requested bytes or number of transactions.
                            full = true;
                            return false;
                        }
                        cur_all_txns += batch.size();
                        // Add this batch to filtered_txns and calculate the number of
                        // unique transactions added in the result so far.
                        cur_unique_txns +=
                            item.txn_summaries
                                .as_ref()
                                .map_or(batch.num_txns(), |summaries| {
                                    summaries
                                        .iter()
                                        .filter(|summary| {
                                            filtered_txns.insert(**summary)
                                                && block_timestamp.as_secs()
                                                    < summary.expiration_timestamp_secs
                                        })
                                        .count() as u64
                                });
                        assert!(item.proof.is_none() == batches_without_proofs);
                        result.push(item);
                        if cur_all_txns == max_txns
                            || cur_unique_txns == max_txns_after_filtering
                            || cur_unique_txns >= soft_max_txns_after_filtering
                            || result.len() > self.max_batches_per_pull
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
            block_total_txns = cur_all_txns,
            block_unique_txns = cur_unique_txns,
            max_txns = max_txns,
            max_txns_after_filtering = max_txns_after_filtering,
            soft_max_txns_after_filtering = soft_max_txns_after_filtering,
            max_bytes = max_txns.size_in_bytes(),
            result_is_proof = !batches_without_proofs,
            result_count = result.len(),
            full = full,
            return_non_full = return_non_full,
            "Pull payloads from QuorumStore: internal"
        );

        counters::EXCLUDED_TXNS_WHEN_PULL.observe(excluded_txns as f64);

        if full || return_non_full {
            // Stable sort, so the order of proofs within an author will not change.
            result.sort_by_key(|item| Reverse(item.info.gas_bucket_start()));
            (result, cur_all_txns, cur_unique_txns, full)
        } else {
            (Vec::new(), PayloadTxnsSize::zero(), 0, full)
        }
    }

    pub(crate) fn handle_updated_block_timestamp(&mut self, block_timestamp: u64) {
        // tolerate asynchronous notification
        if self.latest_block_timestamp > block_timestamp {
            return;
        }
        let start = Instant::now();
        self.latest_block_timestamp = block_timestamp;
        if let Some(time_lag) = aptos_infallible::duration_since_epoch()
            .checked_sub(Duration::from_micros(block_timestamp))
        {
            counters::TIME_LAG_IN_BATCH_PROOF_QUEUE.observe_duration(time_lag);
        }

        let expired = self.expirations.expire(block_timestamp);
        let mut num_expired_but_not_committed = 0;
        for key in &expired {
            if let Some(mut queue) = self.author_to_batches.remove(&key.author()) {
                if let Some(batch) = queue.remove(key) {
                    let item = self
                        .items
                        .get(&key.batch_key)
                        .expect("Entry for unexpired batch must exist");
                    if item.proof.is_some() {
                        // not committed proof that is expired
                        num_expired_but_not_committed += 1;
                        counters::GAP_BETWEEN_BATCH_EXPIRATION_AND_CURRENT_TIME_WHEN_COMMIT
                            .observe((block_timestamp - batch.expiration()) as f64);
                        if let Some(ref txn_summaries) = item.txn_summaries {
                            for txn_summary in txn_summaries {
                                if let Some(count) =
                                    self.txn_summary_num_occurrences.get_mut(txn_summary)
                                {
                                    *count -= 1;
                                    if *count == 0 {
                                        self.txn_summary_num_occurrences.remove(txn_summary);
                                    }
                                };
                            }
                        }
                        self.dec_remaining_proofs(&batch.author(), batch.num_txns());
                        counters::GARBAGE_COLLECTED_IN_PROOF_QUEUE_COUNTER
                            .with_label_values(&["expired_proof"])
                            .inc();
                    }
                    claims::assert_some!(self.items.remove(&key.batch_key));
                }
                if !queue.is_empty() {
                    self.author_to_batches.insert(key.author(), queue);
                }
            }
        }
        counters::PROOF_QUEUE_UPDATE_TIMESTAMP_DURATION.observe_duration(start.elapsed());
        counters::NUM_PROOFS_EXPIRED_WHEN_COMMIT.inc_by(num_expired_but_not_committed);
    }

    // Number of unexpired and uncommitted proofs in the pipeline without txn summaries in
    // batch_summaries
    fn num_proofs_without_batch_summary(&self) -> u64 {
        let mut count = 0;
        self.author_to_batches.values().for_each(|batches| {
            count += batches
                .iter()
                .filter(|(sort_key, _)| {
                    self.items.get(&sort_key.batch_key).map_or(false, |item| {
                        item.proof.is_some() && item.txn_summaries.is_none()
                    })
                })
                .count() as u64;
        });
        count
    }

    // Number of unexpired and uncommitted proofs in the pipeline with txn summaries in
    // batch_summaries
    fn num_proofs_with_batch_summary(&self) -> u64 {
        let mut count = 0;
        self.author_to_batches.values().for_each(|batches| {
            count += batches
                .iter()
                .filter(|(sort_key, _)| {
                    self.items.get(&sort_key.batch_key).map_or(false, |item| {
                        item.proof.is_some() && item.txn_summaries.is_some()
                    })
                })
                .count() as u64;
        });
        count
    }

    #[cfg(test)]
    pub(crate) fn remaining_txns_and_proofs(&self) -> (u64, u64) {
        let start = Instant::now();
        counters::NUM_TOTAL_TXNS_LEFT_ON_UPDATE.observe(self.remaining_txns_with_duplicates as f64);
        counters::NUM_TOTAL_PROOFS_LEFT_ON_UPDATE.observe(self.remaining_proofs as f64);
        counters::NUM_LOCAL_TXNS_LEFT_ON_UPDATE.observe(self.remaining_local_txns as f64);
        counters::NUM_LOCAL_PROOFS_LEFT_ON_UPDATE.observe(self.remaining_local_proofs as f64);

        let remaining_txns_without_duplicates = self.remaining_txns_without_duplicates();
        counters::NUM_UNIQUE_TOTAL_TXNS_LEFT_ON_UPDATE
            .observe(remaining_txns_without_duplicates as f64);

        // Number of txns with more than one batches
        sample!(
            SampleRate::Duration(Duration::from_secs(3)),
            counters::TXNS_WITH_DUPLICATE_BATCHES.observe(
                self.txn_summary_num_occurrences
                    .iter()
                    .filter(|(_, count)| **count > 1)
                    .count() as f64,
            );
        );

        // Number of txns in unexpired and uncommitted proofs with summaries in batch_summaries
        counters::TXNS_IN_PROOFS_WITH_SUMMARIES
            .observe(self.txn_summary_num_occurrences.len() as f64);

        // Number of txns in unexpired and uncommitted proofs without summaries in batch_summaries
        counters::TXNS_IN_PROOFS_WITHOUT_SUMMARIES.observe(
            remaining_txns_without_duplicates
                .saturating_sub(self.txn_summary_num_occurrences.len() as u64) as f64,
        );

        counters::PROOFS_WITHOUT_BATCH_SUMMARY
            .observe(self.num_proofs_without_batch_summary() as f64);
        counters::PROOFS_WITH_BATCH_SUMMARY.observe(self.num_proofs_with_batch_summary() as f64);

        counters::PROOF_QUEUE_REMAINING_TXNS_DURATION.observe_duration(start.elapsed());
        (remaining_txns_without_duplicates, self.remaining_proofs)
    }

    // Mark in the hashmap committed PoS, but keep them until they expire
    pub(crate) fn mark_committed(&mut self, batches: Vec<BatchInfo>) {
        let start = Instant::now();
        for batch in batches.into_iter() {
            let batch_key = BatchKey::from_info(&batch);
            if let Some(item) = self.items.get(&batch_key) {
                if let Some(ref proof) = item.proof {
                    let insertion_time = item
                        .proof_insertion_time
                        .expect("Insertion time is updated with proof");
                    counters::pos_to_commit(
                        proof.gas_bucket_start(),
                        insertion_time.elapsed().as_secs_f64(),
                    );
                    self.dec_remaining_proofs(&batch.author(), batch.num_txns());
                    counters::GARBAGE_COLLECTED_IN_PROOF_QUEUE_COUNTER
                        .with_label_values(&["committed_proof"])
                        .inc();
                }
                let item = self
                    .items
                    .get_mut(&batch_key)
                    .expect("must exist due to check");

                if item.proof.is_some() {
                    if let Some(ref txn_summaries) = item.txn_summaries {
                        for txn_summary in txn_summaries {
                            if let Some(count) =
                                self.txn_summary_num_occurrences.get_mut(txn_summary)
                            {
                                *count -= 1;
                                if *count == 0 {
                                    self.txn_summary_num_occurrences.remove(txn_summary);
                                }
                            };
                        }
                    }
                } else if !item.is_committed() {
                    counters::GARBAGE_COLLECTED_IN_PROOF_QUEUE_COUNTER
                        .with_label_values(&["committed_batch_without_proof"])
                        .inc();
                }
                // The item is just marked committed for now.
                // When the batch is expired, then it will be removed from items.
                item.mark_committed();
            } else {
                let batch_sort_key = BatchSortKey::from_info(batch.info());
                self.expirations
                    .add_item(batch_sort_key.clone(), batch.expiration());
                self.author_to_batches
                    .entry(batch.author())
                    .or_default()
                    .insert(batch_sort_key, batch.clone());
                self.items.insert(batch_key, QueueItem {
                    info: batch,
                    txn_summaries: None,
                    proof: None,
                    proof_insertion_time: None,
                    batch_insert_time: None,
                });
            }
        }
        counters::PROOF_QUEUE_COMMIT_DURATION.observe_duration(start.elapsed());
    }
}
