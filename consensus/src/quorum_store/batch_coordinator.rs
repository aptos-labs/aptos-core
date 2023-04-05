// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    network::{NetworkSender, QuorumStoreSender},
    quorum_store::{
        batch_store::BatchStore,
        counters,
        types::{Batch, PersistedValue},
    },
};
use aptos_logger::prelude::*;
use aptos_types::PeerId;
use std::sync::Arc;
use tokio::sync::{mpsc::Receiver, oneshot};

#[derive(Debug)]
pub enum BatchCoordinatorCommand {
    Shutdown(oneshot::Sender<()>),
    NewBatch(Vec<Batch>),
}

pub struct BatchCoordinator {
    my_peer_id: PeerId,
    network_sender: NetworkSender,
    batch_store: Arc<BatchStore<NetworkSender>>,
    max_batch_bytes: u64,
}

impl BatchCoordinator {
    pub(crate) fn new(
        my_peer_id: PeerId,
        network_sender: NetworkSender,
        batch_store: Arc<BatchStore<NetworkSender>>,
        max_batch_bytes: u64,
    ) -> Self {
        Self {
            my_peer_id,
            network_sender,
            batch_store,
            max_batch_bytes,
        }
    }

    async fn handle_batch(&mut self, batch: Batch) -> Option<PersistedValue> {
        let source = batch.author();
        let batch_id = batch.batch_id();
        trace!(
            "QS: got batch message from {} batch_id {}",
            source,
            batch_id,
        );
        counters::RECEIVED_BATCH_COUNT.inc();
        let num_bytes = batch.num_bytes();
        if num_bytes > self.max_batch_bytes {
            error!(
                "Batch from {} exceeds size limit {}, actual size: {}",
                source, self.max_batch_bytes, num_bytes
            );
            return None;
        }
        Some(batch.into())
    }

    fn persist_and_send_digests(&self, persist_requests: Vec<PersistedValue>) {
        if persist_requests.is_empty() {
            return;
        }

        let batch_store = self.batch_store.clone();
        let network_sender = self.network_sender.clone();
        let my_peer_id = self.my_peer_id;
        tokio::spawn(async move {
            let peer_id = persist_requests[0].author();
            let signed_batch_infos = batch_store.persist(persist_requests);
            if !signed_batch_infos.is_empty() {
                if my_peer_id != peer_id {
                    counters::RECEIVED_REMOTE_BATCHES_COUNT.inc_by(signed_batch_infos.len() as u64);
                }
                network_sender
                    .send_signed_batch_info_msg(signed_batch_infos, vec![peer_id])
                    .await;
            }
        });
    }

    pub(crate) async fn start(mut self, mut command_rx: Receiver<BatchCoordinatorCommand>) {
        while let Some(command) = command_rx.recv().await {
            match command {
                BatchCoordinatorCommand::Shutdown(ack_tx) => {
                    ack_tx
                        .send(())
                        .expect("Failed to send shutdown ack to QuorumStoreCoordinator");
                    break;
                },
                BatchCoordinatorCommand::NewBatch(batches) => {
                    let mut persist_requests = vec![];
                    for batch in batches.into_iter() {
                        if let Some(persist_request) = self.handle_batch(batch).await {
                            persist_requests.push(persist_request);
                        }
                    }
                    self.persist_and_send_digests(persist_requests);
                },
            }
        }
    }
}
