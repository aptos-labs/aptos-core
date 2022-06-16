// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;
// use futures::channel::{mpsc, mpsc::Sender, oneshot};
use crate::quorum_store::batch_reader::BatchReader;
use aptos_types::transaction::SignedTransaction;
use arc_swap::ArcSwapOption;
use consensus_types::common::Payload;
use consensus_types::proof_of_store::LogicalTime;
use executor_types::Error;
use tokio::sync::oneshot;

/// Notification of execution committed logical time for QuorumStore to clean.
#[async_trait::async_trait]
pub trait DataManager: Send + Sync {
    /// Notification of committed logical time
    async fn notify_commit(&self, logical_time: LogicalTime);

    fn new_epoch(&self, data_reader: Arc<BatchReader>);

    async fn get_data(&self, payload: Payload) -> Result<Vec<SignedTransaction>, Error>;
}

/// Execution -> QuorumStore notification of commits.
pub struct QuorumStoreDataManager {
    data_reader: ArcSwapOption<BatchReader>, // TODO: consider arc_swap
}

impl QuorumStoreDataManager {
    /// new
    pub fn new() -> Self {
        Self {
            data_reader: ArcSwapOption::from(None),
        }
    }
}

#[async_trait::async_trait]
impl DataManager for QuorumStoreDataManager {
    async fn notify_commit(&self, logical_time: LogicalTime) {
        self.data_reader
            .load()
            .as_ref()
            .unwrap() //TODO: can this be None? Need to make sure we call new_epoch() first.
            .update_certified_round(logical_time)
            .await;
    }

    // TODO: handle the case that the data was garbage collected and return error
    async fn get_data(&self, payload: Payload) -> Result<Vec<SignedTransaction>, Error> {
        match payload {
            Payload::DirectMempool(txns) => Ok(txns),
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

    fn new_epoch(&self, data_reader: Arc<BatchReader>) {
        self.data_reader.swap(Some(data_reader));
    }
}
