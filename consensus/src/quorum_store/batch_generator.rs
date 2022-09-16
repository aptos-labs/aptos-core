// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block_storage::BlockReader,
    quorum_store::{
        batch_coordinator::BatchCoordinatorCommand,
        counters,
        quorum_store_db::BatchIdDB,
        types::BatchId,
        utils::{BatchBuilder, MempoolProxy, RoundExpirations},
    },
};
use aptos_consensus_types::{
    common::{Round, TransactionSummary},
    proof_of_store::{LogicalTime, ProofOfStore},
};
use aptos_logger::prelude::*;
use aptos_mempool::QuorumStoreRequest;
use futures::{future::BoxFuture, stream::FuturesUnordered, StreamExt};
use futures_channel::{mpsc::Sender, oneshot};
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

pub struct BatchGenerator {
    db: Arc<dyn BatchIdDB>,
    mempool_proxy: MempoolProxy,
    batch_coordinator_tx: TokioSender<BatchCoordinatorCommand>,
    batches_in_progress: HashMap<BatchId, Vec<TransactionSummary>>,
    batch_expirations: RoundExpirations<BatchId>,
    batch_builder: BatchBuilder,
    latest_logical_time: LogicalTime,
    mempool_txn_pull_max_count: u64,
    mempool_txn_pull_max_bytes: u64,
    batch_expiry_round_gap_when_init: Round,
    end_batch_ms: u128,
    last_end_batch_time: Instant,
    // for consensus back pressure
    block_store: Arc<dyn BlockReader + Send + Sync>,
    // quorum store back pressure, get updated from proof manager
    qs_back_pressure: bool,
}

impl BatchGenerator {
    pub fn new(
        epoch: u64,
        db: Arc<dyn BatchIdDB>,
        mempool_tx: Sender<QuorumStoreRequest>,
        batch_coordinator_tx: TokioSender<BatchCoordinatorCommand>,
        mempool_txn_pull_timeout_ms: u64,
        mempool_txn_pull_max_count: u64,
        mempool_txn_pull_max_bytes: u64,
        max_batch_counts: usize,
        max_batch_bytes: usize,
        batch_expiry_round_gap_when_init: Round,
        end_batch_ms: u128,
        block_store: Arc<dyn BlockReader + Send + Sync>,
    ) -> Self {
        let batch_id = if let Some(id) = db
            .clean_and_get_batch_id(epoch)
            .expect("Could not read from db")
        {
            id + 1
        } else {
            0
        };
        db.save_batch_id(epoch, batch_id + 1)
            .expect("Could not save to db");

        Self {
            db,
            mempool_proxy: MempoolProxy::new(mempool_tx, mempool_txn_pull_timeout_ms),
            batch_coordinator_tx,
            batches_in_progress: HashMap::new(),
            batch_expirations: RoundExpirations::new(),
            batch_builder: BatchBuilder::new(batch_id, max_batch_counts, max_batch_bytes),
            latest_logical_time: LogicalTime::new(epoch, 0),
            mempool_txn_pull_max_count,
            mempool_txn_pull_max_bytes,
            batch_expiry_round_gap_when_init,
            end_batch_ms,
            last_end_batch_time: Instant::now(),
            block_store,
            qs_back_pressure: false,
        }
    }

    pub(crate) async fn handle_scheduled_pull(
        &mut self,
        end_batch_when_back_pressure: bool,
    ) -> Option<ProofCompletedChannel> {
        // TODO: as an optimization, we could filter out the txns that have expired

        let mut exclude_txns: Vec<_> = self
            .batches_in_progress
            .values()
            .flatten()
            .cloned()
            .collect();
        exclude_txns.extend(self.batch_builder.summaries().clone());

        debug!("QS: excluding txs len: {:?}", exclude_txns.len());
        let mut end_batch = false;
        // TODO: size and unwrap or not?
        let pulled_txns = self
            .mempool_proxy
            .pull_internal(
                self.mempool_txn_pull_max_count,
                self.mempool_txn_pull_max_bytes,
                // allow creating non-full fragments
                // is this a good place to disable fragments actually?
                true,
                exclude_txns,
            )
            .await
            .unwrap();

        debug!("QS: pulled_txns len: {:?}", pulled_txns.len());
        if pulled_txns.is_empty() {
            counters::PULLED_EMPTY_TXNS_COUNT.inc();
        } else {
            counters::PULLED_TXNS_COUNT.inc();
            counters::PULLED_TXNS_NUM.observe(pulled_txns.len() as f64);
        }

        for txn in pulled_txns {
            if !self.batch_builder.append_transaction(&txn) {
                end_batch = true;
                break;
            }
        }

        let serialized_txns = self.batch_builder.take_serialized_txns();

        if self.last_end_batch_time.elapsed().as_millis() > self.end_batch_ms {
            end_batch = true;
        }

        if end_batch_when_back_pressure {
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

            self.db
                .save_batch_id(self.latest_logical_time.epoch(), batch_id + 1)
                .expect("Could not save to db");

            let (proof_tx, proof_rx) = oneshot::channel();
            let expiry_round =
                self.latest_logical_time.round() + self.batch_expiry_round_gap_when_init;
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
                debug!(
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
        mut back_pressure_rx: tokio::sync::mpsc::Receiver<bool>,
        mut interval: Interval,
    ) {
        let mut proofs_in_progress: FuturesUnordered<BoxFuture<'_, _>> = FuturesUnordered::new();

        // this is the flag that records whether there is backpressure during last txn pulling from the mempool
        let mut back_pressure_in_last_pull = false;

        loop {
            let _timer = counters::WRAPPER_MAIN_LOOP.start_timer();

            tokio::select! {
                _ = interval.tick() => {
                    if self.qs_back_pressure || self.block_store.back_pressure() {
                        counters::QS_BACKPRESSURE.set(1);
                        // quorum store needs to be back pressured
                        // if last txn pull is not back pressured, there may be unfinished batch so we need to end the batch
                        if !back_pressure_in_last_pull {
                            if let Some(proof_rx) = self.handle_scheduled_pull(true).await {
                                proofs_in_progress.push(Box::pin(proof_rx));
                            }
                        }
                        back_pressure_in_last_pull = true;
                    } else {
                        counters::QS_BACKPRESSURE.set(0);
                        // no back pressure
                        if let Some(proof_rx) = self.handle_scheduled_pull(false).await {
                            proofs_in_progress.push(Box::pin(proof_rx));
                        }
                        back_pressure_in_last_pull = false;
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
                            debug!("QS: got clean request from execution");
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
                },
                Some(updated_back_pressure) = back_pressure_rx.recv() => {
                    self.qs_back_pressure = updated_back_pressure;
                },
            }
        }
    }
}
