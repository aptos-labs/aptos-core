// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    quorum_store::{
        batch_coordinator::BatchCoordinatorCommand, batch_reader::BatchReaderCommand, counters,
        proof_coordinator::ProofCoordinatorCommand, proof_manager::ProofManagerCommand,
    },
    round_manager::VerifiedEvent,
};
use aptos_channels::aptos_channel;
use aptos_logger::debug;
use aptos_types::PeerId;
use futures::StreamExt;
use tokio::sync::mpsc::Sender;

pub(crate) struct NetworkListener {
    network_msg_rx: aptos_channel::Receiver<PeerId, VerifiedEvent>,
    batch_reader_tx: Sender<BatchReaderCommand>,
    proof_coordinator_tx: Sender<ProofCoordinatorCommand>,
    remote_batch_coordinator_tx: Vec<Sender<BatchCoordinatorCommand>>,
    proof_manager_tx: Sender<ProofManagerCommand>,
}

impl NetworkListener {
    pub(crate) fn new(
        network_msg_rx: aptos_channel::Receiver<PeerId, VerifiedEvent>,
        batch_reader_tx: Sender<BatchReaderCommand>,
        proof_coordinator_tx: Sender<ProofCoordinatorCommand>,
        remote_batch_coordinator_tx: Vec<Sender<BatchCoordinatorCommand>>,
        proof_manager_tx: Sender<ProofManagerCommand>,
    ) -> Self {
        Self {
            network_msg_rx,
            batch_reader_tx,
            proof_coordinator_tx,
            remote_batch_coordinator_tx,
            proof_manager_tx,
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
