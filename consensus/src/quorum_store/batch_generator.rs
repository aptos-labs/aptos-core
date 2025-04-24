// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{
    monitor,
    network::{NetworkSender, QuorumStoreSender},
    quorum_store::{
        batch_store::BatchWriter,
        counters,
        quorum_store_db::QuorumStoreStorage,
        types::Batch,
        utils::{MempoolProxy, TimeExpirations},
    },
};
use aptos_config::config::QuorumStoreConfig;
use aptos_consensus_types::{
    common::{TransactionInProgress, TransactionSummary},
    proof_of_store::{BatchId, BatchInfo},
};
use aptos_experimental_runtimes::thread_manager::optimal_min_len;
use aptos_logger::prelude::*;
use aptos_mempool::QuorumStoreRequest;
use aptos_types::{transaction::SignedTransaction, PeerId};
use futures_channel::mpsc::Sender;
use rayon::prelude::*;
use std::{
    collections::{btree_map::Entry, BTreeMap, HashMap},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::time::Interval;

#[derive(Debug)]
pub enum BatchGeneratorCommand {
    CommitNotification(u64, Vec<BatchInfo>),
    ProofExpiration(Vec<BatchId>),
    RemoteBatch(Batch),
    Shutdown(tokio::sync::oneshot::Sender<()>),
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct BackPressure {
    pub txn_count: bool,
    pub proof_count: bool,
}

struct BatchInProgress {
    txns: Vec<TransactionSummary>,
    expiry_time_usecs: u64,
}

impl BatchInProgress {
    fn new(txns: Vec<TransactionSummary>, expiry_time_usecs: u64) -> Self {
        Self {
            txns,
            expiry_time_usecs,
        }
    }
}

pub struct BatchGenerator {
    epoch: u64,
    my_peer_id: PeerId,
    batch_id: BatchId,
    db: Arc<dyn QuorumStoreStorage>,
    batch_writer: Arc<dyn BatchWriter>,
    config: QuorumStoreConfig,
    mempool_proxy: MempoolProxy,
    batches_in_progress: HashMap<(PeerId, BatchId), BatchInProgress>,
    txns_in_progress_sorted: BTreeMap<TransactionSummary, TransactionInProgress>,
    batch_expirations: TimeExpirations<(PeerId, BatchId)>,
    latest_block_timestamp: u64,
    last_end_batch_time: Instant,
    // quorum store back pressure, get updated from proof manager
    back_pressure: BackPressure,
}

impl BatchGenerator {
    pub(crate) fn new(
        epoch: u64,
        my_peer_id: PeerId,
        config: QuorumStoreConfig,
        db: Arc<dyn QuorumStoreStorage>,
        batch_writer: Arc<dyn BatchWriter>,
        mempool_tx: Sender<QuorumStoreRequest>,
        mempool_txn_pull_timeout_ms: u64,
    ) -> Self {
        let batch_id = if let Some(mut id) = db
            .clean_and_get_batch_id(epoch)
            .expect("Could not read from db")
        {
            // If the node shut down mid-batch, then this increment is needed
            id.increment();
            id
        } else {
            BatchId::new(aptos_infallible::duration_since_epoch().as_micros() as u64)
        };
        debug!("Initialized with batch_id of {}", batch_id);
        let mut incremented_batch_id = batch_id;
        incremented_batch_id.increment();
        db.save_batch_id(epoch, incremented_batch_id)
            .expect("Could not save to db");

        Self {
            epoch,
            my_peer_id,
            batch_id,
            db,
            batch_writer,
            config,
            mempool_proxy: MempoolProxy::new(mempool_tx, mempool_txn_pull_timeout_ms),
            batches_in_progress: HashMap::new(),
            txns_in_progress_sorted: BTreeMap::new(),
            batch_expirations: TimeExpirations::new(),
            latest_block_timestamp: 0,
            last_end_batch_time: Instant::now(),
            back_pressure: BackPressure {
                txn_count: false,
                proof_count: false,
            },
        }
    }

    fn insert_batch(
        &mut self,
        author: PeerId,
        batch_id: BatchId,
        txns: Vec<SignedTransaction>,
        expiry_time_usecs: u64,
    ) {
        if self.batches_in_progress.contains_key(&(author, batch_id)) {
            return;
        }

        if author != self.my_peer_id {
            return;
        }

        // let txns_in_progress: Vec<_> = txns
        //     .par_iter()
        //     .with_min_len(optimal_min_len(txns.len(), 32))
        //     .map(|txn| {
        //         (
        //             TransactionSummary::new(
        //                 txn.sender(),
        //                 txn.sequence_number(),
        //                 txn.committed_hash(),
        //             ),
        //             TransactionInProgress::new(txn.gas_unit_price()),
        //         )
        //     })
        //     .collect();

        let mut txns = vec![];
        // for (summary, info) in txns_in_progress {
        //     let txn_info = self
        //         .txns_in_progress_sorted
        //         .entry(summary)
        //         .or_insert_with(|| TransactionInProgress::new(info.gas_unit_price));
        //     txn_info.increment();
        //     txn_info.gas_unit_price = info.gas_unit_price.max(txn_info.gas_unit_price);
        //     txns.push(summary);
        // }
        let updated_expiry_time_usecs = self
            .batches_in_progress
            .get(&(author, batch_id))
            .map_or(expiry_time_usecs, |batch_in_progress| {
                expiry_time_usecs.max(batch_in_progress.expiry_time_usecs)
            });
        self.batches_in_progress.insert(
            (author, batch_id),
            BatchInProgress::new(txns, updated_expiry_time_usecs),
        );
        self.batch_expirations
            .add_item((author, batch_id), updated_expiry_time_usecs);
    }

    fn create_new_batch(
        &mut self,
        txns: Vec<SignedTransaction>,
        expiry_time: u64,
        bucket_start: u64,
    ) -> Batch {
        let batch_id = self.batch_id;
        self.batch_id.increment();
        self.db
            .save_batch_id(self.epoch, self.batch_id)
            .expect("Could not save to db");

        self.insert_batch(self.my_peer_id, batch_id, txns.clone(), expiry_time);

        counters::CREATED_BATCHES_COUNT.inc();
        counters::num_txn_per_batch(bucket_start.to_string().as_str(), txns.len());

        Batch::new(
            batch_id,
            txns,
            self.epoch,
            expiry_time,
            self.my_peer_id,
            bucket_start,
        )
    }

    /// Push num_txns from txns into batches. If num_txns is larger than max size, then multiple
    /// batches are pushed.
    fn push_bucket_to_batches(
        &mut self,
        batches: &mut Vec<Batch>,
        txns: &mut Vec<SignedTransaction>,
        num_txns_in_bucket: usize,
        expiry_time: u64,
        bucket_start: u64,
        total_batches_remaining: &mut u64,
    ) {
        let mut txns_remaining = num_txns_in_bucket;
        while txns_remaining > 0 {
            if *total_batches_remaining == 0 {
                return;
            }
            let num_take_txns = std::cmp::min(self.config.sender_max_batch_txns, txns_remaining);
            let mut batch_bytes_remaining = self.config.sender_max_batch_bytes as u64;
            let num_batch_txns = num_take_txns;
            // let num_batch_txns = txns
            //     .iter()
            //     .take(num_take_txns)
            //     .take_while(|txn| {
            //         let txn_bytes = txn.txn_bytes_len() as u64;
            //         if batch_bytes_remaining.checked_sub(txn_bytes).is_some() {
            //             batch_bytes_remaining -= txn_bytes;
            //             true
            //         } else {
            //             false
            //         }
            //     })
            //     .count();
            if num_batch_txns > 0 {
                let batch_txns: Vec<_> = txns.drain(0..num_batch_txns).collect();
                let batch = self.create_new_batch(batch_txns, expiry_time, bucket_start);
                batches.push(batch);
                *total_batches_remaining = total_batches_remaining.saturating_sub(1);
                txns_remaining -= num_batch_txns;
            }
        }
    }

    fn bucket_into_batches(
        &mut self,
        pulled_txns: &mut Vec<SignedTransaction>,
        expiry_time: u64,
    ) -> Vec<Batch> {
        // Sort by gas, in descending order. This is a stable sort on existing mempool ordering,
        // so will not reorder accounts or their sequence numbers as long as they have the same gas.
        // pulled_txns.sort_by_key(|txn| u64::MAX - txn.gas_unit_price());
        //
        // let reverse_buckets_excluding_zero: Vec<_> = self
        //     .config
        //     .batch_buckets
        //     .iter()
        //     .skip(1)
        //     .rev()
        //     .cloned()
        //     .collect();
        //
        let mut max_batches_remaining = self.config.sender_max_num_batches as u64;
        let mut batches = vec![];
        // for bucket_start in &reverse_buckets_excluding_zero {
        //     if pulled_txns.is_empty() || max_batches_remaining == 0 {
        //         return batches;
        //     }
        //
        //     // Search for key in descending gas order
        //     let num_txns_in_bucket = match pulled_txns
        //         .binary_search_by_key(&(u64::MAX - (*bucket_start - 1), PeerId::ZERO), |txn| {
        //             (u64::MAX - txn.gas_unit_price(), txn.sender())
        //         }) {
        //         Ok(index) => index,
        //         Err(index) => index,
        //     };
        //     if num_txns_in_bucket == 0 {
        //         continue;
        //     }
        //
        //     self.push_bucket_to_batches(
        //         &mut batches,
        //         pulled_txns,
        //         num_txns_in_bucket,
        //         expiry_time,
        //         *bucket_start,
        //         &mut max_batches_remaining,
        //     );
        // }
        if !pulled_txns.is_empty() && max_batches_remaining > 0 {
            self.push_bucket_to_batches(
                &mut batches,
                pulled_txns,
                pulled_txns.len(),
                expiry_time,
                0,
                &mut max_batches_remaining,
            );
        }
        batches
    }

    fn remove_batch_in_progress(&mut self, author: PeerId, batch_id: BatchId) -> bool {
        let removed = self.batches_in_progress.remove(&(author, batch_id));
        match removed {
            Some(batch_in_progress) => {
                for txn in batch_in_progress.txns {
                    if let Entry::Occupied(mut o) = self.txns_in_progress_sorted.entry(txn) {
                        let info = o.get_mut();
                        if info.decrement() == 0 {
                            o.remove();
                        }
                    }
                }
                true
            },
            None => false,
        }
    }

    #[cfg(test)]
    pub fn remove_batch_in_progress_for_test(&mut self, author: PeerId, batch_id: BatchId) -> bool {
        self.remove_batch_in_progress(author, batch_id)
    }

    #[cfg(test)]
    pub fn txns_in_progress_sorted_len(&self) -> usize {
        self.txns_in_progress_sorted.len()
    }

    pub(crate) async fn handle_scheduled_pull(&mut self, max_count: u64) -> Vec<Batch> {
        counters::BATCH_PULL_EXCLUDED_TXNS.observe(self.txns_in_progress_sorted.len() as f64);
        trace!(
            "QS: excluding txs len: {:?}",
            self.txns_in_progress_sorted.len()
        );

        let mut pulled_txns = self
            .mempool_proxy
            .pull_internal(
                max_count,
                self.config.sender_max_total_bytes as u64,
                self.txns_in_progress_sorted.clone(),
            )
            .await
            .unwrap_or_default();

        trace!("QS: pulled_txns len: {:?}", pulled_txns.len());

        if pulled_txns.is_empty() {
            counters::PULLED_EMPTY_TXNS_COUNT.inc();
            // Quorum store metrics
            counters::CREATED_EMPTY_BATCHES_COUNT.inc();

            counters::EMPTY_BATCH_CREATION_DURATION
                .observe_duration(self.last_end_batch_time.elapsed());
            self.last_end_batch_time = Instant::now();
            return vec![];
        } else {
            counters::PULLED_TXNS_COUNT.inc();
            counters::PULLED_TXNS_NUM.observe(pulled_txns.len() as f64);
            if pulled_txns.len() as u64 == max_count {
                counters::BATCH_PULL_FULL_TXNS.observe(max_count as f64)
            }
        }
        counters::BATCH_CREATION_DURATION.observe_duration(self.last_end_batch_time.elapsed());

        let bucket_compute_start = Instant::now();
        let expiry_time = aptos_infallible::duration_since_epoch().as_micros() as u64
            + self.config.batch_expiry_gap_when_init_usecs;
        let batches = self.bucket_into_batches(&mut pulled_txns, expiry_time);
        self.last_end_batch_time = Instant::now();
        counters::BATCH_CREATION_COMPUTE_LATENCY.observe_duration(bucket_compute_start.elapsed());

        batches
    }

    pub(crate) fn handle_remote_batch(
        &mut self,
        author: PeerId,
        batch_id: BatchId,
        txns: Vec<SignedTransaction>,
    ) {
        let expiry_time_usecs = aptos_infallible::duration_since_epoch().as_micros() as u64
            + self.config.remote_batch_expiry_gap_when_init_usecs;
        self.insert_batch(author, batch_id, txns, expiry_time_usecs);
    }

    pub async fn start(
        mut self,
        mut network_sender: NetworkSender,
        mut cmd_rx: tokio::sync::mpsc::Receiver<BatchGeneratorCommand>,
        mut back_pressure_rx: tokio::sync::mpsc::Receiver<BackPressure>,
        mut interval: Interval,
    ) {
        let start = Instant::now();

        let mut last_non_empty_pull = start;
        let back_pressure_decrease_duration =
            Duration::from_millis(self.config.back_pressure.decrease_duration_ms);
        let back_pressure_increase_duration =
            Duration::from_millis(self.config.back_pressure.increase_duration_ms);
        let mut back_pressure_decrease_latest = start;
        let mut back_pressure_increase_latest = start;
        let mut dynamic_pull_txn_per_s = (self.config.back_pressure.dynamic_min_txn_per_s
            + self.config.back_pressure.dynamic_max_txn_per_s)
            / 2;

        loop {
            let _timer = counters::BATCH_GENERATOR_MAIN_LOOP.start_timer();

            tokio::select! {
                biased;
                Some(updated_back_pressure) = back_pressure_rx.recv() => {
                    self.back_pressure = updated_back_pressure;
                },
                _ = interval.tick() => monitor!("batch_generator_handle_tick", {

                    let tick_start = Instant::now();
                    // TODO: refactor back_pressure logic into its own function
                    if self.back_pressure.txn_count {
                        // multiplicative decrease, every second
                        if back_pressure_decrease_latest.elapsed() >= back_pressure_decrease_duration {
                            back_pressure_decrease_latest = tick_start;
                            dynamic_pull_txn_per_s = std::cmp::max(
                                (dynamic_pull_txn_per_s as f64 * self.config.back_pressure.decrease_fraction) as u64,
                                self.config.back_pressure.dynamic_min_txn_per_s,
                            );
                            trace!("QS: dynamic_max_pull_txn_per_s: {}", dynamic_pull_txn_per_s);
                        }
                        counters::QS_BACKPRESSURE_TXN_COUNT.observe(1.0);
                        counters::QS_BACKPRESSURE_MAKE_STRICTER_TXN_COUNT.observe(1.0);
                        counters::QS_BACKPRESSURE_DYNAMIC_MAX.observe(dynamic_pull_txn_per_s as f64);
                    } else {
                        // additive increase, every second
                        if back_pressure_increase_latest.elapsed() >= back_pressure_increase_duration {
                            back_pressure_increase_latest = tick_start;
                            dynamic_pull_txn_per_s = std::cmp::min(
                                dynamic_pull_txn_per_s + self.config.back_pressure.additive_increase_when_no_backpressure,
                                self.config.back_pressure.dynamic_max_txn_per_s,
                            );
                            trace!("QS: dynamic_max_pull_txn_per_s: {}", dynamic_pull_txn_per_s);
                        }
                        counters::QS_BACKPRESSURE_TXN_COUNT.observe(
                            if dynamic_pull_txn_per_s < self.config.back_pressure.dynamic_max_txn_per_s { 1.0 } else { 0.0 }
                        );
                        counters::QS_BACKPRESSURE_MAKE_STRICTER_TXN_COUNT.observe(0.0);
                        counters::QS_BACKPRESSURE_DYNAMIC_MAX.observe(dynamic_pull_txn_per_s as f64);
                    }
                    if self.back_pressure.proof_count {
                        counters::QS_BACKPRESSURE_PROOF_COUNT.observe(1.0);
                    } else {
                        counters::QS_BACKPRESSURE_PROOF_COUNT.observe(0.0);
                    }
                    let since_last_non_empty_pull_ms = std::cmp::min(
                        tick_start.duration_since(last_non_empty_pull).as_millis(),
                        self.config.batch_generation_max_interval_ms as u128
                    ) as usize;
                    if (!self.back_pressure.proof_count
                        && since_last_non_empty_pull_ms >= self.config.batch_generation_min_non_empty_interval_ms)
                        || since_last_non_empty_pull_ms == self.config.batch_generation_max_interval_ms {

                        let dynamic_pull_max_txn = std::cmp::max(
                            (since_last_non_empty_pull_ms as f64 / 1000.0 * dynamic_pull_txn_per_s as f64) as u64, 1);
                        let pull_max_txn = std::cmp::min(
                            dynamic_pull_max_txn,
                            self.config.sender_max_total_txns as u64,
                        );
                        let batches = self.handle_scheduled_pull(pull_max_txn).await;
                        if !batches.is_empty() {
                            last_non_empty_pull = tick_start;
                            let batch_writer = self.batch_writer.clone();
                            let mut network_sender = network_sender.clone();

                            tokio::task::spawn(async move {
                                let persist_start = Instant::now();
                                let batches_clone = batches.clone();

                                tokio::task::spawn_blocking(move || {
                                    let mut persist_requests = vec![];
                                    for batch in batches_clone {
                                        persist_requests.push(batch.into());
                                    }
                                    batch_writer.persist(persist_requests);
                                }).await.unwrap();
                                counters::BATCH_CREATION_PERSIST_LATENCY.observe_duration(persist_start.elapsed());

                                network_sender.broadcast_batch_msg(batches).await;
                            });
                        } else if tick_start.elapsed() > interval.period().checked_div(2).unwrap_or(Duration::ZERO) {
                            // If the pull takes too long, it's also accounted as a non-empty pull to avoid pulling too often.
                            last_non_empty_pull = tick_start;
                            sample!(
                                SampleRate::Duration(Duration::from_secs(1)),
                                info!(
                                    "QS: pull took a long time, {} ms",
                                    tick_start.elapsed().as_millis()
                                )
                            );
                        }
                    }
                }),
                Some(cmd) = cmd_rx.recv() => monitor!("batch_generator_handle_command", {
                    match cmd {
                        BatchGeneratorCommand::CommitNotification(block_timestamp, batches) => monitor!("qs_bgc_commit", {
                            trace!(
                                "QS: got clean request from execution, block timestamp {}",
                                block_timestamp
                            );
                            // Block timestamp is updated asynchronously, so it may race when it enters state sync.
                            if self.latest_block_timestamp > block_timestamp {
                                continue;
                            }
                            self.latest_block_timestamp = block_timestamp;

                            for (author, batch_id) in batches.iter().map(|b| (b.author(), b.batch_id())) {
                                if self.remove_batch_in_progress(author, batch_id) {
                                    counters::BATCH_IN_PROGRESS_COMMITTED.inc();
                                }
                            }

                            // Cleans up all batches that expire in timestamp <= block_timestamp. This is
                            // safe since clean request must occur only after execution result is certified.
                            for (author, batch_id) in self.batch_expirations.expire(block_timestamp) {
                                if let Some(batch_in_progress) = self.batches_in_progress.get(&(author, batch_id)) {
                                    // If there is an identical batch with higher expiry time, re-insert it.
                                    if batch_in_progress.expiry_time_usecs > block_timestamp {
                                        self.batch_expirations.add_item((author, batch_id), batch_in_progress.expiry_time_usecs);
                                        continue;
                                    }
                                }
                                if self.remove_batch_in_progress(author, batch_id) {
                                    counters::BATCH_IN_PROGRESS_EXPIRED.inc();
                                    debug!(
                                        "QS: logical time based expiration batch w. id {} from batches_in_progress, new size {}",
                                        batch_id,
                                        self.batches_in_progress.len(),
                                    );
                                }
                            }
                        }),
                        BatchGeneratorCommand::ProofExpiration(batch_ids) => monitor!("qs_bgc_proofexp", {
                            for batch_id in batch_ids {
                                counters::BATCH_IN_PROGRESS_TIMEOUT.inc();
                                debug!(
                                    "QS: received timeout for proof of store, batch id = {}",
                                    batch_id
                                );
                                // Not able to gather the proof, allow transactions to be polled again.
                                self.remove_batch_in_progress(self.my_peer_id, batch_id);
                            }
                        }),
                        BatchGeneratorCommand::RemoteBatch(batch) => {
                            monitor!("qs_bgc_remote", self.handle_remote_batch(batch.author(), batch.batch_id(), batch.into_transactions()));
                        },
                        BatchGeneratorCommand::Shutdown(ack_tx) => {
                            ack_tx
                                .send(())
                                .expect("Failed to send shutdown ack");
                            break;
                        },
                    }
                })
            }
        }
    }
}
