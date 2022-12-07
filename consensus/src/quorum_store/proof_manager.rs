// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::quorum_store::counters;
use crate::quorum_store::utils::ProofQueue;
use crate::round_manager::VerifiedEvent;
use aptos_crypto::HashValue;
use aptos_logger::prelude::*;
use aptos_types::PeerId;
use channel::aptos_channel;
use consensus_types::common::{Payload, PayloadFilter};
use consensus_types::proof_of_store::LogicalTime;
use consensus_types::request_response::{
    BlockProposalCommand, CleanCommand, ConsensusResponse, WrapperCommand,
};
use futures::StreamExt;
use futures_channel::mpsc::Receiver;
use std::collections::HashSet;

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

    pub(crate) fn handle_clean_request(&mut self, msg: CleanCommand) {
        match msg {
            CleanCommand::CleanRequest(logical_time, digests) => {
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
        }
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
                    }
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
                    Payload::empty()
                } else {
                    debug!(
                        "QS: GetBlockRequest excluded len {}, block len {}",
                        excluded_proofs.len(),
                        proof_block.len()
                    );
                    Payload::InQuorumStore(proof_block)
                });
                match callback.send(Ok(res)) {
                    Ok(_) => (),
                    Err(err) => debug!("BlockResponse receiver not available! error {:?}", err),
                }
            }
        }
    }

    pub async fn start(
        mut self,
        mut proposal_rx: Receiver<BlockProposalCommand>,
        mut clean_rx: Receiver<CleanCommand>,
        mut network_msg_rx: aptos_channel::Receiver<PeerId, VerifiedEvent>,
        // TODO: receive proofs from proof coordinator
    ) {
        loop {
            // TODO: additional main loop counter
            let _timer = counters::WRAPPER_MAIN_LOOP.start_timer();

            tokio::select! {
                Some(msg) = proposal_rx.next() => {
                    self.handle_proposal_request(msg)
                },
                Some(msg) = clean_rx.next() => {
                    self.handle_clean_request(msg)
                }
                Some(msg) = network_msg_rx.next() => {
                   if let VerifiedEvent::ProofOfStoreBroadcast(proof) = msg{
                        debug!("QS: got proof from peer");

                        counters::REMOTE_POS_COUNT.inc();
                        self.proofs_for_consensus.push(*proof);
                    }
                },
            }
        }
    }
}
