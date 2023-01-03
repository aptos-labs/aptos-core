// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::network::{NetworkSender, QuorumStoreSender};
use crate::quorum_store::counters;
use crate::quorum_store::types::BatchId;
use crate::quorum_store::utils::ProofQueue;
use crate::round_manager::VerifiedEvent;
use aptos_channels::aptos_channel;
use aptos_consensus_types::common::{Payload, PayloadFilter, ProofWithData};
use aptos_consensus_types::proof_of_store::{LogicalTime, ProofOfStore};
use aptos_consensus_types::request_response::{
    BlockProposalCommand, CleanCommand, ConsensusResponse,
};
use aptos_crypto::HashValue;
use aptos_logger::prelude::*;
use aptos_types::PeerId;
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
    remaining_proof_num: usize,
}

impl ProofManager {
    pub fn new(epoch: u64) -> Self {
        Self {
            proofs_for_consensus: ProofQueue::new(),
            latest_logical_time: LogicalTime::new(epoch, 0),
            remaining_proof_num: 0,
        }
    }

    pub(crate) async fn handle_local_proof(
        &mut self,
        proof: ProofOfStore,
        network_sender: &mut NetworkSender,
    ) {
        self.proofs_for_consensus.push(proof.clone());
        network_sender.broadcast_proof_of_store(proof).await;
    }

    pub(crate) fn handle_remote_proof(&mut self, proof: ProofOfStore) {
        self.proofs_for_consensus.push(proof.clone());
    }

    pub(crate) fn handle_commit_notification(
        &mut self,
        logical_time: LogicalTime,
        digests: Vec<HashValue>,
    ) {
        debug!("QS: got clean request from execution");
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

    pub(crate) fn handle_proposal_request(&mut self, msg: BlockProposalCommand) {
        match msg {
            // TODO: check what max_txns consensus is using
            BlockProposalCommand::GetBlockRequest(round, max_txns, max_bytes, filter, callback) => {
                // TODO: Pass along to batch_store
                let excluded_proofs: HashSet<HashValue> = match filter {
                    PayloadFilter::Empty => HashSet::new(),
                    PayloadFilter::DirectMempool(_) => {
                        unreachable!()
                    },
                    PayloadFilter::InQuorumStore(proofs) => proofs,
                };

                let (proof_block, remaining_proof_num) = self.proofs_for_consensus.pull_proofs(
                    &excluded_proofs,
                    LogicalTime::new(self.latest_logical_time.epoch(), round),
                    max_txns,
                    max_bytes,
                );
                self.remaining_proof_num = remaining_proof_num;

                let res = ConsensusResponse::GetBlockResponse(if proof_block.is_empty() {
                    Payload::empty(true)
                } else {
                    debug!(
                        "QS: GetBlockRequest excluded len {}, block len {}",
                        excluded_proofs.len(),
                        proof_block.len()
                    );
                    Payload::InQuorumStore(ProofWithData::new(proof_block))
                });
                match callback.send(Ok(res)) {
                    Ok(_) => (),
                    Err(err) => debug!("BlockResponse receiver not available! error {:?}", err),
                }
            },
        }
    }

    pub async fn start(
        mut self,
        mut network_sender: NetworkSender,
        mut proposal_rx: Receiver<BlockProposalCommand>,
        mut proof_rx: tokio::sync::mpsc::Receiver<ProofManagerCommand>,
    ) {
        loop {
            // TODO: additional main loop counter
            let _timer = counters::WRAPPER_MAIN_LOOP.start_timer();

            tokio::select! {
                Some(msg) = proposal_rx.next() => {
                    self.handle_proposal_request(msg);
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
                        },
                    }
                },
            }
        }
    }
}
