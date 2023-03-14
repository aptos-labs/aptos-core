// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    network::QuorumStoreSender,
    quorum_store::{counters, types::BatchRequest},
};
use aptos_crypto::HashValue;
use aptos_executor_types::*;
use aptos_logger::prelude::*;
use aptos_types::{transaction::SignedTransaction, PeerId};
use futures::{stream::FuturesUnordered, StreamExt};
use std::time::Duration;
use tokio::sync::oneshot;

struct BatchRequesterState {
    signers: Vec<PeerId>,
    next_index: usize,
    ret_tx: oneshot::Sender<Result<Vec<SignedTransaction>, aptos_executor_types::Error>>,
    num_retries: usize,
    max_num_retry: usize,
}

impl BatchRequesterState {
    fn new(
        signers: Vec<PeerId>,
        ret_tx: oneshot::Sender<Result<Vec<SignedTransaction>, aptos_executor_types::Error>>,
    ) -> Self {
        Self {
            signers,
            next_index: 0,
            ret_tx,
            num_retries: 0,
            max_num_retry: 5, // TODO: get it from config.
        }
    }

    fn next_request_peers(&mut self, num_peers: usize) -> Option<Vec<PeerId>> {
        if self.num_retries < self.max_num_retry {
            self.num_retries += 1;
            let ret = self
                .signers
                .iter()
                .cycle()
                .skip(self.next_index)
                .take(num_peers)
                .cloned()
                .collect();
            self.next_index = (self.next_index + num_peers) % self.signers.len();
            Some(ret)
        } else {
            None
        }
    }

    // TODO: if None, then return an error to the caller
    fn serve_request(self, digest: HashValue, maybe_payload: Option<Vec<SignedTransaction>>) {
        if let Some(payload) = maybe_payload {
            trace!(
                "QS: batch to oneshot, digest {}, tx {:?}",
                digest,
                self.ret_tx
            );
            if self.ret_tx.send(Ok(payload)).is_err() {
                debug!(
                    "Receiver of requested batch not available for digest {}",
                    digest
                )
            };
        } else {
            counters::RECEIVED_BATCH_REQUEST_TIMEOUT_COUNT.inc();
            debug!("QS: batch timed out, digest {}", digest);
            if self.ret_tx.send(Err(Error::CouldNotGetData)).is_err() {
                debug!(
                    "Receiver of requested batch not available for timed out digest {}",
                    digest
                );
            }
        }
    }
}

pub(crate) struct BatchRequester<T> {
    epoch: u64,
    my_peer_id: PeerId,
    request_num_peers: usize,
    request_timeout_ms: usize,
    network_sender: T,
}

impl<T: QuorumStoreSender + 'static> BatchRequester<T> {
    pub(crate) fn new(
        epoch: u64,
        my_peer_id: PeerId,
        request_num_peers: usize,
        request_timeout_ms: usize,
        network_sender: T,
    ) -> Self {
        Self {
            epoch,
            my_peer_id,
            request_num_peers,
            request_timeout_ms,
            network_sender,
        }
    }

    pub(crate) fn request_batch(
        &self,
        digest: HashValue,
        signers: Vec<PeerId>,
        ret_tx: oneshot::Sender<Result<Vec<SignedTransaction>, Error>>,
    ) {
        let mut request_state = BatchRequesterState::new(signers, ret_tx);
        let network_sender = self.network_sender.clone();
        let request_num_peers = self.request_num_peers;
        let my_peer_id = self.my_peer_id;
        let epoch = self.epoch;
        let timeout = Duration::from_millis(self.request_timeout_ms as u64);

        tokio::spawn(async move {
            while let Some(request_peers) = request_state.next_request_peers(request_num_peers) {
                let mut futures = FuturesUnordered::new();
                trace!("QS: requesting from {:?}", request_peers);
                let request = BatchRequest::new(my_peer_id, epoch, digest);
                for peer in request_peers {
                    counters::SENT_BATCH_REQUEST_COUNT.inc();
                    futures.push(network_sender.request_batch(request.clone(), peer, timeout));
                }
                while let Some(response) = futures.next().await {
                    if let Ok(batch) = response {
                        counters::RECEIVED_BATCH_COUNT.inc();
                        if batch.verify().is_ok() {
                            let digest = batch.digest();
                            let payload = batch.into_payload();
                            request_state.serve_request(digest, Some(payload));
                            return;
                        }
                    }
                }
                counters::SENT_BATCH_REQUEST_RETRY_COUNT.inc();
            }
            request_state.serve_request(digest, None);
        });
    }
}
