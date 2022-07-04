// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::quorum_store::batch_reader::BatchReader;
use aptos_crypto::HashValue;
use aptos_logger::debug;
use aptos_types::transaction::SignedTransaction;
use arc_swap::ArcSwapOption;
use consensus_types::{
    common::Payload, proof_of_store::LogicalTime, request_response::WrapperCommand,
};
use executor_types::Error;
use futures::channel::mpsc::Sender;
use std::sync::Arc;

/// Notification of execution committed logical time for QuorumStore to clean.
#[async_trait::async_trait]
pub trait DataManager: Send + Sync {
    /// Notification of committed logical time
    async fn notify_commit(&self, logical_time: LogicalTime, payloads: Vec<Payload>);

    fn new_epoch(
        &self,
        data_reader: Arc<BatchReader>,
        quorum_store_wrapper_tx: Sender<WrapperCommand>,
    );

    async fn get_data(&self, payload: Payload) -> Result<Vec<SignedTransaction>, Error>;
}

/// Execution -> QuorumStore notification of commits.
pub struct QuorumStoreDataManager {
    data_reader: ArcSwapOption<BatchReader>,
    quorum_store_wrapper_tx: ArcSwapOption<Sender<WrapperCommand>>,
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
                Payload::Empty => Vec::new(),
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
            .try_send(WrapperCommand::CleanRequest(logical_time, digests))
            .expect("could not send to wrapper");
    }

    // TODO: handle the case that the data was garbage collected and return error
    async fn get_data(&self, payload: Payload) -> Result<Vec<SignedTransaction>, Error> {
        match payload {
            Payload::Empty => {
                debug!("QSE: empty Payload");
                Ok(Vec::new())
            }
            Payload::DirectMempool(_) => {
                unreachable!("Quorum store should be used.")
            }
            Payload::InQuorumStore(poss) => {
                let mut receivers = Vec::new();
                for pos in poss {
                    debug!("QSE: requesting pos {:?}, digest {}", pos, pos.digest());
                    receivers.push(
                        self.data_reader
                            .load()
                            .as_ref()
                            .unwrap() //TODO: can this be None? Need to make sure we call new_epoch() first.
                            .get_batch(pos)
                            .await,
                    );
                }
                let mut ret = Vec::new();
                for rx in receivers {
                    debug!("QSE: waiting on rx");

                    match rx
                        .await
                        .expect("Oneshot channel to get a batch was dropped")
                    {
                        Ok(data) => {
                            debug!("QS: data {:?}", data);
                            ret.push(data)
                        }
                        Err(e) => {
                            // TODO: error is the right type now, propagate?.
                            debug!("QS: could not get data {:?}", e);
                            return Err(Error::CouldNotGetData);
                        }
                    }

                    debug!("QSE: done waiting");
                }
                Ok(ret.into_iter().flatten().collect())
            }
        }
    }

    fn new_epoch(
        &self,
        data_reader: Arc<BatchReader>,
        quorum_store_wrapper_tx: Sender<WrapperCommand>,
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

    fn new_epoch(&self, _: Arc<BatchReader>, _: Sender<WrapperCommand>) {}

    async fn get_data(&self, payload: Payload) -> Result<Vec<SignedTransaction>, Error> {
        match payload {
            Payload::Empty => Ok(Vec::new()),
            Payload::DirectMempool(txns) => Ok(txns),
            Payload::InQuorumStore(_) => {
                unreachable!("Quorum store should not be used.")
            }
        }
    }
}
