// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::types::PersistedValue;
use crate::{
    monitor,
    quorum_store::{batch_generator::BackPressure, counters, utils::ProofQueue},
};
use aptos_consensus_types::{
    common::{Payload, PayloadFilter, ProofWithData, ProofWithDataWithTxnLimit},
    proof_of_store::{BatchInfo, ProofOfStore, ProofOfStoreMsg},
    request_response::{GetPayloadCommand, GetPayloadResponse},
};
use aptos_logger::prelude::*;
use aptos_types::{transaction::SignedTransaction, PeerId};
use futures::StreamExt;
use futures_channel::mpsc::Receiver;
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug)]
pub enum ProofManagerCommand {
    ReceiveProofs(ProofOfStoreMsg),
    ReceiveBatches(Vec<PersistedValue>),
    CommitNotification(u64, Vec<BatchInfo>),
    Shutdown(tokio::sync::oneshot::Sender<()>),
}

pub struct ProofManager {
    proofs_for_consensus: ProofQueue,
    // TODO: Should this be a DashMap?
    batches_without_proof_of_store: HashMap<BatchInfo, Option<Vec<SignedTransaction>>>,
    // Storing the most recent 20 batches added to batches_without_proof_of_store.
    // This lets us remove the older batches from batches_without_proof_of_store whenever required.
    recent_batches_without_proof_of_store: VecDeque<BatchInfo>,
    back_pressure_total_txn_limit: u64,
    remaining_total_txn_num: u64,
    back_pressure_total_proof_limit: u64,
    remaining_total_proof_num: u64,
    allow_batches_without_pos_in_proposal: bool,
}

impl ProofManager {
    pub fn new(
        my_peer_id: PeerId,
        back_pressure_total_txn_limit: u64,
        back_pressure_total_proof_limit: u64,
        allow_batches_without_pos_in_proposal: bool,
    ) -> Self {
        Self {
            proofs_for_consensus: ProofQueue::new(my_peer_id),
            batches_without_proof_of_store: HashMap::new(),
            recent_batches_without_proof_of_store: VecDeque::new(),
            back_pressure_total_txn_limit,
            remaining_total_txn_num: 0,
            back_pressure_total_proof_limit,
            remaining_total_proof_num: 0,
            allow_batches_without_pos_in_proposal,
        }
    }

    pub(crate) fn receive_proofs(&mut self, proofs: Vec<ProofOfStore>) {
        for proof in proofs.into_iter() {
            self.batches_without_proof_of_store.remove(proof.info());
            self.proofs_for_consensus.push(proof);
        }
        (self.remaining_total_txn_num, self.remaining_total_proof_num) =
            self.proofs_for_consensus.remaining_txns_and_proofs();
    }

    pub(crate) fn receive_batches(&mut self, batches: Vec<PersistedValue>) {
        if self.allow_batches_without_pos_in_proposal {
            for mut batch in batches.into_iter() {
                self.batches_without_proof_of_store
                    .insert(batch.batch_info().clone(), batch.take_payload());
                self.recent_batches_without_proof_of_store
                    .push_back(batch.batch_info().clone());
                if self.recent_batches_without_proof_of_store.len() > 20 {
                    self.recent_batches_without_proof_of_store.pop_front();
                }
            }
        }
    }

    pub(crate) fn handle_commit_notification(
        &mut self,
        block_timestamp: u64,
        batches: Vec<BatchInfo>,
    ) {
        trace!(
            "QS: got clean request from execution at block timestamp {}",
            block_timestamp
        );
        for batch in batches.iter() {
            self.batches_without_proof_of_store.remove(batch);
        }
        self.proofs_for_consensus.mark_committed(batches);
        self.proofs_for_consensus
            .handle_updated_block_timestamp(block_timestamp);
        (self.remaining_total_txn_num, self.remaining_total_proof_num) =
            self.proofs_for_consensus.remaining_txns_and_proofs();
    }

    pub(crate) fn handle_proposal_request(&mut self, msg: GetPayloadCommand) {
        match msg {
            GetPayloadCommand::GetPayloadRequest(
                max_txns,
                max_bytes,
                max_inline_txns,
                max_inline_bytes,
                return_non_full,
                filter,
                callback,
            ) => {
                let excluded_batches: HashSet<_> = match filter {
                    PayloadFilter::Empty => HashSet::new(),
                    PayloadFilter::DirectMempool(_) => {
                        unreachable!()
                    },
                    PayloadFilter::InQuorumStore(proofs) => proofs,
                };

                let proof_block = self.proofs_for_consensus.pull_proofs(
                    &excluded_batches,
                    max_txns,
                    max_bytes,
                    return_non_full,
                );

                let mut inline_block: Vec<(BatchInfo, Vec<SignedTransaction>)> = vec![];
                if self.allow_batches_without_pos_in_proposal {
                    // TODO: Add a counter in grafana to monitor how many inline transactions/bytes are added
                    let mut cur_txns: u64 = proof_block.iter().map(|p| p.num_txns()).sum();
                    let mut cur_bytes: u64 = proof_block.iter().map(|p| p.num_bytes()).sum();
                    let mut inline_txns: u64 = 0;
                    let mut inline_bytes: u64 = 0;
                    let proof_batches =
                        proof_block.iter().map(|p| p.info()).collect::<HashSet<_>>();

                    self.batches_without_proof_of_store.retain(|batch, _| {
                        !batch.is_expired()
                            && !excluded_batches.contains(batch)
                            && !proof_batches.contains(batch)
                            && self.recent_batches_without_proof_of_store.contains(batch)
                    });
                    for (batch, txns) in self.batches_without_proof_of_store.iter_mut() {
                        if let Some(txns) = txns {
                            // TODO: Should we calculate batch size by summing up sizes of individual txns?
                            // TODO: We are including any batch that satisfies the size requirements here.
                            // Should we prioritize based on other criteria like expiration time, etc?
                            if cur_txns + txns.len() as u64 <= max_txns
                                && cur_bytes + batch.num_bytes() <= max_bytes
                                && inline_txns + txns.len() as u64 <= max_inline_txns
                                && inline_bytes + batch.num_bytes() <= max_inline_bytes
                            {
                                inline_txns += txns.len() as u64;
                                inline_bytes += batch.num_bytes();
                                cur_txns += txns.len() as u64;
                                cur_bytes += batch.num_bytes();
                                // TODO: Can cloning be avoided here?
                                inline_block.push((batch.clone(), txns.clone()));
                            }
                        }
                    }
                }

                let res = GetPayloadResponse::GetPayloadResponse(
                    if proof_block.is_empty() && inline_block.is_empty() {
                        Payload::empty(true)
                    } else if inline_block.is_empty() {
                        trace!(
                            "QS: GetBlockRequest excluded len {}, block len {}",
                            excluded_batches.len(),
                            proof_block.len()
                        );
                        Payload::InQuorumStore(ProofWithData::new(proof_block))
                    } else {
                        trace!(
                            "QS: GetBlockRequest excluded len {}, block len {}, inline len {}",
                            excluded_batches.len(),
                            proof_block.len(),
                            inline_block.len()
                        );
                        // TODO: Need to calcuale max_txns_to_execute correctly here.
                        Payload::QuorumStoreInlineHybrid(
                            inline_block,
                            ProofWithDataWithTxnLimit::new(
                                ProofWithData::new(proof_block),
                                Some(max_txns as usize),
                            ),
                        )
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
        back_pressure_tx: tokio::sync::mpsc::Sender<BackPressure>,
        mut proposal_rx: Receiver<GetPayloadCommand>,
        mut proof_rx: tokio::sync::mpsc::Receiver<ProofManagerCommand>,
    ) {
        let mut back_pressure = BackPressure {
            txn_count: false,
            proof_count: false,
        };

        loop {
            let _timer = counters::PROOF_MANAGER_MAIN_LOOP.start_timer();

            tokio::select! {
                    Some(msg) = proposal_rx.next() => monitor!("proof_manager_handle_proposal", {
                        self.handle_proposal_request(msg);

                        let updated_back_pressure = self.qs_back_pressure();
                        if updated_back_pressure != back_pressure {
                            back_pressure = updated_back_pressure;
                            if back_pressure_tx.send(back_pressure).await.is_err() {
                                debug!("Failed to send back_pressure for proposal");
                            }
                        }
                    }),
                    Some(msg) = proof_rx.recv() => {
                        monitor!("proof_manager_handle_command", {
                        match msg {
                            ProofManagerCommand::Shutdown(ack_tx) => {
                                ack_tx
                                    .send(())
                                    .expect("Failed to send shutdown ack to QuorumStore");
                                break;
                            },
                            ProofManagerCommand::ReceiveProofs(proofs) => {
                                self.receive_proofs(proofs.take());
                            },
                            ProofManagerCommand::ReceiveBatches(batches) => {
                                self.receive_batches(batches);
                            }
                            ProofManagerCommand::CommitNotification(block_timestamp, batches) => {
                                self.handle_commit_notification(
                                    block_timestamp,
                                    batches,
                                );
                            },
                        }
                        let updated_back_pressure = self.qs_back_pressure();
                        if updated_back_pressure != back_pressure {
                            back_pressure = updated_back_pressure;
                            if back_pressure_tx.send(back_pressure).await.is_err() {
                                debug!("Failed to send back_pressure for commit notification");
                            }
                        }
                    })
                }
            }
        }
    }
}
