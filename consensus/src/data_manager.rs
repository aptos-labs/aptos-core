// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;
// use futures::channel::{mpsc, mpsc::Sender, oneshot};
use crate::quorum_store::batch_reader::BatchReader;
use aptos_crypto::HashValue;
use aptos_types::transaction::SignedTransaction;
use arc_swap::ArcSwapOption;
use consensus_types::common::Payload;
use consensus_types::proof_of_store::LogicalTime;
use consensus_types::request_response::ConsensusRequest;
use executor_types::Error;
use futures::channel::mpsc::Sender;
use tokio::sync::oneshot;

/// Notification of execution committed logical time for QuorumStore to clean.
#[async_trait::async_trait]
pub trait DataManager: Send + Sync {
    /// Notification of committed logical time
    async fn notify_commit(&self, logical_time: LogicalTime, payloads: Vec<Payload>);

    fn new_epoch(
        &self,
        data_reader: Arc<BatchReader>,
        quorum_store_wrapper_tx: Sender<ConsensusRequest>,
    );

    async fn get_data(&self, maybe_payload: Option<Payload>) -> Result<Vec<SignedTransaction>, Error>;
}

/// Execution -> QuorumStore notification of commits.
pub struct QuorumStoreDataManager {
    data_reader: ArcSwapOption<BatchReader>,
    quorum_store_wrapper_tx: ArcSwapOption<Sender<ConsensusRequest>>,
}

impl QuorumStoreDataManager {
    /// new
    pub fn new() -> Self {
        Self {
            data_reader: ArcSwapOption::from(None),
            quorum_store_wrapper_tx: ArcSwapOption::from(None),
        }
    }
}

#[async_trait::async_trait]
impl DataManager for QuorumStoreDataManager {
    async fn notify_commit(&self, logical_time: LogicalTime, payloads: Vec<Payload>) {
        self.data_reader
            .load()
            .as_ref()
            .unwrap()
            .update_certified_round(logical_time)
            .await;

        let digests: Vec<HashValue> = payloads
            .into_iter()
            .map(|payload| match payload {
                Payload::DirectMempool(_) => {
                    unreachable!()
                }
                Payload::InQuorumStore(proofs) => proofs,
            })
            .flatten()
            .map(|proof| proof.digest().clone())
            .collect();

        self.quorum_store_wrapper_tx
            .load()
            .as_ref()
            .unwrap()
            .as_ref()
            .clone()
            .try_send(ConsensusRequest::CleanRequest(logical_time, digests))
            .expect("could not send to wrapper");
    }

    // TODO: handle the case that the data was garbage collected and return error
    async fn get_data(&self, maybe_payload: Option<Payload>) -> Result<Vec<SignedTransaction>, Error> {
        match maybe_payload {
            None => { Ok(Vec::new()) }
            Some(payload) => {
                match payload {
                    Payload::DirectMempool(_) => {
                        unreachable!("Quorum store should be used.")
                    }
                    Payload::InQuorumStore(poss) => {
                        let mut receivers = Vec::new();
                        for pos in poss {
                            let (tx_data, rx_data) = oneshot::channel();
                            self.data_reader
                                .load()
                                .as_ref()
                                .unwrap() //TODO: can this be None? Need to make sure we call new_epoch() first.
                                .get_batch(pos, tx_data)
                                .await;
                            receivers.push(rx_data);
                        }
                        let mut ret = Vec::new();
                        for rx in receivers {
                            match rx.await.expect("oneshot was dropped") {
                                Ok(data) => ret.push(data),
                                Err(_) => {
                                    return Err(Error::CouldNotGetData);
                                }
                            }
                        }
                        Ok(ret.into_iter().flatten().collect())
                    }
                }
            }
        }
    }

    fn new_epoch(
        &self,
        data_reader: Arc<BatchReader>,
        quorum_store_wrapper_tx: Sender<ConsensusRequest>,
    ) {
        self.data_reader.swap(Some(data_reader));
        self.quorum_store_wrapper_tx
            .swap(Some(Arc::from(quorum_store_wrapper_tx)));
    }
}

pub struct DummyDataManager {}

impl DummyDataManager {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl DataManager for DummyDataManager {
    async fn notify_commit(&self, _: LogicalTime, _: Vec<Payload>) {}

    fn new_epoch(&self, _: Arc<BatchReader>, _: Sender<ConsensusRequest>) {}

    async fn get_data(&self, maybe_payload: Option<Payload>) -> Result<Vec<SignedTransaction>, Error> {
        match maybe_payload {
            None => { Ok(Vec::new()) }
            Some(payload) => {
                match payload {
                    Payload::DirectMempool(txns) => Ok(txns),
                    Payload::InQuorumStore(_) => {
                        unreachable!("Quorum store should not be used.")
                    }
                }
            }
        }
    }
}
