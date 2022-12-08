// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::quorum_store::batch_reader::BatchReader;
use aptos_consensus_types::{
    block::Block,
    common::{DataStatus, Payload},
    proof_of_store::{LogicalTime, ProofOfStore},
    request_response::PayloadRequest,
};
use aptos_crypto::HashValue;
use aptos_infallible::Mutex;
use aptos_logger::{debug, warn};
use aptos_types::transaction::SignedTransaction;
use executor_types::Error::DataNotFound;
use executor_types::*;
use futures::channel::mpsc::Sender;
use futures::SinkExt;
use tokio::sync::oneshot;

/// Responsible to extract the transactions out of the payload and notify QuorumStore about commits.
/// If QuorumStore is enabled, has to ask BatchReader for the transaction behind the proofs of availability in the payload.
pub enum PayloadManager {
    DirectMempool,
    InQuorumStore(BatchReader, Mutex<Sender<PayloadRequest>>),
}

impl PayloadManager {
    async fn request_transactions(
        proofs: Vec<ProofOfStore>,
        logical_time: LogicalTime,
        batch_reader: &BatchReader,
    ) -> Vec<(
        HashValue,
        oneshot::Receiver<Result<Vec<SignedTransaction>, executor_types::Error>>,
    )> {
        let mut receivers = Vec::new();
        for pos in proofs {
            debug!(
                "QSE: requesting pos {:?}, digest {}, time = {:?}",
                pos,
                pos.digest(),
                logical_time
            );
            if logical_time <= pos.expiration() {
                receivers.push((*pos.digest(), batch_reader.get_batch(pos).await));
            } else {
                debug!("QS: skipped expired pos");
            }
        }
        receivers
    }

    ///Pass commit information to BatchReader and QuorumStore wrapper for their internal cleanups.
    pub async fn notify_commit(&self, logical_time: LogicalTime, payloads: Vec<Payload>) {
        match self {
            PayloadManager::DirectMempool => {}
            PayloadManager::InQuorumStore(batch_reader, quorum_store_wrapper_tx) => {
                batch_reader.update_certified_round(logical_time).await;

                let digests: Vec<HashValue> = payloads
                    .into_iter()
                    .flat_map(|payload| match payload {
                        Payload::DirectMempool(_) => {
                            unreachable!("InQuorumStore should be used");
                        }
                        Payload::InQuorumStore(proof_with_status) => proof_with_status.proofs,
                    })
                    .map(|proof| *proof.digest())
                    .collect();

                let _ = quorum_store_wrapper_tx
                    .lock()
                    .send(PayloadRequest::CleanRequest(logical_time, digests));
            }
        }
    }

    /// Called from consensus to pre-fetch the transaction behind the batches in the block.
    pub async fn prefetch_payload_data(&self, block: &Block) {
        let payload = match block.payload() {
            Some(p) => p,
            None => return,
        };
        match self {
            PayloadManager::DirectMempool => {}
            PayloadManager::InQuorumStore(batch_reader, _) => match payload {
                Payload::InQuorumStore(proof_with_status) => {
                    if proof_with_status.status.lock().is_none() {
                        let receivers = PayloadManager::request_transactions(
                            proof_with_status.proofs.clone(),
                            LogicalTime::new(block.epoch(), block.round()),
                            batch_reader,
                        )
                        .await;
                        proof_with_status
                            .status
                            .lock()
                            .replace(DataStatus::Requested(receivers));
                    }
                }
                Payload::DirectMempool(_) => {
                    unreachable!()
                }
            },
        }
    }

    /// Extract transaction from a given block
    /// Assumes it is never called for the same block concurrently. Otherwise status can be None.
    pub async fn get_transactions(&self, block: &Block) -> Result<Vec<SignedTransaction>, Error> {
        let payload = match block.payload() {
            Some(p) => p,
            None => return Ok(Vec::new()),
        };

        match (self, payload) {
            (PayloadManager::DirectMempool, Payload::DirectMempool(txns)) => Ok(txns.clone()),
            (
                PayloadManager::InQuorumStore(batch_reader, _),
                Payload::InQuorumStore(proof_with_data),
            ) => {
                let status = proof_with_data.status.lock().take();
                match status.expect("Should have been updated before.") {
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
                                Err(e) => {
                                    // We probably advanced epoch already.
                                    warn!("Oneshot channel to get a batch was dropped with error {:?}", e);
                                    let new_receivers = PayloadManager::request_transactions(
                                        proof_with_data.proofs.clone(),
                                        LogicalTime::new(block.epoch(), block.round()),
                                        batch_reader,
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
                                    vec_ret.push(data);
                                }
                                Ok(Err(e)) => {
                                    let new_receivers = PayloadManager::request_transactions(
                                        proof_with_data.proofs.clone(),
                                        LogicalTime::new(block.epoch(), block.round()),
                                        batch_reader,
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
                        let ret: Vec<SignedTransaction> = vec_ret.into_iter().flatten().collect();
                        // execution asks for the data twice, so data is cached here for the second time.
                        proof_with_data
                            .status
                            .lock()
                            .replace(DataStatus::Cached(ret.clone()));
                        Ok(ret)
                    }
                }
            }
            (_, _) => unreachable!(
                "Wrong payload {} epoch {}, round {}, id {}",
                payload,
                block.block_data().epoch(),
                block.block_data().round(),
                block.id()
            ),
        }
    }
}
