// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::quorum_store::{
    batch_coordinator::BatchCoordinatorCommand,
    counters,
    quorum_store_db::QuorumStoreStorage,
    types::BatchId,
    utils::{BatchBuilder, MempoolProxy, RoundExpirations},
};
use aptos_config::config::QuorumStoreConfig;
use aptos_consensus_types::{
    common::TransactionSummary,
    proof_of_store::{LogicalTime, ProofOfStore},
};
use aptos_logger::prelude::*;
use aptos_mempool::QuorumStoreRequest;
use futures::{future::BoxFuture, stream::FuturesUnordered, StreamExt};
use futures_channel::{mpsc::Sender, oneshot};
use rand::{thread_rng, RngCore};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{sync::mpsc::Sender as TokioSender, time::Interval};

type ProofCompletedChannel = oneshot::Receiver<Result<(ProofOfStore, BatchId), ProofError>>;

#[derive(Debug)]
pub enum BatchGeneratorCommand {
    CommitNotification(LogicalTime),
    Shutdown(tokio::sync::oneshot::Sender<()>),
}

#[derive(Debug, PartialEq, Eq)]
pub enum ProofError {
    Timeout(BatchId),
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct BackPressure {
    pub txn_count: bool,
    pub proof_count: bool,
}

pub struct BatchGenerator {
    db: Arc<dyn QuorumStoreStorage>,
    config: QuorumStoreConfig,
    mempool_proxy: MempoolProxy,
    batch_coordinator_tx: TokioSender<BatchCoordinatorCommand>,
    batches_in_progress: HashMap<BatchId, Vec<TransactionSummary>>,
    batch_expirations: RoundExpirations<BatchId>,
    batch_builder: BatchBuilder,
    latest_logical_time: LogicalTime,
    last_end_batch_time: Instant,
    // quorum store back pressure, get updated from proof manager
    back_pressure: BackPressure,
}

impl BatchGenerator {
    pub(crate) fn new(
        epoch: u64,
        config: QuorumStoreConfig,
        db: Arc<dyn QuorumStoreStorage>,
        mempool_tx: Sender<QuorumStoreRequest>,
        batch_coordinator_tx: TokioSender<BatchCoordinatorCommand>,
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
            BatchId::new(thread_rng().next_u64())
        };
        debug!("Initialized with batch_id of {}", batch_id);
        let mut incremented_batch_id = batch_id;
        incremented_batch_id.increment();
        db.save_batch_id(epoch, incremented_batch_id)
            .expect("Could not save to db");
        let max_batch_bytes = config.max_batch_bytes;

        Self {
            db,
            config,
            mempool_proxy: MempoolProxy::new(mempool_tx, mempool_txn_pull_timeout_ms),
            batch_coordinator_tx,
            batches_in_progress: HashMap::new(),
            batch_expirations: RoundExpirations::new(),
            batch_builder: BatchBuilder::new(batch_id, max_batch_bytes),
            latest_logical_time: LogicalTime::new(epoch, 0),
            last_end_batch_time: Instant::now(),
            back_pressure: BackPressure {
                txn_count: false,
                proof_count: false,
            },
        }
    }

    pub(crate) async fn handle_scheduled_pull(
        &mut self,
        max_count: u64,
    ) -> Option<ProofCompletedChannel> {
        // TODO: as an optimization, we could filter out the txns that have expired

        let mut exclude_txns: Vec<_> = self
            .batches_in_progress
            .values()
            .flatten()
            .cloned()
            .collect();
        exclude_txns.extend(self.batch_builder.summaries().clone());

        trace!("QS: excluding txs len: {:?}", exclude_txns.len());
        let mut end_batch = false;
        // TODO: size and unwrap or not?
        let pulled_txns = self
            .mempool_proxy
            .pull_internal(
                max_count,
                self.config.mempool_txn_pull_max_bytes,
                // allow creating non-full fragments
                // is this a good place to disable fragments actually?
                true,
                exclude_txns,
            )
            .await
            .unwrap();

        trace!("QS: pulled_txns len: {:?}", pulled_txns.len());
        if pulled_txns.is_empty() {
            counters::PULLED_EMPTY_TXNS_COUNT.inc();
        } else {
            counters::PULLED_TXNS_COUNT.inc();
            counters::PULLED_TXNS_NUM.observe(pulled_txns.len() as f64);
        }

        for txn in pulled_txns {
            if !self
                .batch_builder
                .append_transaction(&txn, max_count as usize)
            {
                end_batch = true;
                break;
            }
        }

        let serialized_txns = self.batch_builder.take_serialized_txns();

        if self.last_end_batch_time.elapsed().as_millis() > self.config.end_batch_ms as u128 {
            end_batch = true;
        }

        let batch_id = self.batch_builder.batch_id();
        if !end_batch {
            if !serialized_txns.is_empty() {
                self.batch_coordinator_tx
                    .send(BatchCoordinatorCommand::AppendToBatch(
                        serialized_txns,
                        batch_id,
                    ))
                    .await
                    .expect("could not send to QuorumStore");
            }
            None
        } else {
            if self.batch_builder.is_empty() {
                // Quorum store metrics
                counters::CREATED_EMPTY_BATCHES_COUNT.inc();

                let duration = self.last_end_batch_time.elapsed().as_secs_f64();
                counters::EMPTY_BATCH_CREATION_DURATION
                    .observe_duration(Duration::from_secs_f64(duration));

                self.last_end_batch_time = Instant::now();

                return None;
            }

            // Quorum store metrics
            counters::CREATED_BATCHES_COUNT.inc();

            let duration = self.last_end_batch_time.elapsed().as_secs_f64();
            counters::BATCH_CREATION_DURATION.observe_duration(Duration::from_secs_f64(duration));

            counters::NUM_TXN_PER_BATCH.observe(self.batch_builder.summaries().len() as f64);

            let mut incremented_batch_id = batch_id;
            incremented_batch_id.increment();
            self.db
                .save_batch_id(self.latest_logical_time.epoch(), incremented_batch_id)
                .expect("Could not save to db");

            let (proof_tx, proof_rx) = oneshot::channel();
            let expiry_round =
                self.latest_logical_time.round() + self.config.batch_expiry_round_gap_when_init;
            let logical_time = LogicalTime::new(self.latest_logical_time.epoch(), expiry_round);

            self.batch_coordinator_tx
                .send(BatchCoordinatorCommand::EndBatch(
                    serialized_txns,
                    batch_id,
                    logical_time,
                    proof_tx,
                ))
                .await
                .expect("could not send to QuorumStore");

            self.batches_in_progress
                .insert(batch_id, self.batch_builder.take_summaries());
            self.batch_expirations.add_item(batch_id, expiry_round);

            self.last_end_batch_time = Instant::now();

            Some(proof_rx)
        }
    }

    pub(crate) async fn handle_completed_proof(
        &mut self,
        msg: Result<(ProofOfStore, BatchId), ProofError>,
    ) {
        match msg {
            Ok((proof, batch_id)) => {
                trace!(
                    "QS: received proof of store for batch id {}, digest {}",
                    batch_id,
                    proof.digest(),
                );

                counters::LOCAL_POS_COUNT.inc();
            },
            Err(ProofError::Timeout(batch_id)) => {
                // Quorum store measurements
                counters::TIMEOUT_BATCHES_COUNT.inc();

                debug!(
                    "QS: received timeout for proof of store, batch id = {}",
                    batch_id
                );
                // Not able to gather the proof, allow transactions to be polled again.
                self.batches_in_progress.remove(&batch_id);
            },
        }
    }

    pub async fn start(
        mut self,
        mut cmd_rx: tokio::sync::mpsc::Receiver<BatchGeneratorCommand>,
        mut back_pressure_rx: tokio::sync::mpsc::Receiver<BackPressure>,
        mut interval: Interval,
    ) {
        let start = Instant::now();
        let mut proofs_in_progress: FuturesUnordered<BoxFuture<'_, _>> = FuturesUnordered::new();

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
            let _timer = counters::WRAPPER_MAIN_LOOP.start_timer();

            tokio::select! {
                biased;
                Some(updated_back_pressure) = back_pressure_rx.recv() => {
                    self.back_pressure = updated_back_pressure;
                },
                _ = interval.tick() => {
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
                        counters::QS_BACKPRESSURE_TXN_COUNT.observe(1);
                        counters::QS_BACKPRESSURE_DYNAMIC_MAX.observe(dynamic_pull_txn_per_s);
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
                        counters::QS_BACKPRESSURE_TXN_COUNT.observe(0);
                        counters::QS_BACKPRESSURE_DYNAMIC_MAX.observe(dynamic_pull_txn_per_s);
                    }
                    if self.back_pressure.proof_count {
                        counters::QS_BACKPRESSURE_PROOF_COUNT.observe(1);
                    } else {
                        counters::QS_BACKPRESSURE_PROOF_COUNT.observe(0);
                    }
                    let since_last_pull_ms = std::cmp::min(
                        now.duration_since(last_non_empty_pull).as_millis(),
                        self.config.batch_generation_max_interval_ms as u128
                    ) as usize;
                    if !self.back_pressure.proof_count || since_last_pull_ms == self.config.batch_generation_max_interval_ms {
                        last_non_empty_pull = now;
                        let dynamic_pull_max_txn = std::cmp::max(
                            (since_last_pull_ms as f64 / 1000.0 * dynamic_pull_txn_per_s as f64) as u64, 1);
                        if let Some(proof_rx) = self.handle_scheduled_pull(dynamic_pull_max_txn).await {
                            proofs_in_progress.push(Box::pin(proof_rx));
                        }
                    }
                },
                Some(next_proof) = proofs_in_progress.next() => {
                    match next_proof {
                        Ok(proof) => self.handle_completed_proof(proof).await,
                        Err(_) => {
                            debug!("QS: proof oneshot dropped");
                        }
                    }
                },
                Some(cmd) = cmd_rx.recv() => {
                    match cmd {
                        BatchGeneratorCommand::CommitNotification(logical_time) => {
                            trace!(
                                "QS: got clean request from execution, epoch {}, round {}",
                                logical_time.epoch(),
                                logical_time.round()
                            );
                            assert_eq!(
                                self.latest_logical_time.epoch(),
                                logical_time.epoch(),
                                "Wrong epoch"
                            );
                            assert!(
                                self.latest_logical_time <= logical_time,
                                "Decreasing logical time"
                            );
                            self.latest_logical_time = logical_time;
                            // Cleans up all batches that expire in rounds <= logical_time.round(). This is
                            // safe since clean request must occur only after execution result is certified.
                            for batch_id in self.batch_expirations.expire(logical_time.round()) {
                                if self.batches_in_progress.remove(&batch_id).is_some() {
                                    debug!(
                                        "QS: expired batch w. id {} from batches_in_progress, new size {}",
                                        batch_id,
                                        self.batches_in_progress.len(),
                                    );
                                }
                            }
                        },
                        BatchGeneratorCommand::Shutdown(ack_tx) => {
                            ack_tx
                                .send(())
                                .expect("Failed to send shutdown ack");
                            break;
                        },
                    }
                }
            }
        }
    }
}
