// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    network::{NetworkSender, QuorumStoreSender},
    quorum_store::{
        batch_aggregator::BatchAggregator,
        batch_store::{BatchStore, PersistRequest},
        counters,
        types::Fragment,
    },
};
use aptos_logger::prelude::*;
use aptos_types::PeerId;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{mpsc::Receiver, oneshot};

#[derive(Debug)]
pub enum BatchCoordinatorCommand {
    Shutdown(oneshot::Sender<()>),
    AppendFragment(Box<Fragment>),
}

pub struct BatchCoordinator {
    epoch: u64,
    my_peer_id: PeerId,
    network_sender: NetworkSender,
    batch_store: Arc<BatchStore<NetworkSender>>,
    max_batch_bytes: usize,
    batch_aggregators: HashMap<PeerId, BatchAggregator>,
}

impl BatchCoordinator {
    pub(crate) fn new(
        epoch: u64, //TODO: pass the epoch config
        my_peer_id: PeerId,
        network_sender: NetworkSender,
        batch_store: Arc<BatchStore<NetworkSender>>,
        max_batch_bytes: usize,
    ) -> Self {
        Self {
            epoch,
            my_peer_id,
            network_sender,
            batch_store,
            max_batch_bytes,
            batch_aggregators: HashMap::new(),
        }
    }

    async fn handle_fragment(&mut self, fragment: Fragment) -> Option<PersistRequest> {
        let source = fragment.source();
        let entry = self
            .batch_aggregators
            .entry(source)
            .or_insert_with(|| BatchAggregator::new(self.max_batch_bytes));
        if let Some(expiration) = fragment.maybe_expiration() {
            counters::DELIVERED_END_BATCH_COUNT.inc();
            // end batch message
            trace!(
                "QS: got end batch message from {:?} batch_id {}, fragment_id {}",
                source,
                fragment.batch_id(),
                fragment.fragment_id(),
            );
            let batch_id = fragment.batch_id();
            if expiration.epoch() == self.epoch {
                match entry.end_batch(
                    fragment.batch_id(),
                    fragment.fragment_id(),
                    fragment.into_transactions(),
                ) {
                    Ok((num_bytes, payload, digest)) => {
                        let persist_request = PersistRequest::new(
                            source, batch_id, payload, digest, num_bytes, expiration,
                        );
                        return Some(persist_request);
                    },
                    Err(e) => {
                        debug!("Could not append batch from {:?}, error {:?}", source, e);
                    },
                }
            }
            // Malformed request with an inconsistent expiry epoch.
            else {
                trace!(
                    "QS: got end batch message from different epoch {} != {}",
                    expiration.epoch(),
                    self.epoch
                );
            }
        } else if let Err(e) = entry.append_transactions(
            fragment.batch_id(),
            fragment.fragment_id(),
            fragment.into_transactions(),
        ) {
            debug!("Could not append batch from {:?}, error {:?}", source, e);
        }
        None
    }

    fn persist_and_send_digest(&self, persist_request: PersistRequest) {
        let batch_store = self.batch_store.clone();
        let network_sender = self.network_sender.clone();
        let my_peer_id = self.my_peer_id;
        tokio::spawn(async move {
            let peer_id = persist_request.value.author;
            if let Some(signed_digest) = batch_store.persist(persist_request) {
                if my_peer_id != peer_id {
                    counters::DELIVERED_BATCHES_COUNT.inc();
                }
                network_sender
                    .send_signed_digest(signed_digest, vec![peer_id])
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
                BatchCoordinatorCommand::AppendFragment(fragment) => {
                    if let Some(persist_request) = self.handle_fragment(*fragment).await {
                        self.persist_and_send_digest(persist_request);
                    }
                },
            }
        }
    }
}
