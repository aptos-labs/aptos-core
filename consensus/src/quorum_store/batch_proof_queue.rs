// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::{
    batch_store::BatchStore,
    utils::{BatchKey, BatchSortKey, PullKey, TimeExpirations},
};
use crate::quorum_store::counters;
#[cfg(test)]
use aptos_consensus_types::common::Author;
use aptos_consensus_types::{
    common::TxnSummaryWithExpiration,
    proof_of_store::{BatchInfoExt, ProofOfStore, TBatchInfo},
    utils::PayloadTxnsSize,
};
#[cfg(test)]
use aptos_logger::warn;
use aptos_logger::{debug, sample, sample::SampleRate};
#[cfg(test)]
use aptos_metrics_core::TimerHelper;
use aptos_short_hex_str::AsShortHexStr;
#[cfg(test)]
use aptos_types::transaction::SignedTransaction;
use aptos_types::PeerId;
#[cfg(test)]
use rand::prelude::SliceRandom;
use rand::{rngs::SmallRng, thread_rng, Rng, SeedableRng};
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
pub(crate) struct QueueItem {
    /// The info of the Batch this item stores
    pub(crate) info: BatchInfoExt,
    /// Contains the summary of transactions in the batch.
    /// It is optional as the summary can be updated after the proof.
    pub(crate) txn_summaries: Option<Vec<TxnSummaryWithExpiration>>,

    /// Contains the proof associated with the batch.
    /// It is optional as the proof can be updated after the summary.
    pub(crate) proof: Option<ProofOfStore<BatchInfoExt>>,
    /// The time when the proof is inserted into this item.
    pub(crate) proof_insertion_time: Option<Instant>,
    /// The time when the batch (txn summaries) is first inserted into this item.
    pub(crate) batch_insertion_time: Option<Instant>,
}

impl QueueItem {
    fn is_committed(&self) -> bool {
        self.proof.is_none()
            && self.proof_insertion_time.is_none()
            && self.txn_summaries.is_none()
            && self.batch_insertion_time.is_none()
    }

    fn mark_committed(&mut self) {
        self.proof = None;
        self.proof_insertion_time = None;
        self.txn_summaries = None;
        self.batch_insertion_time = None;
    }
}

/// Owned data extracted from a QueueItem during pull_all, avoiding borrow issues.
pub(crate) struct PulledItem {
    pub(crate) info: BatchInfoExt,
    pub(crate) proof: Option<ProofOfStore<BatchInfoExt>>,
    pub(crate) proof_insertion_time: Option<Instant>,
}

pub struct BatchProofQueue {
    my_peer_id: PeerId,
    // Queue per peer to ensure fairness between peers and priority within peer
    author_to_batches: HashMap<PeerId, BTreeMap<BatchSortKey, BatchInfoExt>>,
    // Map of Batch key to QueueItem containing Batch data and proofs
    items: HashMap<BatchKey, QueueItem>,
    // Number of unexpired and uncommitted proofs in which the txn_summary = (sender, replay protector, hash, expiration)
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
    num_items_without_proof: usize,

    batch_expiry_gap_when_init_usecs: u64,

    // Pre-sorted pull candidate maps (non-committed, non-expired items only)
    proof_candidates: BTreeMap<PullKey, BatchKey>,
    batch_candidates: BTreeMap<PullKey, BatchKey>,
    // Reverse map for O(log n) removal from candidate maps; bool = true if in proof_candidates
    batch_key_to_pull_key: HashMap<BatchKey, (PullKey, bool)>,
    // Per-author random tiebreaker (stable "seat number" for fair interleaving)
    author_tiebreakers: HashMap<PeerId, u64>,
    // RNG for generating author tiebreakers
    tiebreaker_rng: SmallRng,

    // Incremental counters for O(1) remaining_txns_and_proofs
    proofs_with_summary: u64,
    proofs_without_summary: u64,
    txns_in_proofs_without_summary: u64,
}

impl BatchProofQueue {
    pub(crate) fn new(
        my_peer_id: PeerId,
        batch_store: Arc<BatchStore>,
        batch_expiry_gap_when_init_usecs: u64,
    ) -> Self {
        Self {
            my_peer_id,
            author_to_batches: HashMap::new(),
            items: HashMap::new(),
            txn_summary_num_occurrences: HashMap::new(),
            expirations: TimeExpirations::new(),
            batch_store,
            latest_block_timestamp: 0,
            remaining_txns_with_duplicates: 0,
            remaining_proofs: 0,
            remaining_local_txns: 0,
            remaining_local_proofs: 0,
            num_items_without_proof: 0,
            batch_expiry_gap_when_init_usecs,
            proof_candidates: BTreeMap::new(),
            batch_candidates: BTreeMap::new(),
            batch_key_to_pull_key: HashMap::new(),
            author_tiebreakers: HashMap::new(),
            tiebreaker_rng: SmallRng::from_rng(&mut thread_rng())
                .unwrap_or_else(|_| SmallRng::seed_from_u64(0)),
            proofs_with_summary: 0,
            proofs_without_summary: 0,
            txns_in_proofs_without_summary: 0,
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

    fn get_or_create_tiebreaker(&mut self, author: PeerId) -> u64 {
        *self
            .author_tiebreakers
            .entry(author)
            .or_insert_with(|| self.tiebreaker_rng.r#gen())
    }

    fn make_pull_key(&mut self, info: &BatchInfoExt) -> PullKey {
        let author = info.author();
        let gas_bucket = info.gas_bucket_start();
        let tiebreaker = self.get_or_create_tiebreaker(author);
        PullKey {
            gas_priority: Reverse(gas_bucket),
            author_tiebreaker: tiebreaker,
            batch_sort_key_rev: Reverse(BatchSortKey::from_info(info)),
        }
    }

    fn remove_from_candidates(&mut self, batch_key: &BatchKey) {
        if let Some((pull_key, is_proof)) = self.batch_key_to_pull_key.remove(batch_key) {
            let map = if is_proof {
                &mut self.proof_candidates
            } else {
                &mut self.batch_candidates
            };
            map.remove(&pull_key);
        }
    }

    #[cfg(test)]
    pub(crate) fn batch_summaries_len(&self) -> usize {
        self.items
            .iter()
            .filter(|(_, item)| item.txn_summaries.is_some())
            .count()
    }

    pub(crate) fn batch_expiry_gap_when_init_usecs(&self) -> u64 {
        self.batch_expiry_gap_when_init_usecs
    }

    pub(crate) fn batch_store(&self) -> &Arc<BatchStore> {
        &self.batch_store
    }

    pub(crate) fn num_batches_without_proof(&self) -> usize {
        debug_assert_eq!(
            self.num_items_without_proof,
            self.items
                .iter()
                .filter(|(_, item)| item.proof.is_none() && !item.is_committed())
                .count(),
            "incremental num_items_without_proof diverged from ground truth"
        );
        self.num_items_without_proof
    }

    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.items.is_empty()
            && self.author_to_batches.is_empty()
            && self.expirations.is_empty()
            && self.txn_summary_num_occurrences.is_empty()
    }

    /// Add the ProofOfStore to proof queue.
    pub(crate) fn insert_proof(&mut self, proof: ProofOfStore<BatchInfoExt>) {
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
        let batch_version = if proof.info().is_v2() { "v2" } else { "v1" };

        let batch_sort_key = BatchSortKey::from_info(proof.info());
        let batches_for_author = self.author_to_batches.entry(author).or_default();
        batches_for_author.insert(batch_sort_key.clone(), proof.info().clone());

        // Check if a batch with a higher batch Id (reverse sorted) exists
        if let Some((prev_batch_key, _)) = batches_for_author
            .range((Bound::Unbounded, Bound::Excluded(batch_sort_key.clone())))
            .next_back()
        {
            if prev_batch_key.gas_bucket_start() == batch_sort_key.gas_bucket_start() {
                counters::PROOF_MANAGER_OUT_OF_ORDER_PROOF_INSERTION
                    .with_label_values(&[author.short_str().as_str()])
                    .inc();
            }
        }

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

        let proof_insertion_now = Instant::now();
        let has_txn_summaries = self
            .items
            .get(&batch_key)
            .is_some_and(|item| item.txn_summaries.is_some());
        let was_in_batch_candidates = self
            .batch_key_to_pull_key
            .get(&batch_key)
            .is_some_and(|(_, is_proof)| !is_proof);

        // Maintain candidate maps: re-key from batch_candidates to proof_candidates, or insert new
        if was_in_batch_candidates {
            // Re-key: remove from batch_candidates, add to proof_candidates
            self.remove_from_candidates(&batch_key);
        }
        let pull_key = self.make_pull_key(proof.info());
        self.proof_candidates
            .insert(pull_key.clone(), batch_key.clone());
        self.batch_key_to_pull_key
            .insert(batch_key.clone(), (pull_key, true));

        // Update incremental counters
        if has_txn_summaries {
            self.proofs_with_summary += 1;
        } else {
            self.proofs_without_summary += 1;
            self.txns_in_proofs_without_summary += num_txns;
        }

        match self.items.entry(batch_key) {
            Entry::Occupied(mut entry) => {
                let item = entry.get_mut();
                // Record proof delay relative to batch insertion (metric 4)
                if item.txn_summaries.is_some() {
                    if let Some(batch_time) = item.batch_insertion_time {
                        counters::PROOF_DELAY_AFTER_BATCH
                            .with_label_values(&[author.short_str().as_str()])
                            .observe(
                                proof_insertion_now.duration_since(batch_time).as_secs_f64()
                                    * 1000.0,
                            );
                    }
                }
                // Item existed without proof, now gaining one
                if item.proof.is_none() && !item.is_committed() {
                    self.num_items_without_proof -= 1;
                }
                item.proof = Some(proof);
                item.proof_insertion_time = Some(proof_insertion_now);
            },
            Entry::Vacant(entry) => {
                entry.insert(QueueItem {
                    info: proof.info().clone(),
                    proof: Some(proof),
                    proof_insertion_time: Some(proof_insertion_now),
                    txn_summaries: None,
                    batch_insertion_time: None,
                });
                // New item with proof — no increment needed
            },
        }

        if author == self.my_peer_id {
            counters::inc_local_pos_count(bucket, batch_version);
        } else {
            counters::inc_remote_pos_count(bucket, batch_version);
        }
        self.inc_remaining_proofs(&author, num_txns);

        sample!(
            SampleRate::Duration(Duration::from_millis(500)),
            self.gc_expired_batch_summaries_without_proofs()
        );
    }

    pub fn insert_batches(
        &mut self,
        batches_with_txn_summaries: Vec<(BatchInfoExt, Vec<TxnSummaryWithExpiration>)>,
    ) {
        let start = Instant::now();

        for (batch_info, txn_summaries) in batches_with_txn_summaries.into_iter() {
            let batch_sort_key = BatchSortKey::from_info(&batch_info);
            let batch_key = BatchKey::from_info(&batch_info);

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
            let has_proof = self
                .items
                .get(&batch_key)
                .is_some_and(|item| item.proof.is_some());
            if has_proof {
                for txn_summary in &txn_summaries {
                    *self
                        .txn_summary_num_occurrences
                        .entry(*txn_summary)
                        .or_insert(0) += 1;
                }
                // Update incremental counters: proof existed without summary, now has summary
                self.proofs_without_summary -= 1;
                let num_txns = batch_info.num_txns();
                self.txns_in_proofs_without_summary -= num_txns;
                self.proofs_with_summary += 1;
            }

            let already_exists = self.items.contains_key(&batch_key);

            match self.items.entry(batch_key.clone()) {
                Entry::Occupied(mut entry) => {
                    let item = entry.get_mut();
                    item.txn_summaries = Some(txn_summaries);
                    if item.batch_insertion_time.is_none() {
                        item.batch_insertion_time = Some(Instant::now());
                    }
                },
                Entry::Vacant(entry) => {
                    entry.insert(QueueItem {
                        info: batch_info.clone(),
                        proof: None,
                        proof_insertion_time: None,
                        txn_summaries: Some(txn_summaries),
                        batch_insertion_time: Some(Instant::now()),
                    });
                    // New item without proof
                    self.num_items_without_proof += 1;
                },
            }

            // Maintain candidate maps: only add to batch_candidates if item is new (no proof yet)
            if !already_exists {
                let pull_key = self.make_pull_key(&batch_info);
                self.batch_candidates
                    .insert(pull_key.clone(), batch_key.clone());
                self.batch_key_to_pull_key
                    .insert(batch_key, (pull_key, false));
            }
            // If item already exists with proof and is in proof_candidates: no PullKey change needed
        }

        sample!(
            SampleRate::Duration(Duration::from_millis(500)),
            self.gc_expired_batch_summaries_without_proofs()
        );
        counters::PROOF_QUEUE_ADD_BATCH_SUMMARIES_DURATION.observe_duration(start.elapsed());
    }

    // If the validator receives the batch from batch coordinator, but doesn't receive the corresponding
    // proof before the batch expires, the batch summary will be garbage collected.
    fn gc_expired_batch_summaries_without_proofs(&mut self) {
        let timestamp = aptos_infallible::duration_since_epoch().as_micros() as u64;
        // Collect keys to remove first, then remove them
        let keys_to_remove: Vec<BatchKey> = self
            .items
            .iter()
            .filter(|(_, item)| {
                !item.is_committed() && item.proof.is_none() && item.info.expiration() <= timestamp
            })
            .map(|(key, _)| key.clone())
            .collect();

        for batch_key in keys_to_remove {
            if let Some(item) = self.items.remove(&batch_key) {
                self.num_items_without_proof -= 1;
                self.author_to_batches
                    .get_mut(&item.info.author())
                    .map(|queue| queue.remove(&BatchSortKey::from_info(&item.info)));
                self.remove_from_candidates(&batch_key);
                counters::GARBAGE_COLLECTED_IN_PROOF_QUEUE_COUNTER
                    .with_label_values(&["expired_batch_without_proof"])
                    .inc();
            }
        }
    }

    fn log_remaining_data_after_pull(
        &self,
        excluded_batches: &HashSet<BatchInfoExt>,
        pulled_proofs: &[ProofOfStore<BatchInfoExt>],
    ) {
        let mut num_proofs_remaining_after_pull = 0;
        let mut num_txns_remaining_after_pull = 0;
        let mut excluded_batch_keys: HashSet<BatchKey> =
            excluded_batches.iter().map(BatchKey::from_info).collect();
        for p in pulled_proofs {
            excluded_batch_keys.insert(BatchKey::from_info(p.info()));
        }

        let remaining_batches = self
            .author_to_batches
            .iter()
            .flat_map(|(_, batches)| batches)
            .filter(|(batch_sort_key, _)| !excluded_batch_keys.contains(&batch_sort_key.batch_key))
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
        debug!(
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

    /// Combined pull that retrieves all eligible items (both with and without proofs) in a single
    /// pass using pre-sorted candidate BTreeMaps. Iterates proof_candidates first (all proofs
    /// before any non-proof items), then batch_candidates. Items come out in priority order
    /// with fairness encoded at insertion time — no shuffle or post-sort needed.
    /// Returns (items, all_txns_size, unique_txns, is_full).
    pub(crate) fn pull_all(
        &mut self,
        excluded_batches: &HashSet<BatchInfoExt>,
        max_txns: PayloadTxnsSize,
        max_txns_after_filtering: u64,
        soft_max_txns_after_filtering: u64,
        return_non_full: bool,
        block_timestamp: Duration,
    ) -> (Vec<PulledItem>, PayloadTxnsSize, u64, bool) {
        let mut result = Vec::new();
        let mut cur_unique_txns = 0;
        let mut cur_all_txns = PayloadTxnsSize::zero();
        let mut excluded_txns_count = 0;
        let mut full = false;

        // Build excluded_txns set once from excluded_batches (immutable during loop)
        let estimated_txns: usize = excluded_batches.iter().map(|b| b.num_txns() as usize).sum();
        let mut excluded_txns: HashSet<TxnSummaryWithExpiration> =
            HashSet::with_capacity(estimated_txns);
        for batch_info in excluded_batches {
            let batch_key = BatchKey::from_info(batch_info);
            if let Some(txn_summaries) = self
                .items
                .get(&batch_key)
                .and_then(|item| item.txn_summaries.as_ref())
            {
                for txn_summary in txn_summaries {
                    excluded_txns.insert(*txn_summary);
                }
            }
        }

        // filtered_txns only needed for multi-occurrence txns (small capacity)
        let mut filtered_txns: HashSet<TxnSummaryWithExpiration> =
            HashSet::with_capacity(max_txns_after_filtering as usize / 5);

        let block_timestamp_secs = block_timestamp.as_secs();

        // Helper macro to process a single candidate item
        macro_rules! process_candidate {
            ($batch_key:expr) => {
                let item = match self.items.get($batch_key) {
                    Some(item) => item,
                    None => continue,
                };
                let batch = &item.info;
                if excluded_batches.contains(batch) {
                    excluded_txns_count += batch.num_txns();
                    continue;
                }
                // Two-tier unique txn counting
                let new_unique = if let Some(ref txn_summaries) = item.txn_summaries {
                    txn_summaries
                        .iter()
                        .filter(|s| {
                            block_timestamp_secs < s.expiration_timestamp_secs
                                && !excluded_txns.contains(s)
                                && (self
                                    .txn_summary_num_occurrences
                                    .get(s)
                                    .copied()
                                    .unwrap_or(0)
                                    <= 1
                                    || filtered_txns.insert(**s))
                        })
                        .count() as u64
                } else {
                    batch.num_txns()
                };
                let unique_txns = cur_unique_txns + new_unique;
                if cur_all_txns + batch.size() > max_txns || unique_txns > max_txns_after_filtering
                {
                    full = true;
                    break;
                }
                cur_all_txns += batch.size();
                cur_unique_txns = unique_txns;
                result.push(PulledItem {
                    info: item.info.clone(),
                    proof: item.proof.clone(),
                    proof_insertion_time: item.proof_insertion_time,
                });
                if cur_all_txns == max_txns
                    || cur_unique_txns == max_txns_after_filtering
                    || cur_unique_txns >= soft_max_txns_after_filtering
                {
                    full = true;
                    break;
                }
            };
        }

        // Iterate proof_candidates first (all proofs before any batches)
        for (_, batch_key) in &self.proof_candidates {
            process_candidate!(batch_key);
        }

        // Then iterate batch_candidates (items without proofs)
        if !full {
            for (_, batch_key) in &self.batch_candidates {
                process_candidate!(batch_key);
            }
        }

        debug!(
            block_total_txns = cur_all_txns,
            block_unique_txns = cur_unique_txns,
            max_txns = max_txns,
            max_txns_after_filtering = max_txns_after_filtering,
            soft_max_txns_after_filtering = soft_max_txns_after_filtering,
            max_bytes = max_txns.size_in_bytes(),
            result_count = result.len(),
            full = full,
            return_non_full = return_non_full,
            "Pull payloads from QuorumStore: pull_all"
        );

        counters::EXCLUDED_TXNS_WHEN_PULL.observe(excluded_txns_count as f64);

        if full || return_non_full {
            // No sort needed — items come out in priority order from BTreeMaps

            // Batch per-author metrics
            let now_usecs = aptos_infallible::duration_since_epoch().as_micros() as u64;
            let mut author_stats: HashMap<PeerId, (u64, f64, f64)> = HashMap::new();
            for item in &result {
                let author = item.info.author();
                let entry = author_stats.entry(author).or_insert((0, 0.0, 0.0));
                entry.0 += 1;
                let batch_create_ts_usecs = item
                    .info
                    .expiration()
                    .saturating_sub(self.batch_expiry_gap_when_init_usecs);
                entry.1 += now_usecs.saturating_sub(batch_create_ts_usecs) as f64 / 1_000.0;
            }
            for (author, (count, total_age_ms, _total_queue_ms)) in &author_stats {
                let author_str = author.short_str();
                let author_label = author_str.as_str();
                counters::BATCH_PULLED_BY_AUTHOR
                    .with_label_values(&[author_label, "pull_all"])
                    .inc_by(*count);
                counters::BATCH_AGE_WHEN_PULLED
                    .with_label_values(&[author_label, "pull_all"])
                    .observe(*total_age_ms / *count as f64);
            }

            // Record proof-specific pull metrics
            let num_proofs = result.iter().filter(|item| item.proof.is_some()).count();
            counters::PROOF_SIZE_WHEN_PULL.observe(num_proofs as f64);
            counters::BLOCK_SIZE_WHEN_PULL.observe(cur_unique_txns as f64);
            counters::TOTAL_BLOCK_SIZE_WHEN_PULL.observe(cur_all_txns.count() as f64);
            counters::KNOWN_DUPLICATE_TXNS_WHEN_PULL
                .observe((cur_all_txns.count().saturating_sub(cur_unique_txns)) as f64);
            counters::BLOCK_BYTES_WHEN_PULL.observe(cur_all_txns.size_in_bytes() as f64);

            // Log remaining data using proof items
            let pulled_proofs: Vec<_> = result
                .iter()
                .filter_map(|item| item.proof.clone())
                .collect();
            self.log_remaining_data_after_pull(excluded_batches, &pulled_proofs);

            (result, cur_all_txns, cur_unique_txns, full)
        } else {
            (Vec::new(), PayloadTxnsSize::zero(), 0, full)
        }
    }

    // gets excluded and iterates over the vector returning non excluded or expired entries.
    // return the vector of pulled PoS, and the size of the remaining PoS
    // The flag in the second return argument is true iff the entire proof queue is fully utilized
    // when pulling the proofs. If any proof from proof queue cannot be included due to size limits,
    // this flag is set false.
    // Returns the proofs, the number of unique transactions in the proofs, and a flag indicating
    // whether the proof queue is fully utilized.
    #[cfg(test)]
    pub(crate) fn pull_proofs(
        &mut self,
        excluded_batches: &HashSet<BatchInfoExt>,
        max_txns: PayloadTxnsSize,
        max_txns_after_filtering: u64,
        soft_max_txns_after_filtering: u64,
        return_non_full: bool,
        block_timestamp: Duration,
    ) -> (Vec<ProofOfStore<BatchInfoExt>>, PayloadTxnsSize, u64, bool) {
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
            "proof",
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
            self.log_remaining_data_after_pull(excluded_batches, &proof_of_stores);
        }

        (proof_of_stores, all_txns, unique_txns, !is_full)
    }

    #[cfg(test)]
    pub fn pull_batches(
        &mut self,
        excluded_batches: &HashSet<BatchInfoExt>,
        exclude_authors: &HashSet<Author>,
        max_txns: PayloadTxnsSize,
        max_txns_after_filtering: u64,
        soft_max_txns_after_filtering: u64,
        return_non_full: bool,
        block_timestamp: Duration,
        minimum_batch_age_usecs: Option<u64>,
    ) -> (Vec<BatchInfoExt>, PayloadTxnsSize, u64) {
        let (result, pulled_txns, unique_txns, is_full) = self.pull_batches_internal(
            excluded_batches,
            exclude_authors,
            max_txns,
            max_txns_after_filtering,
            soft_max_txns_after_filtering,
            return_non_full,
            block_timestamp,
            minimum_batch_age_usecs,
            "optbatch",
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

    #[cfg(test)]
    fn pull_batches_internal(
        &mut self,
        excluded_batches: &HashSet<BatchInfoExt>,
        exclude_authors: &HashSet<Author>,
        max_txns: PayloadTxnsSize,
        max_txns_after_filtering: u64,
        soft_max_txns_after_filtering: u64,
        return_non_full: bool,
        block_timestamp: Duration,
        minimum_batch_age_usecs: Option<u64>,
        pull_kind: &str,
    ) -> (Vec<BatchInfoExt>, PayloadTxnsSize, u64, bool) {
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
            pull_kind,
        );
        let batches = result.into_iter().map(|item| item.info.clone()).collect();
        (batches, all_txns, unique_txns, is_full)
    }

    #[cfg(test)]
    pub fn pull_batches_with_transactions(
        &mut self,
        excluded_batches: &HashSet<BatchInfoExt>,
        max_txns: PayloadTxnsSize,
        max_txns_after_filtering: u64,
        soft_max_txns_after_filtering: u64,
        return_non_full: bool,
        block_timestamp: Duration,
    ) -> (
        Vec<(BatchInfoExt, Vec<SignedTransaction>)>,
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
            "inline",
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

    #[cfg(test)]
    fn pull_internal(
        &mut self,
        batches_without_proofs: bool,
        excluded_batches: &HashSet<BatchInfoExt>,
        exclude_authors: &HashSet<Author>,
        max_txns: PayloadTxnsSize,
        max_txns_after_filtering: u64,
        soft_max_txns_after_filtering: u64,
        return_non_full: bool,
        block_timestamp: Duration,
        min_batch_age_usecs: Option<u64>,
        pull_kind: &str,
    ) -> (Vec<&QueueItem>, PayloadTxnsSize, u64, bool) {
        let mut result = Vec::new();
        let mut cur_unique_txns = 0;
        let mut cur_all_txns = PayloadTxnsSize::zero();
        let mut excluded_txns = 0;
        let mut full = false;
        // Pre-size filtered_txns based on excluded batches
        let estimated_txns: usize = excluded_batches.iter().map(|b| b.num_txns() as usize).sum();
        let mut filtered_txns =
            HashSet::with_capacity(estimated_txns + max_txns_after_filtering as usize);
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

        let block_timestamp_secs = block_timestamp.as_secs();
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
                    if item.is_committed() {
                        return None;
                    }
                    if batches_without_proofs ^ item.proof.is_none() {
                        return None;
                    }

                    let batch_create_ts_usecs = item
                        .info
                        .expiration()
                        .saturating_sub(self.batch_expiry_gap_when_init_usecs);

                    // Ensure that the batch was created at least `min_batch_age_usecs` ago to
                    // reduce the chance of inline fetches.
                    if max_batch_creation_ts_usecs
                        .is_some_and(|max_create_ts| batch_create_ts_usecs > max_create_ts)
                    {
                        counters::BATCH_SKIPPED_TOO_YOUNG
                            .with_label_values(&[item.info.author().short_str().as_str()])
                            .inc();
                        return None;
                    }

                    return Some((info, item));
                }
                None
            });
            iters.push(batch_iter);
        }

        let mut rng =
            SmallRng::from_rng(&mut thread_rng()).unwrap_or_else(|_| SmallRng::seed_from_u64(0));
        'outer: loop {
            iters.shuffle(&mut rng);
            let mut any_progress = false;
            let mut i = 0;
            while i < iters.len() {
                if let Some((batch, item)) = iters[i].next() {
                    any_progress = true;
                    if excluded_batches.contains(batch) {
                        excluded_txns += batch.num_txns();
                    } else {
                        // Single-pass: use insert() which returns true for new entries
                        let new_unique = if let Some(ref txn_summaries) = item.txn_summaries {
                            txn_summaries
                                .iter()
                                .filter(|s| {
                                    filtered_txns.insert(**s)
                                        && block_timestamp_secs < s.expiration_timestamp_secs
                                })
                                .count() as u64
                        } else {
                            batch.num_txns()
                        };
                        let unique_txns = cur_unique_txns + new_unique;
                        if cur_all_txns + batch.size() > max_txns
                            || unique_txns > max_txns_after_filtering
                        {
                            full = true;
                            break 'outer;
                        }
                        cur_all_txns += batch.size();
                        cur_unique_txns = unique_txns;
                        assert!(item.proof.is_none() == batches_without_proofs);
                        result.push(item);
                        if cur_all_txns == max_txns
                            || cur_unique_txns == max_txns_after_filtering
                            || cur_unique_txns >= soft_max_txns_after_filtering
                        {
                            full = true;
                            break 'outer;
                        }
                    }
                    i += 1;
                } else {
                    let _ = iters.swap_remove(i);
                    // Don't increment i; swapped element now at position i
                }
            }
            if !any_progress {
                break;
            }
        }
        debug!(
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

            // Batch per-author metrics: accumulate stats, emit once per author
            let now_usecs = aptos_infallible::duration_since_epoch().as_micros() as u64;
            let mut author_stats: HashMap<PeerId, (u64, f64, f64)> = HashMap::new();
            for item in &result {
                let author = item.info.author();
                let entry = author_stats.entry(author).or_insert((0, 0.0, 0.0));
                entry.0 += 1;

                let batch_create_ts_usecs = item
                    .info
                    .expiration()
                    .saturating_sub(self.batch_expiry_gap_when_init_usecs);
                entry.1 += now_usecs.saturating_sub(batch_create_ts_usecs) as f64 / 1_000.0;

                if let Some(insertion_time) = item.batch_insertion_time {
                    entry.2 += insertion_time.elapsed().as_secs_f64() * 1000.0;
                }
            }
            for (author, (count, total_age_ms, total_queue_ms)) in &author_stats {
                let author_str = author.short_str();
                let author_label = author_str.as_str();
                counters::BATCH_PULLED_BY_AUTHOR
                    .with_label_values(&[author_label, pull_kind])
                    .inc_by(*count);
                counters::BATCH_AGE_WHEN_PULLED
                    .with_label_values(&[author_label, pull_kind])
                    .observe(*total_age_ms / *count as f64);
                if *total_queue_ms > 0.0 {
                    counters::BATCH_QUEUE_DURATION
                        .with_label_values(&[author_label])
                        .observe(*total_queue_ms / *count as f64);
                }
            }

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
                queue.remove(key);
                if !queue.is_empty() {
                    self.author_to_batches.insert(key.author(), queue);
                }
            }

            if let Some(item) = self.items.get(&key.batch_key) {
                if item.is_committed() {
                    // Tombstone — just remove from items
                    self.items.remove(&key.batch_key);
                    continue;
                }

                if item.proof.is_some() {
                    // not committed proof that is expired
                    num_expired_but_not_committed += 1;
                    let batch_expiration = item.info.expiration();
                    counters::GAP_BETWEEN_BATCH_EXPIRATION_AND_CURRENT_TIME_WHEN_COMMIT
                        .observe((block_timestamp - batch_expiration) as f64);
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
                        // Decrement incremental counter
                        self.proofs_with_summary -= 1;
                    } else {
                        self.proofs_without_summary -= 1;
                        self.txns_in_proofs_without_summary -= item.info.num_txns();
                    }
                    self.dec_remaining_proofs(&item.info.author(), item.info.num_txns());
                    counters::GARBAGE_COLLECTED_IN_PROOF_QUEUE_COUNTER
                        .with_label_values(&["expired_proof"])
                        .inc();
                } else {
                    // Item without proof being removed
                    self.num_items_without_proof -= 1;
                }
                // Remove from candidate maps
                self.remove_from_candidates(&key.batch_key);
                claims::assert_some!(self.items.remove(&key.batch_key));
            }
        }
        counters::PROOF_QUEUE_UPDATE_TIMESTAMP_DURATION.observe_duration(start.elapsed());
        counters::NUM_PROOFS_EXPIRED_WHEN_COMMIT.inc_by(num_expired_but_not_committed);
    }

    pub(crate) fn remaining_txns_and_proofs(&self) -> (u64, u64) {
        let start = Instant::now();
        counters::NUM_TOTAL_TXNS_LEFT_ON_UPDATE.observe(self.remaining_txns_with_duplicates as f64);
        counters::NUM_TOTAL_PROOFS_LEFT_ON_UPDATE.observe(self.remaining_proofs as f64);
        counters::NUM_LOCAL_TXNS_LEFT_ON_UPDATE.observe(self.remaining_local_txns as f64);
        counters::NUM_LOCAL_PROOFS_LEFT_ON_UPDATE.observe(self.remaining_local_proofs as f64);

        // O(1) using incremental counters
        let remaining_txns_without_duplicates =
            self.txn_summary_num_occurrences.len() as u64 + self.txns_in_proofs_without_summary;

        // Ground truth checks in debug builds
        #[cfg(debug_assertions)]
        {
            let mut gt_extra_txns = 0u64;
            let mut gt_proofs_without = 0u64;
            let mut gt_proofs_with = 0u64;
            for batches in self.author_to_batches.values() {
                for (sort_key, _) in batches.iter() {
                    if let Some(item) = self.items.get(&sort_key.batch_key) {
                        if item.proof.is_some() {
                            if item.txn_summaries.is_some() {
                                gt_proofs_with += 1;
                            } else {
                                gt_proofs_without += 1;
                                gt_extra_txns += item.proof.as_ref().map_or(0, |p| p.num_txns());
                            }
                        }
                    }
                }
            }
            debug_assert_eq!(
                self.proofs_with_summary, gt_proofs_with,
                "incremental proofs_with_summary diverged"
            );
            debug_assert_eq!(
                self.proofs_without_summary, gt_proofs_without,
                "incremental proofs_without_summary diverged"
            );
            debug_assert_eq!(
                self.txns_in_proofs_without_summary, gt_extra_txns,
                "incremental txns_in_proofs_without_summary diverged"
            );
        }

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
        counters::TXNS_IN_PROOFS_WITHOUT_SUMMARIES
            .observe(self.txns_in_proofs_without_summary as f64);

        counters::PROOFS_WITHOUT_BATCH_SUMMARY.observe(self.proofs_without_summary as f64);
        counters::PROOFS_WITH_BATCH_SUMMARY.observe(self.proofs_with_summary as f64);

        counters::PROOF_QUEUE_REMAINING_TXNS_DURATION.observe_duration(start.elapsed());
        (remaining_txns_without_duplicates, self.remaining_proofs)
    }

    // Mark in the hashmap committed PoS, but keep them until they expire
    pub(crate) fn mark_committed(&mut self, batches: Vec<BatchInfoExt>) {
        let start = Instant::now();
        for batch in batches.into_iter() {
            let batch_key = BatchKey::from_info(&batch);

            // Extract state we need before mutating
            let item_state = self.items.get(&batch_key).map(|item| {
                (
                    item.is_committed(),
                    item.proof.is_some(),
                    item.txn_summaries.is_some(),
                    item.proof.as_ref().map(|p| p.gas_bucket_start()),
                    item.proof_insertion_time,
                )
            });

            match item_state {
                Some((true, _, _, _, _)) => {
                    // Already committed
                    continue;
                },
                Some((false, has_proof, has_txn_summaries, gas_bucket, proof_insertion_time)) => {
                    if has_proof {
                        let insertion_time =
                            proof_insertion_time.expect("Insertion time is updated with proof");
                        counters::pos_to_commit(
                            gas_bucket.expect("proof must have gas bucket"),
                            insertion_time.elapsed().as_secs_f64(),
                        );
                        self.dec_remaining_proofs(&batch.author(), batch.num_txns());
                        counters::GARBAGE_COLLECTED_IN_PROOF_QUEUE_COUNTER
                            .with_label_values(&["committed_proof"])
                            .inc();

                        // Decrement txn_summary_num_occurrences
                        if let Some(item) = self.items.get(&batch_key) {
                            if let Some(ref txn_summaries) = item.txn_summaries {
                                let summaries: Vec<_> = txn_summaries.clone();
                                for txn_summary in &summaries {
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
                        }

                        // Decrement incremental counters
                        if has_txn_summaries {
                            self.proofs_with_summary -= 1;
                        } else {
                            self.proofs_without_summary -= 1;
                            self.txns_in_proofs_without_summary -= batch.num_txns();
                        }
                    } else {
                        // Item without proof being committed
                        self.num_items_without_proof -= 1;
                        counters::GARBAGE_COLLECTED_IN_PROOF_QUEUE_COUNTER
                            .with_label_values(&["committed_batch_without_proof"])
                            .inc();
                    }

                    // Remove from candidate maps
                    self.remove_from_candidates(&batch_key);

                    // Remove from author_to_batches
                    if let Some(author_batches) = self.author_to_batches.get_mut(&batch.author()) {
                        author_batches.remove(&BatchSortKey::from_info(&batch));
                        if author_batches.is_empty() {
                            self.author_to_batches.remove(&batch.author());
                        }
                    }

                    // Mark committed (tombstone — stays in items until expiration)
                    self.items
                        .get_mut(&batch_key)
                        .expect("must exist")
                        .mark_committed();
                },
                None => {
                    // Item doesn't exist — insert a committed tombstone
                    let batch_sort_key = BatchSortKey::from_info(&batch);
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
                        batch_insertion_time: None,
                    });
                },
            }
        }
        counters::PROOF_QUEUE_COMMIT_DURATION.observe_duration(start.elapsed());
    }
}
