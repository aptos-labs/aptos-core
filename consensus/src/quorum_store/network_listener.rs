// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::quorum_store::batch_coordinator::BatchCoordinatorCommand;
use crate::quorum_store::proof_manager::ProofManagerCommand;
use crate::{
    quorum_store::{
        batch_aggregator::BatchAggregator,
        batch_reader::BatchReaderCommand,
        batch_store::{BatchStoreCommand, PersistRequest},
        counters,
        proof_coordinator::ProofCoordinatorCommand,
        types::Fragment,
    },
    round_manager::VerifiedEvent,
};
use aptos_channels::aptos_channel;
use aptos_logger::debug;
use aptos_types::PeerId;
use futures::StreamExt;
use std::collections::HashMap;
use tokio::sync::mpsc::Sender;

pub(crate) struct NetworkListener {
    // TODO: reconsider which fields are needed.
    epoch: u64,
    network_msg_rx: aptos_channel::Receiver<PeerId, VerifiedEvent>,
    batch_reader_tx: Sender<BatchReaderCommand>,
    proof_coordinator_tx: Sender<ProofCoordinatorCommand>,
    batch_coordinator_tx: Sender<BatchCoordinatorCommand>,
    proof_manager_tx: Sender<ProofManagerCommand>,
    max_batch_bytes: usize,
}

impl NetworkListener {
    pub(crate) fn new(
        epoch: u64,
        network_msg_rx: aptos_channel::Receiver<PeerId, VerifiedEvent>,
        batch_reader_tx: Sender<BatchReaderCommand>,
        proof_coordinator_tx: Sender<ProofCoordinatorCommand>,
        batch_coordinator_tx: Sender<BatchCoordinatorCommand>,
        proof_manager_tx: Sender<ProofManagerCommand>,
        max_batch_bytes: usize,
    ) -> Self {
        Self {
            epoch,
            network_msg_rx,
            batch_reader_tx,
            proof_coordinator_tx,
            batch_coordinator_tx,
            proof_manager_tx,
            max_batch_bytes,
        }
    }

    pub async fn start(mut self) {
        debug!("QS: starting networking");
        //batch fragment -> batch_aggregator, persist it, and prapre signedDigests
        //Keep in memory caching in side the DB wrapper.
        //chack id -> self, call PoQSB.
        while let Some(msg) = self.network_msg_rx.next().await {
            // debug!("QS: network_listener msg {:?}", msg);
            match msg {
                // TODO: does the assumption have to be that network listener is shutdown first?
                VerifiedEvent::Shutdown(ack_tx) => {
                    debug!("QS: shutdown network listener received");
                    ack_tx
                        .send(())
                        .expect("Failed to send shutdown ack to QuorumStore");
                    break;
                },
                VerifiedEvent::SignedDigestMsg(signed_digest) => {
                    // debug!("QS: got SignedDigest from network");
                    let cmd = ProofCoordinatorCommand::AppendSignature(*signed_digest);
                    self.proof_coordinator_tx
                        .send(cmd)
                        .await
                        .expect("Could not send signed_digest to proof_coordinator");
                },
                VerifiedEvent::FragmentMsg(fragment) => {
                    counters::DELIVERED_FRAGMENTS_COUNT.inc();
                    self.batch_coordinator_tx
                        .send(BatchCoordinatorCommand::RemoteFragment(fragment))
                        .await
                        .expect("Could not send remote fragment");
                },
                VerifiedEvent::BatchRequestMsg(request) => {
                    counters::RECEIVED_BATCH_REQUEST_COUNT.inc();
                    debug!(
                        "QS: batch request from {:?} digest {}",
                        request.source(),
                        request.digest()
                    );
                    let cmd =
                        BatchReaderCommand::GetBatchForPeer(request.digest(), request.source());
                    self.batch_reader_tx
                        .send(cmd)
                        .await
                        .expect("could not push Batch batch_reader");
                },
                VerifiedEvent::UnverifiedBatchMsg(batch) => {
                    counters::RECEIVED_BATCH_COUNT.inc();
                    debug!(
                        "QS: batch response from {:?} digest {}",
                        batch.source(),
                        batch.digest()
                    );
                    let cmd =
                        BatchReaderCommand::BatchResponse(batch.digest(), batch.into_payload());
                    self.batch_reader_tx
                        .send(cmd)
                        .await
                        .expect("could not push Batch batch_reader");
                },
                VerifiedEvent::ProofOfStoreMsg(proof) => {
                    let cmd = ProofManagerCommand::RemoteProof(*proof);
                    self.proof_manager_tx
                        .send(cmd)
                        .await
                        .expect("could not push Proof proof_of_store");
                },
                _ => {
                    unreachable!()
                },
            };
        }
    }
}
