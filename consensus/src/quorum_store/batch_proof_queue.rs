// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::{
    batch_store::BatchStore,
    utils::{BatchKey, BatchSortKey, TimeExpirations},
};
use crate::quorum_store::counters;
use aptos_consensus_types::{
    common::{Author, TxnSummaryWithExpiration},
    payload_pull_params::PerBatchKindTxnLimits,
    proof_of_store::{BatchInfoExt, BatchKind, ProofOfStore, TBatchInfo},
    utils::PayloadTxnsSize,
};
use aptos_logger::{debug, sample, sample::SampleRate, warn};
use aptos_metrics_core::TimerHelper;
use aptos_short_hex_str::AsShortHexStr;
use aptos_types::{transaction::SignedTransaction, PeerId};
use rand::{prelude::SliceRandom, thread_rng};
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
    info: BatchInfoExt,
    /// Contains the summary of transactions in the batch.
    /// It is optional as the summary can be updated after the proof.
    txn_summaries: Option<Vec<TxnSummaryWithExpiration>>,

    /// Contains the proof associated with the batch.
    /// It is optional as the proof can be updated after the summary.
    proof: Option<ProofOfStore<BatchInfoExt>>,
    /// The time when the proof is inserted into this item.
    proof_insertion_time: Option<Instant>,
    /// The time when the batch (txn summaries) is first inserted into this item.
    batch_insertion_time: Option<Instant>,
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

/// Accumulates state across the 3 sequential pulls (proofs, opt-batches, inline-batches)
/// to avoid redundant work.
pub(crate) struct PullSession<'a> {
    pub(crate) excluded_batch_keys: HashSet<BatchKey>,
    pub(crate) filtered_txns: HashSet<&'a TxnSummaryWithExpiration>,
    pub(crate) cur_txns_per_kind: HashMap<BatchKind, u64>,
}

impl<'a> PullSession<'a> {
    fn new(
        excluded_batches: &HashSet<BatchInfoExt>,
        items: &'a HashMap<BatchKey, QueueItem>,
    ) -> Self {
        let mut excluded_batch_keys = HashSet::with_capacity(excluded_batches.len());
        let mut filtered_txns = HashSet::new();
        for batch_info in excluded_batches {
            let batch_key = BatchKey::from_info(batch_info);
            if let Some(txn_summaries) = items
                .get(&batch_key)
                .and_then(|item| item.txn_summaries.as_ref())
            {
                filtered_txns.extend(txn_summaries.iter());
            }
            excluded_batch_keys.insert(batch_key);
        }
        Self {
            excluded_batch_keys,
            filtered_txns,
            cur_txns_per_kind: HashMap::new(),
        }
    }

    /// Add pulled batches to the session so subsequent pulls exclude them.
    fn add_pulled_batches(
        &mut self,
        pulled: &[BatchInfoExt],
        items: &'a HashMap<BatchKey, QueueItem>,
    ) {
        for info in pulled {
            let batch_key = BatchKey::from_info(info);
            if let Some(txn_summaries) = items
                .get(&batch_key)
                .and_then(|item| item.txn_summaries.as_ref())
            {
                self.filtered_txns.extend(txn_summaries.iter());
            }
            self.excluded_batch_keys.insert(batch_key);
        }
    }

    /// Add pulled proofs to the session so subsequent pulls exclude them.
    fn add_pulled_proofs(
        &mut self,
        pulled: &[ProofOfStore<BatchInfoExt>],
        items: &'a HashMap<BatchKey, QueueItem>,
    ) {
        for proof in pulled {
            let batch_key = BatchKey::from_info(proof.info());
            if let Some(txn_summaries) = items
                .get(&batch_key)
                .and_then(|item| item.txn_summaries.as_ref())
            {
                self.filtered_txns.extend(txn_summaries.iter());
            }
            self.excluded_batch_keys.insert(batch_key);
        }
    }

    pub(crate) fn remaining_per_kind(
        &self,
        per_kind_txn_limits: &PerBatchKindTxnLimits,
    ) -> PerBatchKindTxnLimits {
        per_kind_txn_limits.remaining(&self.cur_txns_per_kind)
    }
}

pub struct BatchProofQueue {
    my_peer_id: PeerId,
    // Queue per peer for batches WITH proofs
    author_to_proof_batches: HashMap<PeerId, BTreeMap<BatchSortKey, BatchInfoExt>>,
    // Queue per peer for batches WITHOUT proofs
    author_to_non_proof_batches: HashMap<PeerId, BTreeMap<BatchSortKey, BatchInfoExt>>,
    // Map of Batch key to QueueItem containing Batch data and proofs
    items: HashMap<BatchKey, QueueItem>,
    // Number of unexpired and uncommitted proofs in which the txn_summary = (sender, replay protector, hash, expiration)
    // has been included. We only count those batches that are in both author maps and items along with proofs.
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

    // Incremental counters (Phase 1: Optimization 3)
    num_batches_without_proof_count: u64,
    num_proofs_without_summary_count: u64,
    num_proofs_with_summary_count: u64,
    remaining_proof_txns_without_summary: u64,
}

impl BatchProofQueue {
    pub(crate) fn new(
        my_peer_id: PeerId,
        batch_store: Arc<BatchStore>,
        batch_expiry_gap_when_init_usecs: u64,
    ) -> Self {
        Self {
            my_peer_id,
            author_to_proof_batches: HashMap::new(),
            author_to_non_proof_batches: HashMap::new(),
            items: HashMap::new(),
            txn_summary_num_occurrences: HashMap::new(),
            expirations: TimeExpirations::new(),
            batch_store,
            latest_block_timestamp: 0,
            remaining_txns_with_duplicates: 0,
            remaining_proofs: 0,
            remaining_local_txns: 0,
            remaining_local_proofs: 0,
            batch_expiry_gap_when_init_usecs,
            num_batches_without_proof_count: 0,
            num_proofs_without_summary_count: 0,
            num_proofs_with_summary_count: 0,
            remaining_proof_txns_without_summary: 0,
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

    pub(crate) fn num_batches_without_proof(&self) -> u64 {
        self.num_batches_without_proof_count
    }

    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.items.is_empty()
            && self.author_to_proof_batches.is_empty()
            && self.author_to_non_proof_batches.is_empty()
            && self.expirations.is_empty()
            && self.txn_summary_num_occurrences.is_empty()
    }

    fn remaining_txns_without_duplicates(&self) -> u64 {
        self.txn_summary_num_occurrences.len() as u64 + self.remaining_proof_txns_without_summary
    }

    /// Create a new PullSession for the given excluded batches.
    pub(crate) fn create_pull_session(
        &self,
        excluded_batches: &HashSet<BatchInfoExt>,
    ) -> PullSession<'_> {
        PullSession::new(excluded_batches, &self.items)
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

        // Move from non_proof to proof map if it existed there, otherwise insert into proof map
        let prev_in_non_proof = self
            .author_to_non_proof_batches
            .get_mut(&author)
            .and_then(|q| q.remove(&batch_sort_key));
        if let Some(ref _prev) = prev_in_non_proof {
            // Clean up empty maps
            if self
                .author_to_non_proof_batches
                .get(&author)
                .is_some_and(|q| q.is_empty())
            {
                self.author_to_non_proof_batches.remove(&author);
            }
        }
        let proof_batches_for_author = self.author_to_proof_batches.entry(author).or_default();
        proof_batches_for_author.insert(batch_sort_key.clone(), proof.info().clone());

        // Check if a batch with a higher batch Id (reverse sorted) exists
        if let Some((prev_batch_key, _)) = proof_batches_for_author
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

        // Determine if existing item had txn_summaries for counter updates
        let had_summaries = self
            .items
            .get(&batch_key)
            .is_some_and(|item| item.txn_summaries.is_some());

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
            },
        }

        if author == self.my_peer_id {
            counters::inc_local_pos_count(bucket, batch_version);
        } else {
            counters::inc_remote_pos_count(bucket, batch_version);
        }
        self.inc_remaining_proofs(&author, num_txns);

        // Update incremental counters
        if had_summaries {
            // Was a batch without proof, now has proof with summary
            self.num_batches_without_proof_count -= 1;
            self.num_proofs_with_summary_count += 1;
        } else {
            // New item with proof but no summary
            self.num_proofs_without_summary_count += 1;
            self.remaining_proof_txns_without_summary += num_txns;
        }

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

            // Determine if existing item already has a proof
            let has_proof = self
                .items
                .get(&batch_key)
                .is_some_and(|item| item.proof.is_some());

            if has_proof {
                // Already in proof map, just update the entry there
                self.author_to_proof_batches
                    .entry(batch_info.author())
                    .or_default()
                    .insert(batch_sort_key.clone(), batch_info.clone());
            } else {
                // No proof yet, goes in non-proof map
                self.author_to_non_proof_batches
                    .entry(batch_info.author())
                    .or_default()
                    .insert(batch_sort_key.clone(), batch_info.clone());
            }
            self.expirations
                .add_item(batch_sort_key, batch_info.expiration());

            // We only count txn summaries first time it is added to the queue
            // and only if the proof already exists.
            if has_proof {
                for txn_summary in &txn_summaries {
                    *self
                        .txn_summary_num_occurrences
                        .entry(*txn_summary)
                        .or_insert(0) += 1;
                }
            }

            let num_txns = batch_info.num_txns();
            match self.items.entry(batch_key) {
                Entry::Occupied(mut entry) => {
                    let item = entry.get_mut();
                    item.txn_summaries = Some(txn_summaries);
                    if item.batch_insertion_time.is_none() {
                        item.batch_insertion_time = Some(Instant::now());
                    }
                },
                Entry::Vacant(entry) => {
                    entry.insert(QueueItem {
                        info: batch_info,
                        proof: None,
                        proof_insertion_time: None,
                        txn_summaries: Some(txn_summaries),
                        batch_insertion_time: Some(Instant::now()),
                    });
                },
            }

            // Update incremental counters
            if has_proof {
                // Had proof without summary, now has both
                self.num_proofs_without_summary_count -= 1;
                self.num_proofs_with_summary_count += 1;
                self.remaining_proof_txns_without_summary -= num_txns;
            } else {
                // New batch without proof
                self.num_batches_without_proof_count += 1;
            }
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
        self.items.retain(|_, item| {
            if item.is_committed() || item.proof.is_some() || item.info.expiration() > timestamp {
                true
            } else {
                self.author_to_non_proof_batches
                    .get_mut(&item.info.author())
                    .map(|queue| queue.remove(&BatchSortKey::from_info(&item.info)));
                counters::GARBAGE_COLLECTED_IN_PROOF_QUEUE_COUNTER
                    .with_label_values(&["expired_batch_without_proof"])
                    .inc();
                self.num_batches_without_proof_count -= 1;
                false
            }
        });
    }

    fn log_remaining_data_after_pull(&self, pulled_proofs: &[ProofOfStore<BatchInfoExt>]) {
        let pulled_txns = pulled_proofs.iter().map(|p| p.num_txns()).sum::<u64>();
        let num_proofs_remaining_after_pull = self
            .remaining_proofs
            .saturating_sub(pulled_proofs.len() as u64);
        let num_txns_remaining_after_pull = self
            .remaining_txns_with_duplicates
            .saturating_sub(pulled_txns);

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

    // gets excluded and iterates over the vector returning non excluded or expired entries.
    // return the vector of pulled PoS, and the size of the remaining PoS
    // The flag in the second return argument is true iff the entire proof queue is fully utilized
    // when pulling the proofs. If any proof from proof queue cannot be included due to size limits,
    // this flag is set false.
    // Returns the proofs, the number of unique transactions in the proofs, and a flag indicating
    // whether the proof queue is fully utilized.
    pub(crate) fn pull_proofs<'a>(
        &'a self,
        session: &mut PullSession<'a>,
        max_txns: PayloadTxnsSize,
        max_txns_after_filtering: u64,
        soft_max_txns_after_filtering: u64,
        return_non_full: bool,
        block_timestamp: Duration,
        per_kind_txn_limits: &PerBatchKindTxnLimits,
    ) -> (
        Vec<ProofOfStore<BatchInfoExt>>,
        PayloadTxnsSize,
        u64,
        bool,
        HashMap<BatchKind, u64>,
    ) {
        let (result, all_txns, unique_txns, is_full, cur_txns_per_kind) = self.pull_internal(
            false,
            session,
            &HashSet::new(),
            max_txns,
            max_txns_after_filtering,
            soft_max_txns_after_filtering,
            return_non_full,
            block_timestamp,
            None,
            per_kind_txn_limits,
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
            self.log_remaining_data_after_pull(&proof_of_stores);
        }

        // Add pulled proofs to the session
        session.add_pulled_proofs(&proof_of_stores, &self.items);
        session.cur_txns_per_kind = cur_txns_per_kind.clone();

        (
            proof_of_stores,
            all_txns,
            unique_txns,
            !is_full,
            cur_txns_per_kind,
        )
    }

    pub fn pull_batches<'a>(
        &'a self,
        session: &mut PullSession<'a>,
        exclude_authors: &HashSet<Author>,
        max_txns: PayloadTxnsSize,
        max_txns_after_filtering: u64,
        soft_max_txns_after_filtering: u64,
        return_non_full: bool,
        block_timestamp: Duration,
        minimum_batch_age_usecs: Option<u64>,
        per_kind_txn_limits: &PerBatchKindTxnLimits,
    ) -> (
        Vec<BatchInfoExt>,
        PayloadTxnsSize,
        u64,
        HashMap<BatchKind, u64>,
    ) {
        let (result, pulled_txns, unique_txns, is_full, cur_txns_per_kind) = self
            .pull_batches_internal(
                session,
                exclude_authors,
                max_txns,
                max_txns_after_filtering,
                soft_max_txns_after_filtering,
                return_non_full,
                block_timestamp,
                minimum_batch_age_usecs,
                per_kind_txn_limits,
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

        // Add pulled batches to session
        session.add_pulled_batches(&result, &self.items);
        // Merge per-kind counts into session
        for (kind, count) in &cur_txns_per_kind {
            *session.cur_txns_per_kind.entry(*kind).or_insert(0) += count;
        }

        (result, pulled_txns, unique_txns, cur_txns_per_kind)
    }

    fn pull_batches_internal<'a>(
        &'a self,
        session: &mut PullSession<'a>,
        exclude_authors: &HashSet<Author>,
        max_txns: PayloadTxnsSize,
        max_txns_after_filtering: u64,
        soft_max_txns_after_filtering: u64,
        return_non_full: bool,
        block_timestamp: Duration,
        minimum_batch_age_usecs: Option<u64>,
        per_kind_txn_limits: &PerBatchKindTxnLimits,
        pull_kind: &str,
    ) -> (
        Vec<BatchInfoExt>,
        PayloadTxnsSize,
        u64,
        bool,
        HashMap<BatchKind, u64>,
    ) {
        let (result, all_txns, unique_txns, is_full, cur_txns_per_kind) = self.pull_internal(
            true,
            session,
            exclude_authors,
            max_txns,
            max_txns_after_filtering,
            soft_max_txns_after_filtering,
            return_non_full,
            block_timestamp,
            minimum_batch_age_usecs,
            per_kind_txn_limits,
            pull_kind,
        );
        let batches = result.into_iter().map(|item| item.info.clone()).collect();
        (batches, all_txns, unique_txns, is_full, cur_txns_per_kind)
    }

    pub fn pull_batches_with_transactions<'a>(
        &'a self,
        session: &mut PullSession<'a>,
        max_txns: PayloadTxnsSize,
        max_txns_after_filtering: u64,
        soft_max_txns_after_filtering: u64,
        return_non_full: bool,
        block_timestamp: Duration,
        per_kind_txn_limits: &PerBatchKindTxnLimits,
    ) -> (
        Vec<(BatchInfoExt, Vec<SignedTransaction>)>,
        PayloadTxnsSize,
        u64,
    ) {
        let (batches, pulled_txns, unique_txns, is_full, _cur_txns_per_kind) = self
            .pull_batches_internal(
                session,
                &HashSet::new(),
                max_txns,
                max_txns_after_filtering,
                soft_max_txns_after_filtering,
                return_non_full,
                block_timestamp,
                None,
                per_kind_txn_limits,
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

    fn pull_internal<'a>(
        &'a self,
        batches_without_proofs: bool,
        session: &mut PullSession<'a>,
        exclude_authors: &HashSet<Author>,
        max_txns: PayloadTxnsSize,
        max_txns_after_filtering: u64,
        soft_max_txns_after_filtering: u64,
        return_non_full: bool,
        block_timestamp: Duration,
        min_batch_age_usecs: Option<u64>,
        per_kind_txn_limits: &PerBatchKindTxnLimits,
        pull_kind: &str,
    ) -> (
        Vec<&'a QueueItem>,
        PayloadTxnsSize,
        u64,
        bool,
        HashMap<BatchKind, u64>,
    ) {
        let mut result = Vec::new();
        let mut cur_unique_txns = 0;
        let mut cur_all_txns = PayloadTxnsSize::zero();
        let mut excluded_txns = 0;
        let mut full = false;
        let mut cur_txns_per_kind: HashMap<BatchKind, u64> = HashMap::new();
        // Defer session mutation: collect pending txns locally so that if the pull
        // is discarded (return_non_full == false && !full), we don't pollute the
        // session's filtered_txns for subsequent pulls.
        let mut pending_filtered_txns: HashSet<&TxnSummaryWithExpiration> = HashSet::new();

        let max_batch_creation_ts_usecs = min_batch_age_usecs
            .map(|min_age| aptos_infallible::duration_since_epoch().as_micros() as u64 - min_age);

        // Select the appropriate author map based on proof status
        let author_map = if batches_without_proofs {
            &self.author_to_non_proof_batches
        } else {
            &self.author_to_proof_batches
        };

        let items = &self.items;
        let batch_expiry_gap = self.batch_expiry_gap_when_init_usecs;

        let mut iters = vec![];
        for (_, batches) in author_map
            .iter()
            .filter(|(author, _)| !exclude_authors.contains(author))
        {
            let batch_iter = batches.iter().rev().filter_map(|(sort_key, _info)| {
                if let Some(item) = items.get(&sort_key.batch_key) {
                    let batch_create_ts_usecs =
                        item.info.expiration().saturating_sub(batch_expiry_gap);

                    if max_batch_creation_ts_usecs
                        .is_some_and(|max_create_ts| batch_create_ts_usecs > max_create_ts)
                    {
                        counters::BATCH_SKIPPED_TOO_YOUNG
                            .with_label_values(&[item.info.author().short_str().as_str()])
                            .inc();
                        return None;
                    }

                    return Some((&item.info, item));
                }
                None
            });
            iters.push(batch_iter);
        }

        // Single shuffle then round-robin
        iters.shuffle(&mut thread_rng());
        let mut idx = 0;
        while !iters.is_empty() && !full {
            match iters[idx].next() {
                Some((batch, item)) => {
                    if session
                        .excluded_batch_keys
                        .contains(&BatchKey::from_info(batch))
                    {
                        excluded_txns += batch.num_txns();
                    } else {
                        // Check per-kind txn limit
                        if let Some(kind) = batch.batch_kind() {
                            if let Some(max) = per_kind_txn_limits.get(&kind) {
                                let cur = cur_txns_per_kind.get(&kind).copied().unwrap_or(0);
                                if cur + batch.num_txns() > max {
                                    // Skip this batch — would exceed per-kind limit.
                                    idx = (idx + 1) % iters.len();
                                    continue;
                                }
                            }
                        }

                        // Calculate unique txns (Optimization 5: single pass)
                        let new_unique =
                            item.txn_summaries
                                .as_ref()
                                .map_or(batch.num_txns(), |summaries| {
                                    summaries
                                        .iter()
                                        .filter(|txn_summary| {
                                            !session.filtered_txns.contains(txn_summary)
                                                && !pending_filtered_txns.contains(txn_summary)
                                                && block_timestamp.as_secs()
                                                    < txn_summary.expiration_timestamp_secs
                                        })
                                        .count() as u64
                                });

                        let unique_txns = cur_unique_txns + new_unique;
                        if cur_all_txns + batch.size() > max_txns
                            || unique_txns > max_txns_after_filtering
                        {
                            // Exceeded the limit for requested bytes or number of transactions.
                            full = true;
                            continue;
                        }
                        cur_all_txns += batch.size();
                        // Update per-kind counter
                        if let Some(kind) = batch.batch_kind() {
                            *cur_txns_per_kind.entry(kind).or_insert(0) += batch.num_txns();
                        }

                        // Accept — bulk insert summaries into pending set (deferred)
                        cur_unique_txns += new_unique;
                        if let Some(ref summaries) = item.txn_summaries {
                            pending_filtered_txns.extend(summaries.iter());
                        }

                        assert!(item.proof.is_none() == batches_without_proofs);
                        result.push(item);
                        if cur_all_txns == max_txns
                            || cur_unique_txns == max_txns_after_filtering
                            || cur_unique_txns >= soft_max_txns_after_filtering
                        {
                            full = true;
                            continue;
                        }
                    }
                    idx = (idx + 1) % iters.len();
                },
                None => {
                    let _ = iters.swap_remove(idx);
                    if !iters.is_empty() {
                        idx %= iters.len();
                    }
                },
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
            // Commit pending filtered txns into session only when the pull is accepted
            session.filtered_txns.extend(pending_filtered_txns);

            // Stable sort, so the order of proofs within an author will not change.
            result.sort_by_key(|item| Reverse(item.info.gas_bucket_start()));

            // Record per-author pull metrics
            let now_usecs = aptos_infallible::duration_since_epoch().as_micros() as u64;
            for item in &result {
                let author_str = item.info.author().short_str();
                let author = author_str.as_str();

                counters::BATCH_PULLED_BY_AUTHOR
                    .with_label_values(&[author, pull_kind])
                    .inc();

                let batch_create_ts_usecs = item
                    .info
                    .expiration()
                    .saturating_sub(self.batch_expiry_gap_when_init_usecs);
                let age_ms = now_usecs.saturating_sub(batch_create_ts_usecs) as f64 / 1_000.0;
                counters::BATCH_AGE_WHEN_PULLED
                    .with_label_values(&[author, pull_kind])
                    .observe(age_ms);

                if let Some(insertion_time) = item.batch_insertion_time {
                    counters::BATCH_QUEUE_DURATION
                        .with_label_values(&[author])
                        .observe(insertion_time.elapsed().as_secs_f64() * 1000.0);
                }
            }

            (
                result,
                cur_all_txns,
                cur_unique_txns,
                full,
                cur_txns_per_kind,
            )
        } else {
            (
                Vec::new(),
                PayloadTxnsSize::zero(),
                0,
                full,
                cur_txns_per_kind,
            )
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
            // Try to remove from proof batches first, then non-proof batches
            let batch = self
                .author_to_proof_batches
                .get_mut(&key.author())
                .and_then(|q| q.remove(key))
                .or_else(|| {
                    self.author_to_non_proof_batches
                        .get_mut(&key.author())
                        .and_then(|q| q.remove(key))
                });

            // ALWAYS clean up items — critical for committed batches
            if let Some(item) = self.items.remove(&key.batch_key) {
                if item.proof.is_some() {
                    // not committed proof that is expired
                    num_expired_but_not_committed += 1;
                    if let Some(ref batch_info) = batch {
                        counters::GAP_BETWEEN_BATCH_EXPIRATION_AND_CURRENT_TIME_WHEN_COMMIT
                            .observe((block_timestamp - batch_info.expiration()) as f64);
                    }
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
                    if let Some(ref batch_info) = batch {
                        self.dec_remaining_proofs(&batch_info.author(), batch_info.num_txns());
                    }
                    counters::GARBAGE_COLLECTED_IN_PROOF_QUEUE_COUNTER
                        .with_label_values(&["expired_proof"])
                        .inc();

                    // Update incremental counters
                    if item.txn_summaries.is_some() {
                        self.num_proofs_with_summary_count -= 1;
                    } else {
                        self.num_proofs_without_summary_count -= 1;
                        self.remaining_proof_txns_without_summary -= batch
                            .as_ref()
                            .map_or(item.info.num_txns(), |b| b.num_txns());
                    }
                } else if !item.is_committed() {
                    // Non-proof, non-committed item expiring
                    self.num_batches_without_proof_count -= 1;
                }
                // committed items: no counter update needed (already decremented on commit)
            }

            // Clean up empty author queues from both maps
            if self
                .author_to_proof_batches
                .get(&key.author())
                .is_some_and(|q| q.is_empty())
            {
                self.author_to_proof_batches.remove(&key.author());
            }
            if self
                .author_to_non_proof_batches
                .get(&key.author())
                .is_some_and(|q| q.is_empty())
            {
                self.author_to_non_proof_batches.remove(&key.author());
            }
        }
        counters::PROOF_QUEUE_UPDATE_TIMESTAMP_DURATION.observe_duration(start.elapsed());
        counters::NUM_PROOFS_EXPIRED_WHEN_COMMIT.inc_by(num_expired_but_not_committed);
    }

    // Number of unexpired and uncommitted proofs in the pipeline without txn summaries in
    // batch_summaries
    fn num_proofs_without_batch_summary(&self) -> u64 {
        self.num_proofs_without_summary_count
    }

    // Number of unexpired and uncommitted proofs in the pipeline with txn summaries in
    // batch_summaries
    fn num_proofs_with_batch_summary(&self) -> u64 {
        self.num_proofs_with_summary_count
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
    pub(crate) fn mark_committed(&mut self, batches: Vec<BatchInfoExt>) {
        let start = Instant::now();
        for batch in batches.into_iter() {
            let batch_key = BatchKey::from_info(&batch);

            // Extract state from immutable borrow first
            let item_state = self.items.get(&batch_key).map(|item| {
                let had_proof = item.proof.is_some();
                let had_summaries = item.txn_summaries.is_some();
                let is_committed = item.is_committed();
                let gas_bucket = item.proof.as_ref().map(|p| p.gas_bucket_start());
                let insertion_elapsed = item.proof_insertion_time.map(|t| t.elapsed());
                (
                    had_proof,
                    had_summaries,
                    is_committed,
                    gas_bucket,
                    insertion_elapsed,
                )
            });

            match item_state {
                Some((
                    had_proof,
                    had_summaries,
                    is_already_committed,
                    gas_bucket,
                    insertion_elapsed,
                )) => {
                    if had_proof {
                        if let (Some(bucket), Some(elapsed)) = (gas_bucket, insertion_elapsed) {
                            counters::pos_to_commit(bucket, elapsed.as_secs_f64());
                        }
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

                    // Remove from whichever author map it's in (committed items no longer in author maps)
                    let batch_sort_key = BatchSortKey::from_info(&batch);
                    if let Some(q) = self.author_to_proof_batches.get_mut(&batch.author()) {
                        q.remove(&batch_sort_key);
                        if q.is_empty() {
                            self.author_to_proof_batches.remove(&batch.author());
                        }
                    }
                    if let Some(q) = self.author_to_non_proof_batches.get_mut(&batch.author()) {
                        q.remove(&batch_sort_key);
                        if q.is_empty() {
                            self.author_to_non_proof_batches.remove(&batch.author());
                        }
                    }

                    // Update incremental counters
                    if !is_already_committed {
                        if had_proof && had_summaries {
                            self.num_proofs_with_summary_count -= 1;
                        } else if had_proof {
                            self.num_proofs_without_summary_count -= 1;
                            self.remaining_proof_txns_without_summary -= batch.num_txns();
                        } else {
                            self.num_batches_without_proof_count -= 1;
                        }
                    }
                },
                None => {
                    let batch_sort_key = BatchSortKey::from_info(&batch);
                    self.expirations
                        .add_item(batch_sort_key, batch.expiration());
                    // Committed items that we haven't seen before: just track in items + expirations
                    // No author map insertion needed — committed items don't participate in pulls
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
