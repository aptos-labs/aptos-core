// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    quorum_store::{
        batch_aggregator::{AggregationMode::IgnoreMissedFragment, BatchAggregator},
        batch_reader::BatchReaderCommand,
        batch_store::{BatchStoreCommand, PersistRequest},
        proof_builder::ProofBuilderCommand,
        types::Fragment,
    },
    round_manager::VerifiedEvent,
};
use aptos_types::PeerId;
use channel::aptos_channel;
use futures::StreamExt;
use std::collections::HashMap;
use std::sync::mpsc::SyncSender;
use tokio::sync::mpsc::Sender;

pub(crate) struct NetworkListener {
    epoch: u64,
    //not sure if needed
    network_msg_rx: aptos_channel::Receiver<PeerId, VerifiedEvent>,
    //not sure if needed, depends who verifies.
    batch_aggregators: HashMap<PeerId, BatchAggregator>,
    batch_store_tx: Sender<BatchStoreCommand>,
    batch_reader_tx: SyncSender<BatchReaderCommand>,
    proof_builder_tx: Sender<ProofBuilderCommand>,
    max_batch_size: usize,
}

impl NetworkListener {
    pub(crate) fn new(
        epoch: u64,
        network_msg_rx: aptos_channel::Receiver<PeerId, VerifiedEvent>,
        batch_store_tx: Sender<BatchStoreCommand>,
        batch_reader_tx: SyncSender<BatchReaderCommand>,
        proof_builder_tx: Sender<ProofBuilderCommand>,
        max_batch_size: usize,
    ) -> Self {
        Self {
            epoch,
            network_msg_rx,
            batch_aggregators: HashMap::new(),
            batch_store_tx,
            batch_reader_tx,
            proof_builder_tx,
            max_batch_size,
        }
    }

    async fn handle_fragment(&mut self, fragment: Fragment) {
        let source = fragment.source();
        let entry = self
            .batch_aggregators
            .entry(source)
            .or_insert(BatchAggregator::new(self.max_batch_size));
        if let Some(expiration) = fragment.fragment_info.maybe_expiration() {
            //end batch message
            if expiration.epoch() == self.epoch {
                if let Some((num_bytes, payload, digest)) = entry.end_batch(
                    fragment.batch_id(),
                    fragment.fragment_id(),
                    fragment.take_transactions(),
                    IgnoreMissedFragment,
                ) {
                    let persist_cmd = BatchStoreCommand::Persist(
                        PersistRequest::new(source, payload, digest, num_bytes, expiration),
                        None,
                    );
                    self.batch_store_tx
                        .send(persist_cmd)
                        .await
                        .expect("BatchStore receiver not available");
                }
            } // Malformed request with an inconsistent expiry epoch.
        } else {
            entry.append_transactions(
                fragment.batch_id(),
                fragment.fragment_id(),
                fragment.take_transactions(),
                IgnoreMissedFragment,
            );
        }
    }

    pub async fn start(mut self) {
        //batch fragment -> batch_aggregator, persist it, and prapre signedDigests
        //Keep in memory caching in side the DB wrapper.
        //chack id -> self, call PoQSB.
        while let Some(msg) = self.network_msg_rx.next().await {
            match msg {
                VerifiedEvent::SignedDigest(signed_digest) => {
                    let cmd = ProofBuilderCommand::AppendSignature(*signed_digest);
                    self.proof_builder_tx
                        .send(cmd)
                        .await
                        .expect("could not push signed_digest to proof_builder");
                }

                VerifiedEvent::Fragment(fragment) => {
                    self.handle_fragment(*fragment).await;
                }

                VerifiedEvent::Batch(batch) => {
                    let cmd: BatchReaderCommand;
                    if batch.maybe_payload.is_some() {
                        cmd = BatchReaderCommand::BatchResponse(
                            batch.batch_info.digest,
                            batch.get_payload(),
                        );
                    } else {
                        cmd = BatchReaderCommand::GetBatchForPeer(
                            batch.batch_info.digest,
                            batch.source,
                        );
                    }
                    self.batch_reader_tx
                        .send(cmd)
                        .expect("could not push Batch batch_reader");
                }

                _ => {
                    unreachable!()
                }
            };
        }
    }
}
