// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::batch_store::BatchStore;
use crate::{
    monitor,
    quorum_store::{batch_generator::BackPressure, batch_proof_queue::BatchProofQueue, counters},
};
use aptos_consensus_types::{
    common::{Payload, PayloadFilter, ProofWithData, TxnSummaryWithExpiration},
    payload::{OptQuorumStorePayload, PayloadExecutionLimit, RaptrPayload, SubBlocks},
    proof_of_store::{BatchInfo, ProofOfStore, ProofOfStoreMsg},
    request_response::{GetPayloadCommand, GetPayloadResponse},
    utils::PayloadTxnsSize,
};
use aptos_logger::prelude::*;
use aptos_types::PeerId;
use futures::StreamExt;
use futures_channel::mpsc::Receiver;
use itertools::Itertools;
use raptr::raptr::types::N_SUB_BLOCKS;
use rayon::prelude::IntoParallelRefIterator;
use std::{cmp::min, collections::HashSet, sync::Arc, time::Duration};

#[derive(Debug)]
pub enum ProofManagerCommand {
    ReceiveProofs(ProofOfStoreMsg),
    ReceiveBatches(Vec<(BatchInfo, Vec<TxnSummaryWithExpiration>)>),
    CommitNotification(u64, Vec<BatchInfo>),
    Shutdown(tokio::sync::oneshot::Sender<()>),
}

pub struct ProofManager {
    batch_proof_queue: BatchProofQueue,
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
        batch_store: Arc<BatchStore>,
        allow_batches_without_pos_in_proposal: bool,
        batch_expiry_gap_when_init_usecs: u64,
        max_batches_per_pull: usize,
    ) -> Self {
        Self {
            batch_proof_queue: BatchProofQueue::new(
                my_peer_id,
                batch_store,
                batch_expiry_gap_when_init_usecs,
                max_batches_per_pull,
            ),
            back_pressure_total_txn_limit,
            remaining_total_txn_num: 0,
            back_pressure_total_proof_limit,
            remaining_total_proof_num: 0,
            allow_batches_without_pos_in_proposal,
        }
    }

    pub(crate) fn receive_proofs(&mut self, proofs: Vec<ProofOfStore>) {
        for proof in proofs.into_iter() {
            self.batch_proof_queue.insert_proof(proof);
        }
        self.update_remaining_txns_and_proofs();
    }

    fn update_remaining_txns_and_proofs(&mut self) {
        // sample!(
        //     SampleRate::Duration(Duration::from_millis(200)),
        //     (self.remaining_total_txn_num, self.remaining_total_proof_num) =
        //         self.batch_proof_queue.remaining_txns_and_proofs();
        // );
    }

    pub(crate) fn receive_batches(
        &mut self,
        batch_summaries: Vec<(BatchInfo, Vec<TxnSummaryWithExpiration>)>,
    ) {
        self.batch_proof_queue.insert_batches(batch_summaries);
        self.update_remaining_txns_and_proofs();
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
        self.batch_proof_queue.mark_committed(batches);
        self.batch_proof_queue
            .handle_updated_block_timestamp(block_timestamp);
        self.update_remaining_txns_and_proofs();
    }

    pub(crate) fn handle_proposal_request(&mut self, msg: GetPayloadCommand) {
        let GetPayloadCommand::GetPayloadRequest(request) = msg;

        let excluded_batches: HashSet<_> = match request.filter {
            PayloadFilter::Empty => HashSet::new(),
            PayloadFilter::DirectMempool(_) => {
                unreachable!()
            },
            PayloadFilter::InQuorumStore(batches) => batches,
        };

        let (proof_block, txns_with_proof_size, cur_unique_txns, proof_queue_fully_utilized) =
            self.batch_proof_queue.pull_proofs(
                &excluded_batches,
                request.max_txns,
                request.max_txns_after_filtering,
                request.soft_max_txns_after_filtering,
                request.return_non_full,
                request.block_timestamp,
            );

        // counters::NUM_BATCHES_WITHOUT_PROOF_OF_STORE
        //     .observe(self.batch_proof_queue.num_batches_without_proof() as f64);
        counters::PROOF_QUEUE_FULLY_UTILIZED
            .observe(if proof_queue_fully_utilized { 1.0 } else { 0.0 });

        let (mut opt_batches, opt_batch_txns_size) =
            // TODO(ibalajiarun): Support unique txn calculation
            if let Some(ref params) = request.maybe_optqs_payload_pull_params {
                let max_opt_batch_txns_size = request.max_txns - txns_with_proof_size;
                let max_opt_batch_txns_after_filtering = request.max_txns_after_filtering - cur_unique_txns;
                let (opt_batches, opt_payload_size, _) =
                    self.batch_proof_queue.pull_batches(
                        &excluded_batches
                            .iter()
                            .cloned()
                            .chain(proof_block.iter().map(|proof| proof.info().clone()))
                            .collect(),
                        &params.exclude_authors,
                        max_opt_batch_txns_size,
                        max_opt_batch_txns_after_filtering,
                        request.soft_max_txns_after_filtering,
                        request.return_non_full,
                        request.block_timestamp,
                        Some(params.minimum_batch_age_usecs),
                    );
                (opt_batches, opt_payload_size)
            } else {
                (Vec::new(), PayloadTxnsSize::zero())
            };

        let cur_txns = txns_with_proof_size + opt_batch_txns_size;
        let (inline_block, inline_block_size) =
            if self.allow_batches_without_pos_in_proposal && proof_queue_fully_utilized {
                let mut max_inline_txns_to_pull = request
                    .max_txns
                    .saturating_sub(cur_txns)
                    .minimum(request.max_inline_txns);
                max_inline_txns_to_pull.set_count(min(
                    max_inline_txns_to_pull.count(),
                    request
                        .max_txns_after_filtering
                        .saturating_sub(cur_unique_txns),
                ));
                let (inline_batches, inline_payload_size, _) =
                    self.batch_proof_queue.pull_batches_with_transactions(
                        &excluded_batches
                            .iter()
                            .cloned()
                            .chain(proof_block.iter().map(|proof| proof.info().clone()))
                            .chain(opt_batches.clone())
                            .collect(),
                        max_inline_txns_to_pull,
                        request.max_txns_after_filtering,
                        request.soft_max_txns_after_filtering,
                        request.return_non_full,
                        request.block_timestamp,
                    );
                (inline_batches, inline_payload_size)
            } else {
                (Vec::new(), PayloadTxnsSize::zero())
            };
        counters::NUM_INLINE_BATCHES.observe(inline_block.len() as f64);
        counters::NUM_INLINE_TXNS.observe(inline_block_size.count() as f64);

        let proof_ratio = txns_with_proof_size.count() as f64
            / (txns_with_proof_size.count() as f64 + opt_batch_txns_size.count() as f64);
        counters::BATCH_PROOF_RATIO.observe(proof_ratio);

        let response = if request.maybe_optqs_payload_pull_params.is_some() {
            let mut sub_blocks = SubBlocks::default();

            fn div_ceil(dividend: usize, divisor: usize) -> usize {
                if dividend % divisor == 0 {
                    dividend / divisor
                } else {
                    dividend / divisor + 1
                }
            }

            let num_chunks = sub_blocks.len();
            let mut chunks_remaining = num_chunks;
            while chunks_remaining > 0 {
                let chunk_size = div_ceil(opt_batches.len(), chunks_remaining);
                let remaining = opt_batches.split_off(chunk_size);
                sub_blocks[num_chunks - chunks_remaining] = opt_batches.into();
                opt_batches = remaining;

                chunks_remaining -= 1;
            }

            Payload::Raptr(RaptrPayload::new(proof_block.into(), sub_blocks))
        } else if proof_block.is_empty() && inline_block.is_empty() {
            Payload::empty(true, self.allow_batches_without_pos_in_proposal)
        } else {
            trace!(
                "QS: GetBlockRequest excluded len {}, block len {}, inline len {}",
                excluded_batches.len(),
                proof_block.len(),
                inline_block.len()
            );
            Payload::QuorumStoreInlineHybrid(inline_block, ProofWithData::new(proof_block), None)
        };

        let res = GetPayloadResponse::GetPayloadResponse(response);
        match request.callback.send(Ok(res)) {
            Ok(_) => (),
            Err(err) => debug!("BlockResponse receiver not available! error {:?}", err),
        }
    }

    /// return true when quorum store is back pressured
    pub(crate) fn qs_back_pressure(&self) -> BackPressure {
        if self.remaining_total_txn_num > self.back_pressure_total_txn_limit
            || self.remaining_total_proof_num > self.back_pressure_total_proof_limit
        {
            sample!(
                SampleRate::Duration(Duration::from_millis(200)),
                info!(
                    "Quorum store is back pressured with {} txns, limit: {}, proofs: {}, limit: {}",
                    self.remaining_total_txn_num,
                    self.back_pressure_total_txn_limit,
                    self.remaining_total_proof_num,
                    self.back_pressure_total_proof_limit
                );
            );
        }

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
                                counters::QUORUM_STORE_MSG_COUNT.with_label_values(&["ProofManager::shutdown"]).inc();
                                ack_tx
                                    .send(())
                                    .expect("Failed to send shutdown ack to QuorumStore");
                                break;
                            },
                            ProofManagerCommand::ReceiveProofs(proofs) => {
                                counters::QUORUM_STORE_MSG_COUNT.with_label_values(&["ProofManager::receive_proofs"]).inc();
                                monitor!("proof_manager_handle_receive_proofs", self.receive_proofs(proofs.take()));
                            },
                            ProofManagerCommand::ReceiveBatches(batches) => {
                                counters::QUORUM_STORE_MSG_COUNT.with_label_values(&["ProofManager::receive_batches"]).inc();
                                monitor!("proof_manager_handle_receive_batches", self.receive_batches(batches));
                            }
                            ProofManagerCommand::CommitNotification(block_timestamp, batches) => {
                                counters::QUORUM_STORE_MSG_COUNT.with_label_values(&["ProofManager::commit_notification"]).inc();
                                monitor!("proof_manager_handle_commit_notif", self.handle_commit_notification(
                                    block_timestamp,
                                    batches,
                                ));
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
