// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::batch_store::BatchStore;
use crate::{
    monitor,
    quorum_store::{batch_generator::BackPressure, batch_proof_queue::BatchProofQueue, counters},
};
use aptos_consensus_types::{
    common::{Payload, PayloadFilter, TxnSummaryWithExpiration},
    payload::{OptQuorumStorePayload, PayloadExecutionLimit},
    proof_of_store::{BatchInfoExt, ProofOfStore, ProofOfStoreMsg, TBatchInfo},
    request_response::{GetPayloadCommand, GetPayloadResponse},
    utils::PayloadTxnsSize,
};
use aptos_logger::prelude::*;
use aptos_metrics_core::TimerHelper;
use aptos_short_hex_str::AsShortHexStr;
use aptos_types::PeerId;
use futures::StreamExt;
use futures_channel::mpsc::Receiver;
use std::{cmp::min, collections::HashSet, sync::Arc, time::{Duration, Instant}};

#[derive(Debug)]
pub enum ProofManagerCommand {
    ReceiveProofs(ProofOfStoreMsg<BatchInfoExt>),
    ReceiveBatches(Vec<(BatchInfoExt, Vec<TxnSummaryWithExpiration>)>),
    CommitNotification(u64, Vec<BatchInfoExt>),
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
    ) -> Self {
        Self {
            batch_proof_queue: BatchProofQueue::new(
                my_peer_id,
                batch_store,
                batch_expiry_gap_when_init_usecs,
            ),
            back_pressure_total_txn_limit,
            remaining_total_txn_num: 0,
            back_pressure_total_proof_limit,
            remaining_total_proof_num: 0,
            allow_batches_without_pos_in_proposal,
        }
    }

    pub(crate) fn receive_proofs(&mut self, proofs: Vec<ProofOfStore<BatchInfoExt>>) {
        for proof in proofs.into_iter() {
            self.batch_proof_queue.insert_proof(proof);
        }
        self.update_remaining_txns_and_proofs();
    }

    fn update_remaining_txns_and_proofs(&mut self) {
        sample!(
            SampleRate::Duration(Duration::from_millis(200)),
            (self.remaining_total_txn_num, self.remaining_total_proof_num) =
                self.batch_proof_queue.remaining_txns_and_proofs();
        );
    }

    pub(crate) fn receive_batches(
        &mut self,
        batch_summaries: Vec<(BatchInfoExt, Vec<TxnSummaryWithExpiration>)>,
    ) {
        self.batch_proof_queue.insert_batches(batch_summaries);
        self.update_remaining_txns_and_proofs();
    }

    pub(crate) fn handle_commit_notification(
        &mut self,
        block_timestamp: u64,
        batches: Vec<BatchInfoExt>,
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

        let pull_start = Instant::now();
        let (all_items, _all_txns_size, all_unique_txns, is_full) =
            self.batch_proof_queue.pull_all(
                &excluded_batches,
                request.max_txns,
                request.max_txns_after_filtering,
                request.soft_max_txns_after_filtering,
                request.return_non_full,
                request.block_timestamp,
            );
        counters::PULL_ALL_DURATION.observe_duration(pull_start.elapsed());

        counters::NUM_BATCHES_WITHOUT_PROOF_OF_STORE
            .observe(self.batch_proof_queue.num_batches_without_proof() as f64);

        // is_full means the block was filled; proof_queue_fully_utilized = is_full
        let proof_queue_fully_utilized = is_full;
        counters::PROOF_QUEUE_FULLY_UTILIZED
            .observe(if proof_queue_fully_utilized { 1.0 } else { 0.0 });

        // Partition into proof items and non-proof items
        let mut proof_block = Vec::new();
        let mut non_proof_items = Vec::new();
        for item in all_items {
            if item.proof.is_some() {
                let proof = item.proof.clone().unwrap();
                let bucket = proof.gas_bucket_start();
                counters::pos_to_pull(
                    bucket,
                    item.proof_insertion_time
                        .expect("proof must exist due to filter")
                        .elapsed()
                        .as_secs_f64(),
                );
                proof_block.push(proof);
            } else {
                non_proof_items.push(item);
            }
        }

        // opt_batches: non-proof items passing exclude_authors + min_batch_age
        let batch_expiry_gap = self.batch_proof_queue.batch_expiry_gap_when_init_usecs();
        let (opt_batches, remaining_non_proof): (Vec<_>, Vec<_>) =
            if let Some(ref params) = request.maybe_optqs_payload_pull_params {
                let max_create_ts = aptos_infallible::duration_since_epoch().as_micros() as u64
                    - params.minimum_batch_age_usecs;
                non_proof_items.into_iter().partition(|item| {
                    !params.exclude_authors.contains(&item.info.author())
                        && item
                            .info
                            .expiration()
                            .saturating_sub(batch_expiry_gap)
                            <= max_create_ts
                })
            } else {
                (Vec::new(), non_proof_items)
            };

        // Record per-phase metrics for opt_batches
        let opt_batches: Vec<BatchInfoExt> =
            opt_batches.into_iter().map(|item| item.info).collect();
        let opt_batch_txns_size: PayloadTxnsSize =
            opt_batches.iter().fold(PayloadTxnsSize::zero(), |acc, b| acc + b.size());
        counters::CONSENSUS_PULL_NUM_TXNS
            .observe_with(&["optbatch"], opt_batch_txns_size.count() as f64);
        counters::CONSENSUS_PULL_SIZE_IN_BYTES
            .observe_with(&["optbatch"], opt_batch_txns_size.size_in_bytes() as f64);

        // Record too-young skipped batches from the partition
        if let Some(ref params) = request.maybe_optqs_payload_pull_params {
            let max_create_ts = aptos_infallible::duration_since_epoch().as_micros() as u64
                - params.minimum_batch_age_usecs;
            for item in &remaining_non_proof {
                let batch_create_ts = item
                    .info
                    .expiration()
                    .saturating_sub(batch_expiry_gap);
                if batch_create_ts > max_create_ts {
                    counters::BATCH_SKIPPED_TOO_YOUNG
                        .with_label_values(&[item.info.author().short_str().as_str()])
                        .inc();
                }
            }
        }

        // inline_block: remaining non-proof items, capped at max_inline_txns
        let txns_with_proof_size: PayloadTxnsSize =
            proof_block.iter().fold(PayloadTxnsSize::zero(), |acc, p| acc + p.info().size());
        let cur_txns = txns_with_proof_size + opt_batch_txns_size;
        let cur_unique_txns = all_unique_txns;
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

                // Fetch transactions for remaining non-proof items up to limit
                let mut inline_result = Vec::new();
                let mut inline_size = PayloadTxnsSize::zero();
                for item in remaining_non_proof {
                    if inline_size + item.info.size() > max_inline_txns_to_pull {
                        break;
                    }
                    if let Ok(mut persisted_value) =
                        self.batch_proof_queue.batch_store().get_batch_from_local(
                            item.info.digest(),
                        )
                    {
                        if let Some(txns) = persisted_value.take_payload() {
                            inline_size += item.info.size();
                            inline_result.push((item.info, txns));
                        }
                    } else {
                        warn!(
                            "Couldn't find a batch in local storage while creating inline block: {:?}",
                            item.info.digest()
                        );
                    }
                }
                (inline_result, inline_size)
            } else {
                (Vec::new(), PayloadTxnsSize::zero())
            };
        counters::NUM_INLINE_BATCHES.observe(inline_block.len() as f64);
        counters::NUM_INLINE_TXNS.observe(inline_block_size.count() as f64);

        let enable_optqs_v2 = request
            .maybe_optqs_payload_pull_params
            .as_ref()
            .is_some_and(|p| p.enable_opt_qs_v2_payload);

        let response = if enable_optqs_v2 {
            // V2: keep BatchInfoExt as-is
            Payload::OptQuorumStore(OptQuorumStorePayload::new_v2(
                inline_block.into(),
                opt_batches.into(),
                proof_block.into(),
                PayloadExecutionLimit::None,
            ))
        } else {
            trace!(
                "QS: GetBlockRequest excluded len {}, block len {}, inline len {}",
                excluded_batches.len(),
                proof_block.len(),
                inline_block.len()
            );
            // V1: downgrade to BatchInfo, filtering out V2 batches
            let inline_block_v1: Vec<_> = inline_block
                .into_iter()
                .filter(|(info, _)| !info.is_v2())
                .map(|(info, txns)| (info.info().clone(), txns))
                .collect();
            let opt_batches_v1: Vec<_> = opt_batches
                .into_iter()
                .filter(|info| !info.is_v2())
                .map(|info| info.info().clone())
                .collect();
            let proof_block_v1: Vec<_> = proof_block
                .into_iter()
                .filter_map(|proof| {
                    if !proof.is_v2() {
                        let (info, sig) = proof.unpack();
                        Some(ProofOfStore::new(info.info().clone(), sig))
                    } else {
                        None
                    }
                })
                .collect();
            Payload::OptQuorumStore(OptQuorumStorePayload::new(
                inline_block_v1.into(),
                opt_batches_v1.into(),
                proof_block_v1.into(),
                PayloadExecutionLimit::None,
            ))
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
                                self.receive_proofs(proofs.take());
                            },
                            ProofManagerCommand::ReceiveBatches(batches) => {
                                counters::QUORUM_STORE_MSG_COUNT.with_label_values(&["ProofManager::receive_batches"]).inc();
                                self.receive_batches(batches);
                            }
                            ProofManagerCommand::CommitNotification(block_timestamp, batches) => {
                                counters::QUORUM_STORE_MSG_COUNT.with_label_values(&["ProofManager::commit_notification"]).inc();
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
