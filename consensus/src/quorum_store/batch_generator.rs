// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{
    monitor,
    network::{NetworkSender, QuorumStoreSender},
    quorum_store::{
        counters,
        quorum_store_db::QuorumStoreStorage,
        types::Batch,
        utils::{MempoolProxy, TimeExpirations},
    },
};
use aptos_config::config::QuorumStoreConfig;
use aptos_consensus_types::{
    common::{TransactionInProgress, TransactionSummary},
    proof_of_store::BatchId,
};
use aptos_logger::prelude::*;
use aptos_mempool::QuorumStoreRequest;
use aptos_types::{transaction::SignedTransaction, PeerId};
use futures_channel::mpsc::Sender;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::time::Interval;

#[derive(Debug)]
pub enum BatchGeneratorCommand {
    CommitNotification(u64),
    ProofExpiration(Vec<BatchId>),
    Shutdown(tokio::sync::oneshot::Sender<()>),
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct BackPressure {
    pub txn_count: bool,
    pub proof_count: bool,
}

pub struct BatchGenerator {
    epoch: u64,
    my_peer_id: PeerId,
    batch_id: BatchId,
    db: Arc<dyn QuorumStoreStorage>,
    config: QuorumStoreConfig,
    mempool_proxy: MempoolProxy,
    batches_in_progress: HashMap<BatchId, Vec<TransactionInProgress>>,
    batch_expirations: TimeExpirations<BatchId>,
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
            config,
            mempool_proxy: MempoolProxy::new(mempool_tx, mempool_txn_pull_timeout_ms),
            batches_in_progress: HashMap::new(),
            batch_expirations: TimeExpirations::new(),
            latest_block_timestamp: 0,
            last_end_batch_time: Instant::now(),
            back_pressure: BackPressure {
                txn_count: false,
                proof_count: false,
            },
        }
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

        let txns_in_progress: Vec<_> = txns
            .iter()
            .map(|txn| TransactionInProgress {
                summary: TransactionSummary {
                    sender: txn.sender(),
                    sequence_number: txn.sequence_number(),
                },
                gas_unit_price: txn.gas_unit_price(),
            })
            .collect();
        self.batches_in_progress.insert(batch_id, txns_in_progress);
        self.batch_expirations.add_item(batch_id, expiry_time);

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
    ) -> bool {
        let mut remaining_txns = num_txns_in_bucket;
        while remaining_txns > 0 {
            if batches.len() == self.config.sender_max_num_batches {
                return false;
            }
            let num_batch_txns = std::cmp::min(self.config.sender_max_batch_txns, remaining_txns);
            let batch_txns: Vec<_> = txns.drain(0..num_batch_txns).collect();
            let batch = self.create_new_batch(batch_txns, expiry_time, bucket_start);
            batches.push(batch);
            remaining_txns -= num_batch_txns;
        }
        true
    }

    fn bucket_into_batches(
        &mut self,
        pulled_txns: &mut Vec<SignedTransaction>,
        expiry_time: u64,
    ) -> Vec<Batch> {
        // Sort by gas, in descending order. This is a stable sort on existing mempool ordering,
        // so will not reorder accounts or their sequence numbers as long as they have the same gas.
        pulled_txns.sort_by_key(|txn| u64::MAX - txn.gas_unit_price());

        let reverse_buckets_excluding_zero: Vec<_> = self
            .config
            .batch_buckets
            .iter()
            .skip(1)
            .rev()
            .cloned()
            .collect();
        let mut batches = vec![];
        for bucket_start in &reverse_buckets_excluding_zero {
            if pulled_txns.is_empty() {
                break;
            }

            // Search for key in descending gas order
            let num_txns_in_bucket = match pulled_txns
                .binary_search_by_key(&(u64::MAX - (*bucket_start - 1), PeerId::ZERO), |txn| {
                    (u64::MAX - txn.gas_unit_price(), txn.sender())
                }) {
                Ok(index) => index,
                Err(index) => index,
            };
            if num_txns_in_bucket == 0 {
                continue;
            }

            let batches_space_remaining = self.push_bucket_to_batches(
                &mut batches,
                pulled_txns,
                num_txns_in_bucket,
                expiry_time,
                *bucket_start,
            );
            if !batches_space_remaining {
                return batches;
            }
        }
        if !pulled_txns.is_empty() {
            self.push_bucket_to_batches(
                &mut batches,
                pulled_txns,
                pulled_txns.len(),
                expiry_time,
                0,
            );
        }
        batches
    }

    pub(crate) async fn handle_scheduled_pull(&mut self, max_count: u64) -> Vec<Batch> {
        let exclude_txns: Vec<_> = self
            .batches_in_progress
            .values()
            .flatten()
            .cloned()
            .collect();
        counters::BATCH_PULL_EXCLUDED_TXNS.observe(exclude_txns.len() as f64);
        trace!("QS: excluding txs len: {:?}", exclude_txns.len());

        let mut pulled_txns = self
            .mempool_proxy
            .pull_internal(
                max_count,
                self.config.mempool_txn_pull_max_bytes,
                exclude_txns,
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
        counters::BATCH_CREATION_COMPUTE_LATENCY.observe_duration(bucket_compute_start.elapsed());
        self.last_end_batch_time = Instant::now();

        batches
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

                    let now = Instant::now();
                    // TODO: refactor back_pressure logic into its own function
                    if self.back_pressure.txn_count {
                        // multiplicative decrease, every second
                        if back_pressure_decrease_latest.elapsed() >= back_pressure_decrease_duration {
                            back_pressure_decrease_latest = now;
                            dynamic_pull_txn_per_s = std::cmp::max(
                                (dynamic_pull_txn_per_s as f64 * self.config.back_pressure.decrease_fraction) as u64,
                                self.config.back_pressure.dynamic_min_txn_per_s,
                            );
                            trace!("QS: dynamic_max_pull_txn_per_s: {}", dynamic_pull_txn_per_s);
                        }
                        counters::QS_BACKPRESSURE_TXN_COUNT.observe(1.0);
                        counters::QS_BACKPRESSURE_DYNAMIC_MAX.observe(dynamic_pull_txn_per_s as f64);
                    } else {
                        // additive increase, every second
                        if back_pressure_increase_latest.elapsed() >= back_pressure_increase_duration {
                            back_pressure_increase_latest = now;
                            dynamic_pull_txn_per_s = std::cmp::min(
                                dynamic_pull_txn_per_s + self.config.back_pressure.dynamic_min_txn_per_s,
                                self.config.back_pressure.dynamic_max_txn_per_s,
                            );
                            trace!("QS: dynamic_max_pull_txn_per_s: {}", dynamic_pull_txn_per_s);
                        }
                        counters::QS_BACKPRESSURE_TXN_COUNT.observe(0.0);
                        counters::QS_BACKPRESSURE_DYNAMIC_MAX.observe(dynamic_pull_txn_per_s as f64);
                    }
                    if self.back_pressure.proof_count {
                        counters::QS_BACKPRESSURE_PROOF_COUNT.observe(1.0);
                    } else {
                        counters::QS_BACKPRESSURE_PROOF_COUNT.observe(0.0);
                    }
                    let since_last_non_empty_pull_ms = std::cmp::min(
                        now.duration_since(last_non_empty_pull).as_millis(),
                        self.config.batch_generation_max_interval_ms as u128
                    ) as usize;
                    if (!self.back_pressure.proof_count
                        && since_last_non_empty_pull_ms >= self.config.batch_generation_min_non_empty_interval_ms)
                        || since_last_non_empty_pull_ms == self.config.batch_generation_max_interval_ms {

                        let dynamic_pull_max_txn = std::cmp::max(
                            (since_last_non_empty_pull_ms as f64 / 1000.0 * dynamic_pull_txn_per_s as f64) as u64, 1);
                        let batches = self.handle_scheduled_pull(dynamic_pull_max_txn).await;
                        if !batches.is_empty() {
                            last_non_empty_pull = now;
                            network_sender.broadcast_batch_msg(batches).await;
                        }
                    }
                }),
                Some(cmd) = cmd_rx.recv() => monitor!("batch_generator_handle_command", {
                    match cmd {
                        BatchGeneratorCommand::CommitNotification(block_timestamp) => {
                            trace!(
                                "QS: got clean request from execution, block timestamp {}",
                                block_timestamp
                            );
                            assert!(
                                self.latest_block_timestamp <= block_timestamp,
                                "Decreasing block timestamp"
                            );
                            self.latest_block_timestamp = block_timestamp;
                            // Cleans up all batches that expire in timestamp <= block_timestamp. This is
                            // safe since clean request must occur only after execution result is certified.
                            for batch_id in self.batch_expirations.expire(block_timestamp) {
                                if self.batches_in_progress.remove(&batch_id).is_some() {
                                    debug!(
                                        "QS: logical time based expiration batch w. id {} from batches_in_progress, new size {}",
                                        batch_id,
                                        self.batches_in_progress.len(),
                                    );
                                }
                            }
                        },
                        BatchGeneratorCommand::ProofExpiration(batch_ids) => {
                            for batch_id in batch_ids {
                                debug!(
                                    "QS: received timeout for proof of store, batch id = {}",
                                    batch_id
                                );
                                // Not able to gather the proof, allow transactions to be polled again.
                                self.batches_in_progress.remove(&batch_id);
                            }
                        }
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
