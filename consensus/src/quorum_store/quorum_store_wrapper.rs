// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::network::NetworkSender;
use crate::network_interface::ConsensusMsg;
use crate::quorum_store::{
    counters,
    quorum_store::{QuorumStoreCommand, QuorumStoreError},
    quorum_store_db::BatchIdDB,
    types::BatchId,
    utils::{BatchBuilder, MempoolProxy, RoundExpirations},
};
use crate::round_manager::VerifiedEvent;
use aptos_crypto::HashValue;
use aptos_logger::debug;
use aptos_mempool::QuorumStoreRequest;
use aptos_types::PeerId;
use channel::aptos_channel;
use consensus_types::{
    common::{Payload, PayloadFilter, Round, TransactionSummary},
    proof_of_store::{LogicalTime, ProofOfStore},
    request_response::{ConsensusResponse, WrapperCommand},
};
use futures::{
    channel::{
        mpsc::{Receiver, Sender},
        oneshot,
    },
    future::BoxFuture,
    stream::FuturesUnordered,
    StreamExt,
};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{sync::mpsc::Sender as TokioSender, sync::oneshot as TokioOneshot, time};

type ProofReceiveChannel = oneshot::Receiver<Result<(ProofOfStore, BatchId), QuorumStoreError>>;

// TODO: Consider storing batches and retrying upon QuorumStoreError:Timeout
#[allow(dead_code)]
pub struct QuorumStoreWrapper {
    mempool_proxy: MempoolProxy,
    quorum_store_sender: TokioSender<QuorumStoreCommand>,
    batches_in_progress: HashMap<BatchId, Vec<TransactionSummary>>,
    batch_expirations: RoundExpirations<BatchId>,
    batch_builder: BatchBuilder,
    latest_logical_time: LogicalTime,
    proofs_for_consensus: HashMap<HashValue, ProofOfStore>,
    mempool_txn_pull_max_count: u64,
    mempool_txn_pull_max_bytes: u64, // TODO: check if this is necessary, also check batch_builder
    // For ensuring that batch size does not exceed QuorumStore limit.
    max_batch_bytes: u64,
    max_batch_expiry_round_gap: Round,
    end_batch_ms: u128,
    last_end_batch_time: Instant,
    db: Arc<dyn BatchIdDB>,
}

impl QuorumStoreWrapper {
    pub fn new(
        epoch: u64,
        db: Arc<dyn BatchIdDB>,
        mempool_tx: Sender<QuorumStoreRequest>,
        quorum_store_sender: TokioSender<QuorumStoreCommand>,
        mempool_txn_pull_timeout_ms: u64,
        mempool_txn_pull_max_count: u64,
        mempool_txn_pull_max_bytes: u64,
        max_batch_bytes: u64,
        max_batch_expiry_round_gap: Round,
        end_batch_ms: u128,
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
            mempool_proxy: MempoolProxy::new(mempool_tx, mempool_txn_pull_timeout_ms),
            quorum_store_sender,
            batches_in_progress: HashMap::new(),
            batch_expirations: RoundExpirations::new(),
            batch_builder: BatchBuilder::new(batch_id, max_batch_bytes as usize),
            latest_logical_time: LogicalTime::new(epoch, 0),
            proofs_for_consensus: HashMap::new(),
            mempool_txn_pull_max_count,
            mempool_txn_pull_max_bytes,
            max_batch_bytes,
            max_batch_expiry_round_gap,
            end_batch_ms,
            last_end_batch_time: Instant::now(),
            db,
        }
    }

    pub(crate) async fn handle_scheduled_pull(&mut self) -> Option<ProofReceiveChannel> {
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
                exclude_txns,
            )
            .await
            .unwrap();

        debug!("QS: pulled_txns len: {:?}", pulled_txns.len());

        for txn in pulled_txns {
            // Daniel: remaining of the txn will be lost when hitting the limit, is it ok?
            if !self.batch_builder.append_transaction(&txn) {
                end_batch = true;
                break;
            }
        }
        let serialized_txns = self.batch_builder.take_serialized_txns();

        if self.last_end_batch_time.elapsed().as_millis() > self.end_batch_ms {
            end_batch = true;
            self.last_end_batch_time = Instant::now();
        }

        let batch_id = self.batch_builder.batch_id();
        if !end_batch {
            if !serialized_txns.is_empty() {
                self.quorum_store_sender
                    .send(QuorumStoreCommand::AppendToBatch(serialized_txns, batch_id))
                    .await
                    .expect("could not send to QuorumStore");
            }
            None
        } else {
            if self.batch_builder.is_empty() {
                // Quorum store metrics
                counters::CREATED_EMPTY_BATCHES_COUNT.inc();

                let duration = chrono::Utc::now().naive_utc().timestamp_millis() as u64
                    - self.batch_builder.time_created();
                counters::EMPTY_BATCH_CREATION_DURATION
                    .observe_duration(Duration::from_millis(duration));

                return None;
            }

            // Quorum store metrics
            let duration = chrono::Utc::now().naive_utc().timestamp_millis() as u64
                - self.batch_builder.time_created();
            counters::BATCH_CREATION_DURATION.observe_duration(Duration::from_millis(duration));

            counters::NUM_TXN_PER_BATCH.observe(self.batch_builder.summaries().len() as f64);

            self.db
                .save_batch_id(self.latest_logical_time.epoch(), batch_id + 1)
                .expect("Could not save to db");

            let (proof_tx, proof_rx) = oneshot::channel();
            let expiry_round = self.latest_logical_time.round() + self.max_batch_expiry_round_gap;
            let logical_time = LogicalTime::new(self.latest_logical_time.epoch(), expiry_round);

            self.quorum_store_sender
                .send(QuorumStoreCommand::EndBatch(
                    serialized_txns,
                    batch_id,
                    logical_time.clone(),
                    proof_tx,
                ))
                .await
                .expect("could not send to QuorumStore");

            self.batches_in_progress
                .insert(batch_id, self.batch_builder.take_summaries());
            self.batch_expirations.add_item(batch_id, expiry_round);

            Some(proof_rx)
        }
    }

    pub(crate) async fn broadcast_completed_proof(
        &mut self,
        proof: ProofOfStore,
        network_sender: &mut NetworkSender,
    ) {
        network_sender
            .broadcast_without_self(ConsensusMsg::ProofOfStoreBroadcastMsg(Box::new(
                proof.clone(),
            )))
            .await;
    }

    pub(crate) async fn insert_proof(&mut self, mut new_proof: ProofOfStore) {
        counters::POS_COUNT.inc();

        let maybe_proof = self.proofs_for_consensus.remove(new_proof.digest());
        if let Some(proof) = maybe_proof {
            if proof.expiration() > new_proof.expiration() {
                new_proof = proof;
            }
        }
        self.proofs_for_consensus
            .insert(*new_proof.digest(), new_proof);
    }

    pub(crate) async fn handle_local_proof(
        &mut self,
        msg: Result<(ProofOfStore, BatchId), QuorumStoreError>,
        network_sender: &mut NetworkSender,
    ) {
        match msg {
            Ok((proof, batch_id)) => {
                debug!(
                    "QS: received proof of store for batch id {}, digest {}",
                    batch_id,
                    proof.digest(),
                );
                // Handle batch_id

                self.insert_proof(proof.clone()).await;
                self.broadcast_completed_proof(proof, network_sender).await;
            }
            Err(QuorumStoreError::Timeout(batch_id)) => {
                // Quorum store measurements
                counters::TIMEOUT_BATCHES_COUNT.inc();

                debug!(
                    "QS: received timeout for proof of store, batch id = {}",
                    batch_id
                );
                // Not able to gather the proof, allow transactions to be polled again.
                self.batches_in_progress.remove(&batch_id);
            }
        }
    }

    pub(crate) async fn handle_consensus_request(&mut self, msg: WrapperCommand) {
        match msg {
            // TODO: check what max_txns consensus is using
            WrapperCommand::GetBlockRequest(round, max_txns, _max_bytes, filter, callback) => {
                // TODO: Pass along to batch_store
                let excluded_proofs: HashSet<HashValue> = match filter {
                    PayloadFilter::Empty => HashSet::new(),
                    PayloadFilter::DirectMempool(_) => {
                        unreachable!()
                    }
                    PayloadFilter::InQuorumStore(proofs) => proofs,
                };

                // TODO: optimization: exclude proofs if corresponding batches aren't present locally.
                let mut proof_block = Vec::new();
                let mut expired = Vec::new();
                for proof in self.proofs_for_consensus.values() {
                    if proof_block.len() == max_txns as usize {
                        break;
                    }

                    if proof.expiration()
                        < LogicalTime::new(self.latest_logical_time.epoch(), round)
                    {
                        expired.push(proof.digest().clone());
                    } else if !excluded_proofs.contains(proof.digest()) {
                        proof_block.push(proof.clone());
                    }
                }
                for digest in expired {
                    self.proofs_for_consensus.remove(&digest);
                }
                let res = ConsensusResponse::GetBlockResponse(if proof_block.is_empty() {
                    Payload::empty()
                } else {
                    debug!(
                        "QS: GetBlockRequest excluded len {}, block len {}",
                        excluded_proofs.len(),
                        proof_block.len()
                    );
                    Payload::InQuorumStore(proof_block)
                });
                callback
                    .send(Ok(res))
                    .expect("BlockResponse receiver not available");
            }
            WrapperCommand::CleanRequest(logical_time, digests) => {
                debug!("QS: got clean request from execution");
                assert_eq!(
                    self.latest_logical_time.epoch(),
                    logical_time.epoch(),
                    "Wrong epoch"
                );
                assert!(
                    self.latest_logical_time < logical_time,
                    "Non-increasing logical time"
                );
                self.latest_logical_time = logical_time;
                for batch_id in self.batch_expirations.expire(logical_time.round()) {
                    if self.batches_in_progress.remove(&batch_id).is_some() {
                        debug!(
                            "QS: expired batch w. id {} from batches_in_progress, new size {}",
                            batch_id,
                            self.batches_in_progress.len(),
                        );
                    }
                }
                for digest in digests {
                    if self.proofs_for_consensus.remove(&digest).is_some() {
                        debug!(
                            "QS: removed digest {} from batches_for_consensus, new size {}",
                            digest,
                            self.proofs_for_consensus.len(),
                        );
                    }
                }
            }
        }
    }

    pub async fn start(
        mut self,
        mut network_sender: NetworkSender,
        mut consensus_receiver: Receiver<WrapperCommand>,
        mut shutdown_rx: Receiver<oneshot::Sender<()>>,
        mut network_msg_rx: aptos_channel::Receiver<PeerId, VerifiedEvent>,
    ) {
        debug!(
            "[QS worker] QuorumStoreWrapper worker for epoch {} starting",
            self.latest_logical_time.epoch(),
        );

        let mut proofs_in_progress: FuturesUnordered<BoxFuture<'_, _>> = FuturesUnordered::new();

        // TODO: parameter? bring back back-off?
        let mut interval = time::interval(Duration::from_millis(
            10, // 50 is currently the end batch timer
        ));

        loop {
            let _timer = counters::WRAPPER_MAIN_LOOP.start_timer();

            tokio::select! {
                Some(ack_tx) = shutdown_rx.next() => {

            let (qs_shutdown_tx, qs_shutdown_rx) = TokioOneshot::channel();
                    self.quorum_store_sender
            .send(QuorumStoreCommand::Shutdown(qs_shutdown_tx))
            .await
            .expect("could not send to QuorumStore");

            qs_shutdown_rx.await.expect("Failed to stop QuorumStore");

                    ack_tx
            .send(())
            .expect("Failed to send shutdown ack from QuorumStoreWrapper");

                    break;
                },

                _ = interval.tick() => {
                    if let Some(proof_rx) = self.handle_scheduled_pull().await {
                        proofs_in_progress.push(Box::pin(proof_rx));
                    }
                },
                Some(next_proof) = proofs_in_progress.next() => {
            match next_proof {
            Ok(proof) => self.handle_local_proof(proof, &mut network_sender).await,
            Err(_) => {
                            debug!("QS: proof oneshot dropped");
                        }
            }
                },
                Some(msg) = consensus_receiver.next() => {
                    self.handle_consensus_request(msg).await;
                },
                Some(msg) = network_msg_rx.next() => {
                   if let VerifiedEvent::ProofOfStoreBroadcast(proof) = msg{
                        debug!("QS: got proof from peer");
                        self.insert_proof(*proof).await;
                    }
                },
            }
        }

        debug!(
            "[QS worker] QuorumStoreWrapper worker for epoch {} stopping",
            self.latest_logical_time.epoch(),
        );

        // Periodically:
        // 1. Pull from mempool.
        // 2. a. Start a batch with these txns if batch is not active
        //    b. Continue batch with these txns if batch is active
        // 3. Close batch if criteria is met.

        // State needed:
        // 1. txn summaries that are part of all pending batches: map<batch_id, vec<txn>>
        //    - pending batches: batches, including those in progress, that have not yet been cleaned.
        //    - batch_id: needs to include epoch, round info.
        // 2. all completed digests that have not yet been cleaned: map<batch_id, digest>
        //    -- is this really needed? pull_payload filters anyway. maybe all that's needed
        //    is a broadcast queue?
    }
}
