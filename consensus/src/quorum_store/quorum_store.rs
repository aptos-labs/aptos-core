// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::network::NetworkSender;
use crate::network_interface::ConsensusMsg;
use crate::quorum_store::{
    batch_aggregator::BatchAggregator,
    batch_reader::BatchReader,
    batch_store::{BatchStore, BatchStoreCommand, PersistRequest},
    network_listener::NetworkListener,
    proof_builder::{ProofBuilder, ProofBuilderCommand, ProofReturnChannel},
    quorum_store_db::QuorumStoreDB,
    types::{BatchId, Fragment, SerializedTransaction},
};
use crate::round_manager::VerifiedEvent;
use aptos_logger::debug;
use aptos_logger::spawn_named;
use aptos_types::{
    validator_signer::ValidatorSigner, validator_verifier::ValidatorVerifier, PeerId,
};
use channel::aptos_channel;
use consensus_types::proof_of_store::SignedDigestInfo;
use consensus_types::{common::Round, proof_of_store::LogicalTime};
use std::sync::Arc;
use tokio::sync::{
    mpsc::{channel, Receiver, Sender},
    oneshot,
};

#[derive(Debug)]
pub enum QuorumStoreCommand {
    AppendToBatch(Vec<SerializedTransaction>, BatchId),
    EndBatch(
        Vec<SerializedTransaction>,
        BatchId,
        LogicalTime,
        ProofReturnChannel,
    ),
    Shutdown(oneshot::Sender<()>),
}

#[derive(Debug, PartialEq, Eq)]
pub enum QuorumStoreError {
    Timeout(BatchId),
}

pub struct QuorumStore {
    epoch: u64,
    my_peer_id: PeerId,
    network_sender: NetworkSender,
    command_rx: Receiver<QuorumStoreCommand>,
    fragment_id: usize,
    batch_aggregator: BatchAggregator,
    batch_store_tx: Sender<BatchStoreCommand>,
    proof_builder_tx: Sender<ProofBuilderCommand>,
}

// TODO: change to appropriately signed integers.
#[derive(Clone)]
pub struct QuorumStoreConfig {
    pub channel_size: usize,
    pub proof_timeout_ms: usize,
    pub batch_request_num_peers: usize,
    pub end_batch_ms: u128,
    pub max_batch_bytes: usize,
    pub batch_request_timeout_ms: usize,
    /// Batches may have expiry set for max_batch_expiry_rounds_gap rounds after the
    /// latest committed round, and it will not be cleared from storage for another
    /// so other batch_expiry_grace_rounds rounds, so the peers on the network
    /// can still fetch the data they fall behind (later, they would have to state-sync).
    pub max_batch_expiry_round_gap: Round,
    pub batch_expiry_grace_rounds: Round,
    pub memory_quota: usize,
    pub db_quota: usize,
    pub mempool_txn_pull_max_count: u64,
    pub mempool_txn_pull_max_bytes: u64,
}

use std::future::Future;
use std::time::Duration;

pub fn spawn_monitored<T>(name: &'static str, future: T)
where
    T: Future + Send + 'static,
    T::Output: Send + 'static,
{
    let metrics_monitor = tokio_metrics::TaskMonitor::new();
    {
        let metrics_monitor = metrics_monitor.clone();
        tokio::spawn(async move {
            for interval in metrics_monitor.intervals() {
                // pretty-print the metric interval
                println!("{name}{:?}", interval);
                // wait 500ms
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        });
        _ = spawn_named!(name, future);
    }
}
impl QuorumStore {
    // TODO: pass epoch state
    pub fn new(
        epoch: u64, //TODO: pass the epoch config
        last_committed_round: Round,
        my_peer_id: PeerId,
        db: Arc<QuorumStoreDB>,
        network_msg_rx: aptos_channel::Receiver<PeerId, VerifiedEvent>,
        network_sender: NetworkSender,
        config: QuorumStoreConfig,
        validator_verifier: ValidatorVerifier, //TODO: pass the epoch config
        signer: ValidatorSigner,
        wrapper_command_rx: tokio::sync::mpsc::Receiver<QuorumStoreCommand>,
    ) -> (Self, Arc<BatchReader>) {
        debug!(
            "QS: QuorumStore new, epoch = {}, last r = {}, timeout ms {}",
            epoch, last_committed_round, config.batch_request_timeout_ms,
        );
        let validator_signer = Arc::new(signer);

        // Prepare communication channels among the threads.
        let (batch_store_tx, batch_store_rx) = channel(config.channel_size);
        let (batch_reader_tx, batch_reader_rx) = channel(config.channel_size);
        let (proof_builder_tx, proof_builder_rx) = channel(config.channel_size);

        let net = NetworkListener::new(
            epoch,
            network_msg_rx,
            batch_store_tx.clone(),
            batch_reader_tx.clone(),
            proof_builder_tx.clone(),
            config.max_batch_bytes,
        );
        let proof_builder = ProofBuilder::new(config.proof_timeout_ms, my_peer_id);
        let (batch_store, batch_reader) = BatchStore::new(
            epoch,
            last_committed_round,
            my_peer_id,
            network_sender.clone(),
            batch_store_tx.clone(),
            batch_reader_tx,
            batch_reader_rx,
            db,
            validator_verifier.clone(),
            validator_signer.clone(),
            config.max_batch_expiry_round_gap,
            config.batch_expiry_grace_rounds,
            config.batch_request_num_peers,
            config.batch_request_timeout_ms,
            config.memory_quota,
            config.db_quota,
        );

        spawn_monitored(
            "Quorum:ProofBuilder",
            proof_builder.start(proof_builder_rx, validator_verifier),
        );
        spawn_monitored("Quorum:NetworkListener", net.start());
        spawn_monitored(
            "Quorum:BatchStore",
            batch_store.start(batch_store_rx, proof_builder_tx.clone()),
        );

        debug!("QS: QuorumStore created");
        (
            Self {
                epoch,
                my_peer_id,
                network_sender,
                command_rx: wrapper_command_rx,
                fragment_id: 0,
                batch_aggregator: BatchAggregator::new(config.max_batch_bytes),
                batch_store_tx,
                proof_builder_tx,
            },
            batch_reader,
        )
    }

    /// Aggregate & compute rolling digest, synchronously by worker.
    fn handle_append_to_batch(
        &mut self,
        fragment_payload: Vec<SerializedTransaction>,
        batch_id: BatchId,
    ) -> ConsensusMsg {
        match self.batch_aggregator.append_transactions(
            batch_id,
            self.fragment_id,
            fragment_payload.clone(),
        ) {
            Ok(()) => {
                let fragment = Fragment::new(
                    self.epoch,
                    batch_id,
                    self.fragment_id,
                    fragment_payload,
                    None,
                    self.my_peer_id,
                );
                ConsensusMsg::FragmentMsg(Box::new(fragment))
            }
            Err(e) => {
                unreachable!(
                    "[QuorumStore] Aggregation failed for own fragments with error {:?}",
                    e
                );
            }
        }
    }

    /// Finalize the batch & digest, synchronously by worker.
    async fn handle_end_batch(
        &mut self,
        fragment_payload: Vec<SerializedTransaction>,
        batch_id: BatchId,
        expiration: LogicalTime,
        proof_tx: ProofReturnChannel,
    ) -> (BatchStoreCommand, Fragment) {
        match self
            .batch_aggregator
            .end_batch(batch_id, self.fragment_id, fragment_payload.clone())
        {
            Ok((num_bytes, payload, digest)) => {
                let fragment = Fragment::new(
                    self.epoch,
                    batch_id,
                    self.fragment_id,
                    fragment_payload,
                    Some(expiration.clone()),
                    self.my_peer_id,
                );

                self.proof_builder_tx
                    .send(ProofBuilderCommand::InitProof(
                        SignedDigestInfo::new(digest, expiration),
                        fragment.batch_id(),
                        proof_tx,
                    ))
                    .await
                    .expect("Failed to send to ProofBuilder");

                let persist_request = PersistRequest::new(
                    self.my_peer_id,
                    payload.clone(),
                    digest,
                    num_bytes,
                    expiration,
                );
                (BatchStoreCommand::Persist(persist_request), fragment)
            }
            Err(e) => {
                unreachable!(
                    "[QuorumStore] Aggregation failed for own fragments with error {:?}",
                    e
                );
            }
        }
    }

    pub async fn start(mut self) {
        debug!(
            "[QS worker] QuorumStore worker for epoch {} starting",
            self.epoch
        );

        while let Some(command) = self.command_rx.recv().await {
            match command {
                QuorumStoreCommand::Shutdown(ack_tx) => {
                    let (batch_store_shutdown_tx, batch_store_shutdown_rx) = oneshot::channel();
                    self.batch_store_tx
                        .send(BatchStoreCommand::Shutdown(batch_store_shutdown_tx))
                        .await
                        .expect("Failed to send to BatchStore");

                    batch_store_shutdown_rx
                        .await
                        .expect("Failed to stop BatchStore");

                    let (proof_builder_shutdown_tx, proof_builder_shutdown_rx) = oneshot::channel();
                    self.proof_builder_tx
                        .send(ProofBuilderCommand::Shutdown(proof_builder_shutdown_tx))
                        .await
                        .expect("Failed to send to ProofBuilder");

                    proof_builder_shutdown_rx
                        .await
                        .expect("Failed to stop ProofBuilder");

                    ack_tx
                        .send(())
                        .expect("Failed to send shutdown ack from QuorumStore");
                    break;
                }
                QuorumStoreCommand::AppendToBatch(fragment_payload, batch_id) => {
                    debug!("QS: end batch cmd received, batch id {}", batch_id);
                    let msg = self.handle_append_to_batch(fragment_payload, batch_id);
                    self.network_sender.broadcast_without_self(msg).await;

                    self.fragment_id = self.fragment_id + 1;
                }

                QuorumStoreCommand::EndBatch(
                    fragment_payload,
                    batch_id,
                    logical_time,
                    proof_tx,
                ) => {
                    debug!("QS: end batch cmd received, batch id = {}", batch_id);
                    let (batch_store_command, fragment) = self
                        .handle_end_batch(fragment_payload, batch_id, logical_time, proof_tx)
                        .await;

                    self.network_sender
                        .broadcast_without_self(ConsensusMsg::FragmentMsg(Box::new(fragment)))
                        .await;

                    self.batch_store_tx
                        .send(batch_store_command)
                        .await
                        .expect("Failed to send to BatchStore");

                    self.fragment_id = 0;
                }
            }
        }

        debug!(
            "[QS worker] QuorumStore worker for epoch {} stopping",
            self.epoch
        );
    }
}
