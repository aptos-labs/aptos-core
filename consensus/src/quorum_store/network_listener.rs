// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    quorum_store::{
        batch_aggregator::BatchAggregator,
        batch_reader::BatchReaderCommand,
        batch_store::{BatchStoreCommand, PersistRequest},
        proof_builder::ProofBuilderCommand,
        types::Fragment,
    },
    round_manager::VerifiedEvent,
};
use aptos_logger::debug;
use aptos_types::PeerId;
use channel::aptos_channel;
use futures::StreamExt;
use std::collections::HashMap;
use tokio::sync::mpsc::Sender;

pub(crate) struct NetworkListener {
    // TODO: reconsider which fields are needed.
    epoch: u64,
    network_msg_rx: aptos_channel::Receiver<PeerId, VerifiedEvent>,
    batch_aggregators: HashMap<PeerId, BatchAggregator>,
    batch_store_tx: Sender<BatchStoreCommand>,
    batch_reader_tx: Sender<BatchReaderCommand>,
    proof_builder_tx: Sender<ProofBuilderCommand>,
    max_batch_bytes: usize,
}

impl NetworkListener {
    pub(crate) fn new(
        epoch: u64,
        network_msg_rx: aptos_channel::Receiver<PeerId, VerifiedEvent>,
        batch_store_tx: Sender<BatchStoreCommand>,
        batch_reader_tx: Sender<BatchReaderCommand>,
        proof_builder_tx: Sender<ProofBuilderCommand>,
        max_batch_bytes: usize,
    ) -> Self {
        Self {
            epoch,
            network_msg_rx,
            batch_aggregators: HashMap::new(),
            batch_store_tx,
            batch_reader_tx,
            proof_builder_tx,
            max_batch_bytes,
        }
    }

    async fn handle_fragment(&mut self, fragment: Fragment) {
        let source = fragment.source();
        let entry = self
            .batch_aggregators
            .entry(source)
            .or_insert(BatchAggregator::new(self.max_batch_bytes));
        if let Some(expiration) = fragment.fragment_info.maybe_expiration() {
            // end batch message
            debug!(
                "QS: got end batch message from {:?} batch_id {}, fragment_id {}",
                source,
                fragment.fragment_info.batch_id(),
                fragment.fragment_info.fragment_id(),
            );
            if expiration.epoch() == self.epoch {
                match entry.end_batch(
                    fragment.batch_id(),
                    fragment.fragment_id(),
                    fragment.take_transactions(),
                ) {
                    Ok((num_bytes, payload, digest)) => {
                        let persist_cmd = BatchStoreCommand::Persist(PersistRequest::new(
                            source, payload, digest, num_bytes, expiration,
                        ));
                        self.batch_store_tx
                            .send(persist_cmd)
                            .await
                            .expect("BatchStore receiver not available");
                    }
                    Err(e) => {
                        debug!("Could not append batch from {:?}, error {:?}", source, e);
                    }
                }
            } // Malformed request with an inconsistent expiry epoch.
        } else {
            // debug!(
            //     "QS: got append_batch message from {:?} batch_id {}, fragment_id {}",
            //     source,
            //     fragment.fragment_info.batch_id(),
            //     fragment.fragment_info.fragment_id()
            // );
            if let Err(e) = entry.append_transactions(
                fragment.batch_id(),
                fragment.fragment_id(),
                fragment.take_transactions(),
            ) {
                debug!("Could not append batch from {:?}, error {:?}", source, e);
            }
        }
    }

    pub async fn start(mut self) {
        debug!("QS: starting networking");
        //batch fragment -> batch_aggregator, persist it, and prapre signedDigests
        //Keep in memory caching in side the DB wrapper.
        //chack id -> self, call PoQSB.
        while let Some(msg) = self.network_msg_rx.next().await {
            match msg {
                VerifiedEvent::SignedDigest(signed_digest) => {
                    debug!("QS: got SignedDigest from network");
                    let cmd = ProofBuilderCommand::AppendSignature(*signed_digest);
                    self.proof_builder_tx
                        .send(cmd)
                        .await
                        .expect("Could not send signed_digest to proof_builder");
                }

                VerifiedEvent::Fragment(fragment) => {
                    self.handle_fragment(*fragment).await;
                }

                VerifiedEvent::Batch(batch) => {
                    let cmd: BatchReaderCommand;
                    if batch.maybe_payload.is_some() {
                        debug!(
                            "QS: batch response from {:?} digest {}",
                            batch.source, batch.batch_info.digest
                        );
                        cmd = BatchReaderCommand::BatchResponse(
                            batch.batch_info.digest,
                            batch.get_payload(),
                        );
                    } else {
                        debug!(
                            "QS: batch request from {:?} digest {}",
                            batch.source, batch.batch_info.digest
                        );
                        cmd = BatchReaderCommand::GetBatchForPeer(
                            batch.batch_info.digest,
                            batch.source,
                        );
                    }

                    self.batch_reader_tx
                        .send(cmd)
                        .await
                        .expect("could not push Batch batch_reader");
                }

                _ => {
                    unreachable!()
                }
            };
        }
    }
}
