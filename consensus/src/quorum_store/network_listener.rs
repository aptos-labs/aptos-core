// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    monitor,
    quorum_store::{
        batch_coordinator::BatchCoordinatorCommand, counters,
        proof_coordinator::ProofCoordinatorCommand, proof_manager::ProofManagerCommand,
    },
    round_manager::VerifiedEvent,
};
use aptos_channels::aptos_channel;
use aptos_logger::prelude::*;
use aptos_types::PeerId;
use futures::StreamExt;
use tokio::sync::mpsc::Sender;

pub(crate) struct NetworkListener {
    network_msg_rx: aptos_channel::Receiver<PeerId, VerifiedEvent>,
    proof_coordinator_tx: Sender<ProofCoordinatorCommand>,
    remote_batch_coordinator_tx: Vec<Sender<BatchCoordinatorCommand>>,
    proof_manager_tx: Sender<ProofManagerCommand>,
}

impl NetworkListener {
    pub(crate) fn new(
        network_msg_rx: aptos_channel::Receiver<PeerId, VerifiedEvent>,
        proof_coordinator_tx: Sender<ProofCoordinatorCommand>,
        remote_batch_coordinator_tx: Vec<Sender<BatchCoordinatorCommand>>,
        proof_manager_tx: Sender<ProofManagerCommand>,
    ) -> Self {
        Self {
            network_msg_rx,
            proof_coordinator_tx,
            remote_batch_coordinator_tx,
            proof_manager_tx,
        }
    }

    pub async fn start(mut self) {
        info!("QS: starting networking");
        while let Some(msg) = self.network_msg_rx.next().await {
            monitor!("qs_network_listener_main_loop", {
                match msg {
                    // TODO: does the assumption have to be that network listener is shutdown first?
                    VerifiedEvent::Shutdown(ack_tx) => {
                        info!("QS: shutdown network listener received");
                        ack_tx
                            .send(())
                            .expect("Failed to send shutdown ack to QuorumStore");
                        break;
                    },
                    VerifiedEvent::SignedDigestMsg(signed_digest) => {
                        let cmd = ProofCoordinatorCommand::AppendSignature(*signed_digest);
                        self.proof_coordinator_tx
                            .send(cmd)
                            .await
                            .expect("Could not send signed_digest to proof_coordinator");
                    },
                    VerifiedEvent::BatchMsg(batch_msg) => {
                        let batch = batch_msg.unpack();
                        counters::DELIVERED_FRAGMENTS_COUNT.inc();
                        let idx = batch.author().to_vec()[0] as usize
                            % self.remote_batch_coordinator_tx.len();
                        trace!(
                            "QS: peer_id {:?},  # network_worker {}, hashed to idx {}",
                            batch.author(),
                            self.remote_batch_coordinator_tx.len(),
                            idx
                        );
                        self.remote_batch_coordinator_tx[idx]
                            .send(BatchCoordinatorCommand::NewBatch(Box::new(batch)))
                            .await
                            .expect("Could not send remote batch");
                    },
                    VerifiedEvent::ProofOfStoreMsg(proof) => {
                        let cmd = ProofManagerCommand::ReceiveProof(*proof);
                        self.proof_manager_tx
                            .send(cmd)
                            .await
                            .expect("could not push Proof proof_of_store");
                    },
                    _ => {
                        unreachable!()
                    },
                };
            });
        }
    }
}
