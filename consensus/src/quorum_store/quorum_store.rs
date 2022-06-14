// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::network::NetworkSender;
use crate::network_interface::ConsensusMsg;
use crate::quorum_store::types::Fragment;
use crate::quorum_store::{
    batch_aggregator::{AggregationMode, BatchAggregator},
    batch_reader::BatchReader,
    batch_store::{BatchStore, BatchStoreCommand, LogicalTime, PersistRequest},
    network_listener::NetworkListener,
    proof_builder::{ProofBuilder, ProofBuilderCommand},
    quorum_store_db::QuorumStoreDB,
    types::{BatchId, ProofOfStore, SignedDigest},
};
use crate::round_manager::VerifiedEvent;
use aptos_crypto::HashValue;
use aptos_types::{
    validator_signer::ValidatorSigner, validator_verifier::ValidatorVerifier, PeerId,
};
use channel::aptos_channel;
use consensus_types::common::{Payload, Round};
use futures::{
    future::BoxFuture,
    stream::{futures_unordered::FuturesUnordered, StreamExt as _},
};
use std::collections::HashMap;
use std::sync::{mpsc::sync_channel, Arc};
use tokio::sync::{
    mpsc::{channel, Receiver, Sender},
    oneshot,
};

pub type ProofReturnChannel = oneshot::Sender<Result<ProofOfStore, QuorumStoreError>>;

#[allow(dead_code)]
pub enum QuorumStoreCommand {
    AppendToBatch(Payload),
    EndBatch(Payload, LogicalTime, ProofReturnChannel),
}

#[derive(Debug)]
pub enum QuorumStoreError {
    Timeout,
    BatchSizeLimit,
}

#[allow(dead_code)]
pub struct QuorumStore {
    epoch: u64,
    my_peer_id: PeerId,
    network_sender: NetworkSender,
    command_rx: Receiver<QuorumStoreCommand>,
    batch_id: BatchId,
    fragment_id: usize,
    batch_aggregator: BatchAggregator,
    batch_store_tx: Sender<BatchStoreCommand>,
    proof_builder_tx: Sender<ProofBuilderCommand>,
    validator_signer: Arc<ValidatorSigner>,
    digest_end_batch: HashMap<HashValue, (Fragment, ProofReturnChannel)>,
}

pub struct QuorumStoreConfig {
    pub channel_size: usize,
    pub proof_timeout_ms: usize,
    pub batch_request_num_peers: usize,
    pub batch_request_timeout_ms: usize,
    /// Don't clean up batches for MAX_EXECUTION_ROUND_LAG rounds, so other
    /// peers on the network can still fetch (later, they would have to state-sync).
    pub max_execution_round_lag: Round,
    pub max_batch_size: usize,
    pub memory_quota: usize,
    pub db_quota: usize,
}

#[allow(dead_code)]
impl QuorumStore {
    //TODO: pass epoc state
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
        wrapper_command_rx: Receiver<QuorumStoreCommand>,
    ) -> (Self, Arc<BatchReader>) {
        let validator_signer = Arc::new(signer);
        //prepare the channels for communication among the working thread.
        let (batch_store_tx, batch_store_rx) = channel(config.channel_size);
        let (batch_reader_tx, batch_reader_rx) = sync_channel(config.channel_size);
        let (proof_builder_tx, proof_builder_rx) = channel(config.channel_size);

        let net = NetworkListener::new(
            epoch,
            network_msg_rx,
            batch_store_tx.clone(),
            batch_reader_tx.clone(),
            proof_builder_tx.clone(),
            config.max_batch_size,
        );
        let proof_builder = ProofBuilder::new(config.proof_timeout_ms);
        let (batch_store, batch_reader) = BatchStore::new(
            epoch,
            last_committed_round,
            my_peer_id,
            network_sender.clone(),
            batch_store_tx.clone(),
            batch_reader_tx,
            batch_reader_rx,
            db,
            validator_signer.clone(),
            config.max_execution_round_lag,
            config.batch_request_num_peers,
            config.batch_request_timeout_ms,
            config.memory_quota,
            config.db_quota,
        );

        tokio::spawn(proof_builder.start(proof_builder_rx, validator_verifier.clone()));
        tokio::spawn(net.start());
        tokio::spawn(batch_store.start(validator_verifier, batch_store_rx));

        (
            Self {
                epoch,
                my_peer_id,
                network_sender,
                command_rx: wrapper_command_rx,
                batch_id: 0,
                fragment_id: 0,
                batch_aggregator: BatchAggregator::new(config.max_batch_size),
                batch_store_tx,
                proof_builder_tx,
                validator_signer,
                digest_end_batch: HashMap::new(),
            },
            batch_reader,
        )
    }

    /// Aggregate & compute rolling digest, synchronously by worker.
    fn handle_append_to_batch(&mut self, fragment_payload: Payload) -> Option<ConsensusMsg> {
        if self.batch_aggregator.append_transactions(
            self.batch_id,
            self.fragment_id,
            fragment_payload.clone(),
            AggregationMode::AssertMissedFragment,
        ) {
            let fragment = Fragment::new(
                self.epoch,
                self.batch_id,
                self.fragment_id,
                fragment_payload,
                None,
                self.my_peer_id,
                self.validator_signer.clone(),
            );
            Some(ConsensusMsg::FragmentMsg(Box::new(fragment)))
        } else {
            None
        }
    }

    /// Finalize the batch & digest, synchronously by worker.
    fn handle_end_batch(
        &mut self,
        fragment_payload: Payload,
        expiration: LogicalTime,
        proof_tx: ProofReturnChannel,
    ) -> Option<(BatchStoreCommand, oneshot::Receiver<SignedDigest>)> {
        if let Some((num_bytes, payload, digest_hash)) = self.batch_aggregator.end_batch(
            self.batch_id,
            self.fragment_id,
            fragment_payload.clone(),
            AggregationMode::AssertMissedFragment,
        ) {
            let (persist_request_tx, persist_request_rx) = oneshot::channel();

            let fragment = Fragment::new(
                self.epoch,
                self.batch_id,
                self.fragment_id,
                fragment_payload,
                Some(expiration.clone()),
                self.my_peer_id,
                self.validator_signer.clone(),
            );
            self.digest_end_batch
                .insert(digest_hash, (fragment, proof_tx));

            let persist_request = PersistRequest::new(
                self.my_peer_id,
                payload.clone(),
                digest_hash,
                num_bytes,
                expiration,
            );
            Some((
                BatchStoreCommand::Persist(persist_request, Some(persist_request_tx)),
                persist_request_rx,
            ))
        } else {
            proof_tx
                .send(Err(QuorumStoreError::BatchSizeLimit))
                .expect("Proof receiver not available");
            None
        }
    }

    pub async fn start(mut self) {
        let mut futures: FuturesUnordered<BoxFuture<'_, _>> = FuturesUnordered::new();

        loop {
            tokio::select! {
                Some(command) = self.command_rx.recv() => {
                    match command {
                        QuorumStoreCommand::AppendToBatch(fragment_payload) => {
                            if let Some(msg) = self.handle_append_to_batch(fragment_payload){
                               self.network_sender.broadcast_without_self(msg).await;
                            }
                            self.fragment_id = self.fragment_id + 1;
                        }

                        QuorumStoreCommand::EndBatch(fragment_payload, logical_time, tx) => {
                            if let
                            Some((batch_store_command, response_rx)) =
                                self.handle_end_batch(fragment_payload, logical_time, tx){

                            self.batch_store_tx
                                .send(batch_store_command)
                                .await
                                .expect("Failed to send to BatchStore");
                            futures.push(Box::pin(response_rx));
                                }

                            self.batch_id = self.batch_id + 1;
                            self.fragment_id = 0;
                        }
                    }
                },

                Some(result) = futures.next() => match result {
                    Ok(signed_digest) => {
                        let (last_fragment, proof_tx) =
                            self.digest_end_batch.remove(&signed_digest.info.digest).unwrap();
                        self.proof_builder_tx
                            .send(ProofBuilderCommand::InitProof(signed_digest, proof_tx))
                            .await
                            .expect("Failed to send to ProofBuilder");

                        //TODO: consider waiting until proof_builder processes the command.
                        self.network_sender
                            .broadcast_without_self(ConsensusMsg::FragmentMsg(Box::new(last_fragment)))
                            .await;
                    },
                    Err(_) => {

                    }
                }
            }
        }
    }
}
