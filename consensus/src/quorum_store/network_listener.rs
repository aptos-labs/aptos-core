// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    monitor,
    quorum_store::{
        batch_coordinator::BatchCoordinatorCommand, counters,
        proof_coordinator::ProofCoordinatorCommand, proof_manager::ProofManagerCommand,
        tracing::observe_batch,
    },
    round_manager::VerifiedEvent,
};
use aptos_channels::aptos_channel;
use aptos_logger::prelude::*;
use aptos_types::PeerId;
use futures::StreamExt;
use std::time::Duration;
use tokio::sync::mpsc::Sender;

pub(crate) struct NetworkListener {
    network_msg_rx: aptos_channel::Receiver<PeerId, (PeerId, VerifiedEvent)>,
    proof_coordinator_tx: Sender<ProofCoordinatorCommand>,
    remote_batch_coordinator_tx: Vec<Sender<BatchCoordinatorCommand>>,
    proof_manager_tx: Sender<ProofManagerCommand>,
}

impl NetworkListener {
    pub(crate) fn new(
        network_msg_rx: aptos_channel::Receiver<PeerId, (PeerId, VerifiedEvent)>,
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
        let mut next_batch_coordinator_idx = 0;
        let batch_expiry_gap_when_init_usecs = Duration::from_secs(60).as_micros() as u64;
        while let Some((sender, msg)) = self.network_msg_rx.next().await {
            monitor!("qs_network_listener_main_loop", {
                match msg {
                    // TODO: does the assumption have to be that network listener is shutdown first?
                    VerifiedEvent::Shutdown(ack_tx) => {
                        counters::QUORUM_STORE_MSG_COUNT
                            .with_label_values(&["NetworkListener::shutdown"])
                            .inc();
                        info!("QS: shutdown network listener received");
                        ack_tx
                            .send(())
                            .expect("Failed to send shutdown ack to QuorumStore");
                        break;
                    },
                    VerifiedEvent::SignedBatchInfo(signed_batch_infos) => {
                        counters::QUORUM_STORE_MSG_COUNT
                            .with_label_values(&["NetworkListener::signedbatchinfo"])
                            .inc();
                        let cmd =
                            ProofCoordinatorCommand::AppendSignature(sender, *signed_batch_infos);
                        self.proof_coordinator_tx
                            .send(cmd)
                            .await
                            .expect("Could not send signed_batch_info to proof_coordinator");
                    },
                    VerifiedEvent::BatchMsg(batch_msg) => {
                        counters::QUORUM_STORE_MSG_COUNT
                            .with_label_values(&["NetworkListener::batchmsg"])
                            .inc();
                        // Batch msg verify function alreay ensures that the batch_msg is not empty.
                        let author = batch_msg.author().expect("Empty batch message");
                        let batches = batch_msg.take();
                        counters::RECEIVED_BATCH_MSG_COUNT.inc();

                        // Round-robin assignment to batch coordinator.
                        let idx = next_batch_coordinator_idx;
                        next_batch_coordinator_idx = (next_batch_coordinator_idx + 1)
                            % self.remote_batch_coordinator_tx.len();
                        trace!(
                            "QS: peer_id {:?},  # network_worker {}, hashed to idx {}",
                            author,
                            self.remote_batch_coordinator_tx.len(),
                            idx
                        );
                        for batch in &batches {
                            let approx_created_ts_usecs = batch
                                .expiration()
                                .saturating_sub(batch_expiry_gap_when_init_usecs);
                            observe_batch(
                                approx_created_ts_usecs,
                                batch.author(),
                                "NETWORKRECEIVE",
                            );
                        }
                        counters::BATCH_COORDINATOR_NUM_BATCH_REQS
                            .with_label_values(&[&idx.to_string()])
                            .inc();
                        self.remote_batch_coordinator_tx[idx]
                            .send(BatchCoordinatorCommand::NewBatches(author, batches))
                            .await
                            .expect("Could not send remote batch");
                    },
                    VerifiedEvent::ProofOfStoreMsg(proofs) => {
                        counters::QUORUM_STORE_MSG_COUNT
                            .with_label_values(&["NetworkListener::proofofstore"])
                            .inc();
                        let cmd = ProofManagerCommand::ReceiveProofs(*proofs);
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
