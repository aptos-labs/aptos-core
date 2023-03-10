// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    network::{NetworkSender, QuorumStoreSender},
    quorum_store::{batch_generator::BackPressure, counters, utils::ProofQueue},
};
use aptos_consensus_types::{
    common::{Payload, PayloadFilter, ProofWithData},
    proof_of_store::{LogicalTime, ProofOfStore},
    request_response::{GetPayloadCommand, GetPayloadResponse},
};
use aptos_crypto::HashValue;
use aptos_logger::prelude::*;
use futures::StreamExt;
use futures_channel::mpsc::Receiver;
use std::collections::HashSet;

#[derive(Debug)]
pub enum ProofManagerCommand {
    LocalProof(ProofOfStore),
    RemoteProof(ProofOfStore),
    CommitNotification(LogicalTime, Vec<HashValue>),
    Shutdown(tokio::sync::oneshot::Sender<()>),
}

pub struct ProofManager {
    proofs_for_consensus: ProofQueue,
    latest_logical_time: LogicalTime,
    back_pressure_total_txn_limit: u64,
    remaining_total_txn_num: u64,
    back_pressure_total_proof_limit: u64,
    remaining_total_proof_num: u64,
}

impl ProofManager {
    pub fn new(
        epoch: u64,
        back_pressure_total_txn_limit: u64,
        back_pressure_total_proof_limit: u64,
    ) -> Self {
        Self {
            proofs_for_consensus: ProofQueue::new(),
            latest_logical_time: LogicalTime::new(epoch, 0),
            back_pressure_total_txn_limit,
            remaining_total_txn_num: 0,
            back_pressure_total_proof_limit,
            remaining_total_proof_num: 0,
        }
    }

    pub(crate) async fn handle_local_proof(
        &mut self,
        proof: ProofOfStore,
        network_sender: &mut NetworkSender,
    ) {
        self.proofs_for_consensus.push(proof.clone(), true);
        network_sender.broadcast_proof_of_store(proof).await;
    }

    pub(crate) fn handle_remote_proof(&mut self, proof: ProofOfStore) {
        self.proofs_for_consensus.push(proof, false);
    }

    pub(crate) fn handle_commit_notification(
        &mut self,
        logical_time: LogicalTime,
        digests: Vec<HashValue>,
    ) {
        trace!(
            "QS: got clean request from execution at epoch {}, round {}",
            logical_time.epoch(),
            logical_time.round()
        );
        assert_eq!(
            self.latest_logical_time.epoch(),
            logical_time.epoch(),
            "Wrong epoch"
        );
        assert!(
            self.latest_logical_time <= logical_time,
            "Decreasing logical time"
        );
        self.latest_logical_time = logical_time;
        self.proofs_for_consensus.mark_committed(digests);
    }

    pub(crate) fn handle_proposal_request(&mut self, msg: GetPayloadCommand) {
        match msg {
            // TODO: check what max_txns consensus is using
            GetPayloadCommand::GetPayloadRequest(
                round,
                max_txns,
                max_bytes,
                return_non_full,
                filter,
                callback,
            ) => {
                // TODO: Pass along to batch_store
                let excluded_proofs: HashSet<HashValue> = match filter {
                    PayloadFilter::Empty => HashSet::new(),
                    PayloadFilter::DirectMempool(_) => {
                        unreachable!()
                    },
                    PayloadFilter::InQuorumStore(proofs) => proofs,
                };

                let proof_block = self.proofs_for_consensus.pull_proofs(
                    &excluded_proofs,
                    LogicalTime::new(self.latest_logical_time.epoch(), round),
                    max_txns,
                    max_bytes,
                    return_non_full,
                );
                (self.remaining_total_txn_num, self.remaining_total_proof_num) = self
                    .proofs_for_consensus
                    .num_total_txns_and_proofs(LogicalTime::new(
                        self.latest_logical_time.epoch(),
                        round,
                    ));

                let res = GetPayloadResponse::GetPayloadResponse(
                    if proof_block.is_empty() {
                        Payload::empty(true)
                    } else {
                        trace!(
                            "QS: GetBlockRequest excluded len {}, block len {}",
                            excluded_proofs.len(),
                            proof_block.len()
                        );
                        Payload::InQuorumStore(ProofWithData::new(proof_block))
                    },
                );
                match callback.send(Ok(res)) {
                    Ok(_) => (),
                    Err(err) => debug!("BlockResponse receiver not available! error {:?}", err),
                }
            },
        }
    }

    /// return true when quorum store is back pressured
    pub(crate) fn qs_back_pressure(&self) -> BackPressure {
        BackPressure {
            txn_count: self.remaining_total_txn_num > self.back_pressure_total_txn_limit,
            proof_count: self.remaining_total_proof_num > self.back_pressure_total_proof_limit,
        }
    }

    pub async fn start(
        mut self,
        mut network_sender: NetworkSender,
        back_pressure_tx: tokio::sync::mpsc::Sender<BackPressure>,
        mut proposal_rx: Receiver<GetPayloadCommand>,
        mut proof_rx: tokio::sync::mpsc::Receiver<ProofManagerCommand>,
    ) {
        let mut back_pressure = BackPressure {
            txn_count: false,
            proof_count: false,
        };

        loop {
            // TODO: additional main loop counter
            let _timer = counters::WRAPPER_MAIN_LOOP.start_timer();

            tokio::select! {
                Some(msg) = proposal_rx.next() => {
                    self.handle_proposal_request(msg);

                    let updated_back_pressure = self.qs_back_pressure();
                    if updated_back_pressure != back_pressure {
                        back_pressure = updated_back_pressure;
                        if back_pressure_tx.send(back_pressure).await.is_err() {
                            debug!("Failed to send back_pressure for proposal");
                        }
                    }
                },
                Some(msg) = proof_rx.recv() => {
                    match msg {
                        ProofManagerCommand::Shutdown(ack_tx) => {
                            ack_tx
                                .send(())
                                .expect("Failed to send shutdown ack to QuorumStore");
                            break;
                        },
                        ProofManagerCommand::LocalProof(proof) => {
                            self.handle_local_proof(proof, &mut network_sender).await;
                        },
                        ProofManagerCommand::RemoteProof(proof) => {
                            self.handle_remote_proof(proof);
                        },
                        ProofManagerCommand::CommitNotification(logical_time, digests) => {
                            self.handle_commit_notification(logical_time, digests);

                            // update the backpressure upon new commit round
                            (self.remaining_total_txn_num, self.remaining_total_proof_num) =
                                self.proofs_for_consensus.num_total_txns_and_proofs(logical_time);
                            // TODO: keeping here for metrics, might be part of the backpressure in the future?
                            self.proofs_for_consensus.clean_local_proofs(logical_time);
                            let updated_back_pressure = self.qs_back_pressure();
                            if updated_back_pressure != back_pressure {
                                back_pressure = updated_back_pressure;
                                if back_pressure_tx.send(back_pressure).await.is_err() {
                                    debug!("Failed to send back_pressure for commit notification");
                                }
                            }
                        },
                    }
                },
            }
        }
    }
}
