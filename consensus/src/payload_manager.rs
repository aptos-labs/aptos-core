// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::quorum_store::batch_reader::BatchReader;
use aptos_crypto::HashValue;
use aptos_logger::{debug, warn};
use aptos_types::transaction::SignedTransaction;
use arc_swap::ArcSwapOption;
use consensus_types::common::DataStatus;
use consensus_types::{
    block::Block,
    common::Payload,
    proof_of_store::{LogicalTime, ProofOfStore},
    request_response::PayloadRequest,
};
use executor_types::Error::{DataNotFound};
use executor_types::*;
use futures::channel::mpsc::Sender;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use futures::SinkExt;
use tokio::sync::oneshot;

/// Responsible to extract the transactions out of the payload and notify QuorumStore about commits.
/// If QuorumStore is enabled, has to ask BatchReader for the transaction behind the proofs of availability in the payload.
pub struct PayloadManager {
    quorum_store_enabled: AtomicBool,
    data_reader: ArcSwapOption<BatchReader>,
    quorum_store_wrapper_tx: ArcSwapOption<Sender<PayloadRequest>>,
}

impl PayloadManager {
    pub fn new() -> Self {
        Self {
            quorum_store_enabled: AtomicBool::new(false),
            data_reader: ArcSwapOption::from(None),
            quorum_store_wrapper_tx: ArcSwapOption::from(None),
        }
    }

    #[allow(dead_code)]
    pub fn enable_quorum_store(&self) {
        self.quorum_store_enabled.store(true, Ordering::Relaxed)
    }

    async fn request_transactions(
        &self,
        proofs: Vec<ProofOfStore>,
        logical_time: LogicalTime,
    ) -> Vec<(HashValue, oneshot::Receiver<Result<Vec<SignedTransaction>, executor_types::Error>>)> {
        let mut receivers = Vec::new();
        for pos in proofs {
            debug!(
                "QSE: requesting pos {:?}, digest {}, time = {:?}",
                pos,
                pos.digest(),
                logical_time
            );
            if logical_time <= pos.expiration() {
                receivers.push(
                    (
                        pos.digest().clone(),
                        self.data_reader
                            .load()
                            .as_ref()
                            .unwrap() //TODO: can this be None? Need to make sure we call new_epoch() first.
                            .get_batch(pos)
                            .await
                    )
                );
            } else {
                debug!("QS: skipped expired pos");
            }
        }
        receivers
    }

    ///Pass commit information to BatchReader and QuorumStore wrapper for their internal cleanups.
    pub async fn notify_commit(&self, logical_time: LogicalTime, payloads: Vec<Payload>) {
        if self.quorum_store_enabled.load(Ordering::Relaxed) {
            self.data_reader
                .load()
                .as_ref()
                .unwrap()
                .update_certified_round(logical_time)
                .await;

            let digests: Vec<HashValue> = payloads
                .into_iter()
                .flat_map(|payload| match payload {
                    Payload::DirectMempool(_) => {
                        warn!("InQuorumStore should be used");
                        Vec::new()
                    }
                    Payload::InQuorumStore(proof_with_status) => proof_with_status.proofs,
                    Payload::Empty => Vec::new(),
                })
                .map(|proof| *proof.digest())
                .collect();

            let _ = self
                .quorum_store_wrapper_tx
                .load()
                .as_ref()
                .unwrap()
                .as_ref()
                .clone()
                .send(PayloadRequest::CleanRequest(logical_time, digests));
        }
    }

    /// Called from consensus to pre-fetch the transaction behind the batches in the block.
    pub async fn prefetch_payload_data(&self, block: &Block) {
        if self.quorum_store_enabled.load(Ordering::Relaxed) && block.payload().is_some() {
            match block.payload().unwrap() {
                Payload::InQuorumStore(proof_with_status) => {
                    if proof_with_status.status.lock().is_none() {
                        let receivers = self
                            .request_transactions(
                                proof_with_status.proofs.clone(),
                                LogicalTime::new(block.epoch(), block.round()),
                            )
                            .await;
                        proof_with_status
                            .status
                            .lock()
                            .replace(DataStatus::Requested(receivers));
                    }
                }
                Payload::Empty => {}
                Payload::DirectMempool(_) => {
                    unreachable!()
                }
            }
        }
    }

    /// Extract transaction from a given block
    /// Assumes it is never called for the same block concurrently. Otherwise status can be None.
    pub async fn get_transactions(&self, block: &Block) -> Result<Vec<SignedTransaction>, Error> {
        if block.payload().map_or(true, |p| p.is_empty()) {
            return Ok(Vec::new());
        }

        if self.quorum_store_enabled.load(Ordering::Relaxed) {
            match block.payload().unwrap() {
                Payload::InQuorumStore(proof_with_data) => {
                    let status = proof_with_data.status.lock().take();
                    match status.expect("Should have been updated before") {
                        DataStatus::Cached(data) => {
                            proof_with_data
                                .status
                                .lock()
                                .replace(DataStatus::Cached(data.clone()));
                            Ok(data)
                        }
                        DataStatus::Requested(receivers) => {
                            let mut vec_ret = Vec::new();
                            debug!("QSE: waiting for data on {} receivers", receivers.len());
                            for (digest, rx) in receivers {
                                match rx.await {
                                    Err(_) => {
                                        // We probably advanced epoch already.
                                        warn!("Oneshot channel to get a batch was dropped");
                                        let new_receivers = self
                                            .request_transactions(
                                                proof_with_data.proofs.clone(),
                                                LogicalTime::new(block.epoch(), block.round()),
                                            )
                                            .await;
                                        // Could not get all data so requested again
                                        proof_with_data
                                            .status
                                            .lock()
                                            .replace(DataStatus::Requested(new_receivers));
                                        return Err(DataNotFound(digest));
                                    }
                                    Ok(Ok(data)) => {
                                        debug!("QSE: got data, len {}", data.len());
                                        vec_ret.push(data);
                                    }
                                    Ok(Err(e)) => {
                                        debug!("QS: got error from receiver {:?}", e);
                                        let new_receivers = self
                                            .request_transactions(
                                                proof_with_data.proofs.clone(),
                                                LogicalTime::new(block.epoch(), block.round()),
                                            )
                                            .await;
                                        // Could not get all data so requested again
                                        proof_with_data
                                            .status
                                            .lock()
                                            .replace(DataStatus::Requested(new_receivers));
                                        return Err(e);
                                    }
                                }
                            }
                            let ret: Vec<SignedTransaction> =
                                vec_ret.into_iter().flatten().collect();
                            // execution asks for the data twice, so data is cached here for the second time.
                            proof_with_data
                                .status
                                .lock()
                                .replace(DataStatus::Cached(ret.clone()));
                            Ok(ret)
                        }
                    }
                }
                _ => {
                    // the Empty case is checked in the beginning
                    warn!("should use QuorumStore");
                    Ok(Vec::new())
                }
            }
        } else {
            match block.payload().unwrap() {
                Payload::DirectMempool(txns) => Ok(txns.clone()),
                _ => {
                    // the Empty case is checked in the beginning
                    warn!("should not use QuorumStore");
                    Ok(Vec::new())
                }
            }
        }
    }

    /// Since QuorumStore restarts every epoch, new_epoch updates the relevant communication information
    #[allow(dead_code)]
    pub fn new_epoch(
        &self,
        data_reader: Arc<BatchReader>,
        quorum_store_wrapper_tx: Sender<PayloadRequest>,
    ) {
        if self.quorum_store_enabled.load(Ordering::Relaxed) {
            self.data_reader.swap(Some(data_reader));
            self.quorum_store_wrapper_tx
                .swap(Some(Arc::from(quorum_store_wrapper_tx)));
        }
    }
}
