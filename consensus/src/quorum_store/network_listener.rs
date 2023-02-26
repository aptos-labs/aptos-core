// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    network::{NetworkSender, QuorumStoreSender},
    quorum_store::{
        batch_coordinator::BatchCoordinatorCommand, batch_reader::BatchReader, counters,
        proof_coordinator::ProofCoordinatorCommand, proof_manager::ProofManagerCommand,
        types::Batch,
    },
    round_manager::VerifiedEvent,
};
use aptos_channels::aptos_channel;
use aptos_logger::debug;
use aptos_types::PeerId;
use futures::StreamExt;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

pub(crate) struct NetworkListener {
    epoch: u64,
    my_peer_id: PeerId,
    network_msg_rx: aptos_channel::Receiver<PeerId, VerifiedEvent>,
    batch_reader: Arc<BatchReader<NetworkSender>>,
    proof_coordinator_tx: Sender<ProofCoordinatorCommand>,
    remote_batch_coordinator_tx: Vec<Sender<BatchCoordinatorCommand>>,
    proof_manager_tx: Sender<ProofManagerCommand>,
    network_sender: NetworkSender,
}

impl NetworkListener {
    pub(crate) fn new(
        epoch: u64,
        my_peer_id: PeerId,
        network_msg_rx: aptos_channel::Receiver<PeerId, VerifiedEvent>,
        batch_reader: Arc<BatchReader<NetworkSender>>,
        proof_coordinator_tx: Sender<ProofCoordinatorCommand>,
        remote_batch_coordinator_tx: Vec<Sender<BatchCoordinatorCommand>>,
        proof_manager_tx: Sender<ProofManagerCommand>,
        network_sender: NetworkSender,
    ) -> Self {
        Self {
            epoch,
            my_peer_id,
            network_msg_rx,
            batch_reader,
            proof_coordinator_tx,
            remote_batch_coordinator_tx,
            proof_manager_tx,
            network_sender,
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
                    let idx = fragment.source().to_vec()[0] as usize
                        % self.remote_batch_coordinator_tx.len();
                    debug!(
                        "QS: peer_id {:?},  # network_worker {}, hashed to idx {}",
                        fragment.source(),
                        self.remote_batch_coordinator_tx.len(),
                        idx
                    );
                    self.remote_batch_coordinator_tx[idx]
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
                    if let Ok(value) = self.batch_reader.get_batch_from_local(&request.digest()) {
                        let batch =
                            Batch::new(self.my_peer_id, self.epoch, request.digest(), value);
                        self.network_sender
                            .send_batch(batch, vec![request.source()])
                            .await;
                    }
                },
                VerifiedEvent::UnverifiedBatchMsg(batch) => {
                    counters::RECEIVED_BATCH_COUNT.inc();
                    debug!(
                        "QS: batch response from {:?} digest {}",
                        batch.source(),
                        batch.digest()
                    );
                    self.batch_reader
                        .receive_batch(batch.digest(), batch.into_payload())
                        .await;
                },
                VerifiedEvent::ProofOfStoreMsg(proof) => {
                    counters::REMOTE_POS_COUNT.inc();
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
