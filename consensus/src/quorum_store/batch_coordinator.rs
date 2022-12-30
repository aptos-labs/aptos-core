// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::network::{NetworkSender, QuorumStoreSender};
use crate::quorum_store::batch_aggregator::BatchAggregator;
use crate::quorum_store::batch_store::{BatchStoreCommand, PersistRequest};
use crate::quorum_store::counters;
use crate::quorum_store::proof_builder::{ProofBuilderCommand, ProofReturnChannel};
use crate::quorum_store::quorum_store::QuorumStoreCommand;
use crate::quorum_store::types::{BatchId, Fragment, SerializedTransaction};
use aptos_consensus_types::proof_of_store::{LogicalTime, SignedDigestInfo};
use aptos_logger::prelude::*;
use aptos_types::PeerId;
use tokio::sync::mpsc::{Receiver, Sender};

pub struct BatchCoordinator {
    epoch: u64,
    my_peer_id: PeerId,
    network_sender: NetworkSender,
    command_rx: Receiver<QuorumStoreCommand>,
    batch_aggregator: BatchAggregator,
    batch_store_tx: Sender<BatchStoreCommand>,
    proof_builder_tx: Sender<ProofBuilderCommand>,
    fragment_id: usize,
}

impl BatchCoordinator {
    pub(crate) fn new(
        epoch: u64, //TODO: pass the epoch config
        my_peer_id: PeerId,
        network_sender: NetworkSender,
        wrapper_command_rx: Receiver<QuorumStoreCommand>,
        // TODO: probably build here
        batch_aggregator: BatchAggregator,
        batch_store_tx: Sender<BatchStoreCommand>,
        proof_builder_tx: Sender<ProofBuilderCommand>,
    ) -> Self {
        Self {
            epoch,
            my_peer_id,
            network_sender,
            command_rx: wrapper_command_rx,
            batch_aggregator,
            batch_store_tx,
            proof_builder_tx,
            fragment_id: 0,
        }
    }

    /// Aggregate & compute rolling digest, synchronously by worker.
    fn handle_append_to_batch(
        &mut self,
        fragment_payload: Vec<SerializedTransaction>,
        batch_id: BatchId,
    ) -> Fragment {
        match self.batch_aggregator.append_transactions(
            batch_id,
            self.fragment_id,
            fragment_payload.clone(),
        ) {
            Ok(()) => Fragment::new(
                self.epoch,
                batch_id,
                self.fragment_id,
                fragment_payload,
                None,
                self.my_peer_id,
            ),
            Err(e) => {
                unreachable!(
                    "[QuorumStore] Aggregation failed for own fragments with error {:?}",
                    e
                );
            },
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
                        SignedDigestInfo::new(
                            digest,
                            expiration,
                            payload.len() as u64,
                            num_bytes as u64,
                        ),
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
            },
            Err(e) => {
                unreachable!(
                    "[QuorumStore] Aggregation failed for own fragments with error {:?}",
                    e
                );
            },
        }
    }

    pub(crate) async fn start(mut self) {
        while let Some(command) = self.command_rx.recv().await {
            match command {
                QuorumStoreCommand::Shutdown(ack_tx) => {
                    // TODO: make sure this works
                    ack_tx
                        .send(())
                        .expect("Failed to send shutdown ack to QuorumStoreCoordinator");
                    break;
                },
                QuorumStoreCommand::AppendToBatch(fragment_payload, batch_id) => {
                    debug!("QS: end batch cmd received, batch id {}", batch_id);
                    let msg = self.handle_append_to_batch(fragment_payload, batch_id);
                    self.network_sender.broadcast_fragment(msg).await;

                    self.fragment_id = self.fragment_id + 1;
                },

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

                    self.network_sender.broadcast_fragment(fragment).await;

                    self.batch_store_tx
                        .send(batch_store_command)
                        .await
                        .expect("Failed to send to BatchStore");

                    counters::NUM_FRAGMENT_PER_BATCH.observe(self.fragment_id as f64);

                    self.fragment_id = 0;
                },
            }
        }
    }
}
