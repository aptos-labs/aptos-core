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
use anyhow::ensure;
use aptos_logger::prelude::*;
use aptos_types::PeerId;
use std::sync::Arc;
use tokio::sync::{mpsc::Receiver, oneshot};

#[derive(Debug)]
pub enum BatchCoordinatorCommand {
    Shutdown(oneshot::Sender<()>),
    NewBatches(Vec<Batch>),
}

pub struct BatchCoordinator {
    my_peer_id: PeerId,
    network_sender: NetworkSender,
    batch_store: Arc<BatchStore<NetworkSender>>,
    max_batch_txns: u64,
    max_batch_bytes: u64,
    max_total_txns: u64,
    max_total_bytes: u64,
}

impl BatchCoordinator {
    pub(crate) fn new(
        my_peer_id: PeerId,
        network_sender: NetworkSender,
        batch_store: Arc<BatchStore<NetworkSender>>,
        max_batch_txns: u64,
        max_batch_bytes: u64,
        max_total_txns: u64,
        max_total_bytes: u64,
    ) -> Self {
        Self {
            my_peer_id,
            network_sender,
            batch_store,
            max_batch_txns,
            max_batch_bytes,
            max_total_txns,
            max_total_bytes,
        }
    }

    async fn handle_batch(&mut self, batch: Batch) -> Option<PersistedValue> {
        let author = batch.author();
        let batch_id = batch.batch_id();
        trace!(
            "QS: got batch message from {} batch_id {}",
            author,
            batch_id,
        );
        counters::RECEIVED_BATCH_COUNT.inc();
        if batch.num_txns() > self.max_batch_txns {
            warn!(
                "Batch from {} exceeds txn limit {}, actual txns: {}",
                author,
                self.max_batch_txns,
                batch.num_txns(),
            );
            return None;
        }
        if batch.num_bytes() > self.max_batch_bytes {
            warn!(
                "Batch from {} exceeds size limit {}, actual size: {}",
                author,
                self.max_batch_bytes,
                batch.num_bytes(),
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

    fn ensure_max_limits(&self, batches: &[Batch]) -> anyhow::Result<()> {
        let mut total_txns = 0;
        let mut total_bytes = 0;
        for batch in batches.iter() {
            ensure!(
                batch.num_txns() <= self.max_batch_txns,
                "Exceeds batch txn limit {} > {}",
                batch.num_txns(),
                self.max_batch_txns,
            );
            ensure!(
                batch.num_bytes() <= self.max_batch_bytes,
                "Exceeds batch bytes limit {} > {}",
                batch.num_bytes(),
                self.max_batch_bytes,
            );

            total_txns += batch.num_txns();
            total_bytes += batch.num_bytes();
        }
        ensure!(
            total_txns <= self.max_total_txns,
            "Exceeds total txn limit {} > {}",
            total_txns,
            self.max_total_txns,
        );
        ensure!(
            total_bytes <= self.max_total_bytes,
            "Exceeds total bytes limit: {} > {}",
            total_bytes,
            self.max_total_bytes,
        );

        Ok(())
    }

    async fn handle_batches_msg(&mut self, batches: Vec<Batch>) {
        if let Err(e) = self.ensure_max_limits(&batches) {
            warn!("Batch from {}: {}", batches.first().unwrap().author(), e);
            return;
        }

        let mut persist_requests = vec![];
        for batch in batches.into_iter() {
            if let Some(persist_request) = self.handle_batch(batch).await {
                persist_requests.push(persist_request);
            }
        }
        self.persist_and_send_digests(persist_requests);
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
                BatchCoordinatorCommand::NewBatches(batches) => {
                    self.handle_batches_msg(batches).await;
                },
            }
        }
    }
}
