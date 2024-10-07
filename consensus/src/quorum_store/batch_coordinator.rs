// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    network::{NetworkSender, QuorumStoreSender},
    quorum_store::{
        batch_generator::BatchGeneratorCommand,
        batch_store::{BatchStore, BatchWriter},
        counters,
        proof_manager::ProofManagerCommand,
        types::{Batch, PersistedValue},
    },
};
use anyhow::ensure;
use aptos_logger::prelude::*;
use aptos_types::PeerId;
use std::sync::Arc;
use tokio::sync::{
    mpsc::{Receiver, Sender},
    oneshot,
};

#[derive(Debug)]
pub enum BatchCoordinatorCommand {
    Shutdown(oneshot::Sender<()>),
    NewBatches(PeerId, Vec<Batch>),
}

/// The `BatchCoordinator` is responsible for coordinating the receipt and persistence of batches.
pub struct BatchCoordinator {
    my_peer_id: PeerId,
    network_sender: Arc<NetworkSender>,
    sender_to_proof_manager: Arc<Sender<ProofManagerCommand>>,
    sender_to_batch_generator: Arc<Sender<BatchGeneratorCommand>>,
    batch_store: Arc<BatchStore>,
    max_batch_txns: u64,
    max_batch_bytes: u64,
    max_total_txns: u64,
    max_total_bytes: u64,
}

impl BatchCoordinator {
    pub(crate) fn new(
        my_peer_id: PeerId,
        network_sender: NetworkSender,
        sender_to_proof_manager: Sender<ProofManagerCommand>,
        sender_to_batch_generator: Sender<BatchGeneratorCommand>,
        batch_store: Arc<BatchStore>,
        max_batch_txns: u64,
        max_batch_bytes: u64,
        max_total_txns: u64,
        max_total_bytes: u64,
    ) -> Self {
        Self {
            my_peer_id,
            network_sender: Arc::new(network_sender),
            sender_to_proof_manager: Arc::new(sender_to_proof_manager),
            sender_to_batch_generator: Arc::new(sender_to_batch_generator),
            batch_store,
            max_batch_txns,
            max_batch_bytes,
            max_total_txns,
            max_total_bytes,
        }
    }

    fn persist_and_send_digests(&self, persist_requests: Vec<PersistedValue>) {
        if persist_requests.is_empty() {
            return;
        }

        let batch_store = self.batch_store.clone();
        let network_sender = self.network_sender.clone();
        let sender_to_proof_manager = self.sender_to_proof_manager.clone();
        tokio::spawn(async move {
            let peer_id = persist_requests[0].author();
            let batches = persist_requests
                .iter()
                .map(|persisted_value| {
                    (
                        persisted_value.batch_info().clone(),
                        persisted_value.summary(),
                    )
                })
                .collect();
            let signed_batch_infos = batch_store.persist(persist_requests);
            if !signed_batch_infos.is_empty() {
                network_sender
                    .send_signed_batch_info_msg(signed_batch_infos, vec![peer_id])
                    .await;
            }
            let _ = sender_to_proof_manager
                .send(ProofManagerCommand::ReceiveBatches(batches))
                .await;
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

    async fn handle_batches_msg(&mut self, author: PeerId, batches: Vec<Batch>) {
        if let Err(e) = self.ensure_max_limits(&batches) {
            error!("Batch from {}: {}", author, e);
            counters::RECEIVED_BATCH_MAX_LIMIT_FAILED.inc();
            return;
        }

        let mut persist_requests = vec![];
        for batch in batches.into_iter() {
            // TODO: maybe don't message batch generator if the persist is unsuccessful?
            if let Err(e) = self
                .sender_to_batch_generator
                .send(BatchGeneratorCommand::RemoteBatch(batch.clone()))
                .await
            {
                warn!("Failed to send batch to batch generator: {}", e);
            }
            persist_requests.push(batch.into());
        }
        counters::RECEIVED_BATCH_COUNT.inc_by(persist_requests.len() as u64);
        if author != self.my_peer_id {
            counters::RECEIVED_REMOTE_BATCH_COUNT.inc_by(persist_requests.len() as u64);
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
                BatchCoordinatorCommand::NewBatches(author, batches) => {
                    self.handle_batches_msg(author, batches).await;
                },
            }
        }
    }
}
