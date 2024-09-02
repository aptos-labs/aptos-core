// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    monitor,
    quorum_store::{
        batch_coordinator::BatchCoordinatorCommand, counters,
        proof_coordinator::ProofCoordinatorCommand, proof_manager::ProofManagerCommand,
    },
    round_manager::{UnverifiedEvent, VerifiedEvent},
};
use aptos_channels::aptos_channel;
use aptos_logger::prelude::*;
use aptos_types::PeerId;
use futures::StreamExt;
use tokio::sync::mpsc::Sender;

pub(crate) struct NetworkListener {
    verified_network_msg_rx: aptos_channel::Receiver<PeerId, VerifiedEvent>,
    unverified_network_msg_rx: aptos_channel::Receiver<PeerId, UnverifiedEvent>,
    proof_coordinator_tx: Sender<ProofCoordinatorCommand>,
    remote_batch_coordinator_tx: Vec<Sender<BatchCoordinatorCommand>>,
    proof_manager_tx: Sender<ProofManagerCommand>,
}

impl NetworkListener {
    pub(crate) fn new(
        verified_network_msg_rx: aptos_channel::Receiver<PeerId, VerifiedEvent>,
        unverified_network_msg_rx: aptos_channel::Receiver<PeerId, UnverifiedEvent>,
        proof_coordinator_tx: Sender<ProofCoordinatorCommand>,
        remote_batch_coordinator_tx: Vec<Sender<BatchCoordinatorCommand>>,
        proof_manager_tx: Sender<ProofManagerCommand>,
    ) -> Self {
        Self {
            verified_network_msg_rx,
            unverified_network_msg_rx,
            proof_coordinator_tx,
            remote_batch_coordinator_tx,
            proof_manager_tx,
        }
    }

    pub async fn start(mut self) {
        info!("QS: starting networking");
        loop {
            tokio::select! {
                Some(msg) = self.verified_network_msg_rx.next() => monitor!("qs_network_listener_main_loop", {
                    match msg {
                        // TODO: does the assumption have to be that network listener is shutdown first?
                        VerifiedEvent::Shutdown(ack_tx) => {
                            info!("QS: shutdown network listener received");
                            ack_tx
                                .send(())
                                .expect("Failed to send shutdown ack to QuorumStore");
                            break;
                        },
                        VerifiedEvent::SignedBatchInfo(signed_batch_infos) => {
                            let cmd = ProofCoordinatorCommand::AppendSignature((*signed_batch_infos, true));
                            self.proof_coordinator_tx
                                .send(cmd)
                                .await
                                .expect("Could not send signed_batch_info to proof_coordinator");
                        },
                        VerifiedEvent::BatchMsg(batch_msg) => {
                            let author = batch_msg.author();
                            let batches = batch_msg.take();
                            counters::RECEIVED_BATCH_MSG_COUNT.inc();

                            let idx =
                                author.to_vec()[0] as usize % self.remote_batch_coordinator_tx.len();
                            trace!(
                                "QS: peer_id {:?},  # network_worker {}, hashed to idx {}",
                                author,
                                self.remote_batch_coordinator_tx.len(),
                                idx
                            );
                            self.remote_batch_coordinator_tx[idx]
                                .send(BatchCoordinatorCommand::NewBatches(author, batches))
                                .await
                                .expect("Could not send remote batch");
                        },
                        VerifiedEvent::ProofOfStoreMsg(proofs) => {
                            let cmd = ProofManagerCommand::ReceiveProofs(*proofs);
                            self.proof_manager_tx
                                .send(cmd)
                                .await
                                .expect("could not push Proof proof_of_store");
                        },
                        _ => {
                            unreachable!()
                        },
                    }
                }),
                Some(msg) = self.unverified_network_msg_rx.next() => monitor!("qs_unverified_network_listener_main_loop", {
                    match msg {
                        UnverifiedEvent::SignedBatchInfo(signed_batch_infos) => {
                            let cmd = ProofCoordinatorCommand::AppendSignature((*signed_batch_infos, false));
                            self.proof_coordinator_tx
                                .send(cmd)
                                .await
                                .expect("Could not send signed_batch_info to proof_coordinator");
                        },
                        _ => {
                            unreachable!()
                        },
                    }
                }),
            }
        }
        // while let Some(msg) = self.verified_network_msg_rx.next().await {
        //     monitor!("qs_network_listener_main_loop", {
        //         match msg {
        //             // TODO: does the assumption have to be that network listener is shutdown first?
        //             VerifiedEvent::Shutdown(ack_tx) => {
        //                 info!("QS: shutdown network listener received");
        //                 ack_tx
        //                     .send(())
        //                     .expect("Failed to send shutdown ack to QuorumStore");
        //                 break;
        //             },
        //             VerifiedEvent::SignedBatchInfo(signed_batch_infos) => {
        //                 let cmd = ProofCoordinatorCommand::AppendSignature(*signed_batch_infos);
        //                 self.proof_coordinator_tx
        //                     .send(cmd)
        //                     .await
        //                     .expect("Could not send signed_batch_info to proof_coordinator");
        //             },
        //             VerifiedEvent::BatchMsg(batch_msg) => {
        //                 let author = batch_msg.author();
        //                 let batches = batch_msg.take();
        //                 counters::RECEIVED_BATCH_MSG_COUNT.inc();

        //                 let idx =
        //                     author.to_vec()[0] as usize % self.remote_batch_coordinator_tx.len();
        //                 trace!(
        //                     "QS: peer_id {:?},  # network_worker {}, hashed to idx {}",
        //                     author,
        //                     self.remote_batch_coordinator_tx.len(),
        //                     idx
        //                 );
        //                 self.remote_batch_coordinator_tx[idx]
        //                     .send(BatchCoordinatorCommand::NewBatches(author, batches))
        //                     .await
        //                     .expect("Could not send remote batch");
        //             },
        //             VerifiedEvent::ProofOfStoreMsg(proofs) => {
        //                 let cmd = ProofManagerCommand::ReceiveProofs(*proofs);
        //                 self.proof_manager_tx
        //                     .send(cmd)
        //                     .await
        //                     .expect("could not push Proof proof_of_store");
        //             },
        //             _ => {
        //                 unreachable!()
        //             },
        //         };
        //     });
        // }
    }
}
