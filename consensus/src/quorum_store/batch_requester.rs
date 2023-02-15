// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    network::QuorumStoreSender,
    quorum_store::{counters, types::BatchRequest, utils::DigestTimeouts},
};
use aptos_crypto::{hash::DefaultHasher, HashValue};
use aptos_executor_types::*;
use aptos_logger::debug;
use aptos_types::{transaction::SignedTransaction, PeerId};
use bcs::to_bytes;
use std::collections::HashMap;
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
            debug!(
                "QS: batch to oneshot, digest {}, tx {:?}",
                digest, self.ret_tx
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

pub(crate) struct BatchRequester<T: QuorumStoreSender> {
    epoch: u64,
    my_peer_id: PeerId,
    request_num_peers: usize,
    request_timeout_ms: usize,
    digest_to_state: HashMap<HashValue, BatchRequesterState>,
    timeouts: DigestTimeouts,
    network_sender: T,
}

impl<T: QuorumStoreSender> BatchRequester<T> {
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
            digest_to_state: HashMap::new(),
            timeouts: DigestTimeouts::new(),
            network_sender,
        }
    }

    async fn send_requests(&self, digest: HashValue, request_peers: Vec<PeerId>) {
        // Quorum Store measurements
        counters::SENT_BATCH_REQUEST_COUNT.inc();
        let request = BatchRequest::new(self.my_peer_id, self.epoch, digest);
        self.network_sender
            .send_batch_request(request, request_peers)
            .await;
    }

    pub(crate) async fn add_request(
        &mut self,
        digest: HashValue,
        signers: Vec<PeerId>,
        ret_tx: oneshot::Sender<Result<Vec<SignedTransaction>, aptos_executor_types::Error>>,
    ) {
        let mut request_state = BatchRequesterState::new(signers, ret_tx);
        let request_peers = request_state
            .next_request_peers(self.request_num_peers)
            .unwrap(); // note: this is the first try

        debug!("QS: requesting from {:?}", request_peers);

        self.digest_to_state.insert(digest, request_state);
        self.send_requests(digest, request_peers).await;
        self.timeouts.add_digest(digest, self.request_timeout_ms);
    }

    pub(crate) async fn handle_timeouts(&mut self) {
        for digest in self.timeouts.expire() {
            debug!("QS: timed out batch request, digest = {}", digest);
            if let Some(state) = self.digest_to_state.get_mut(&digest) {
                if let Some(request_peers) = state.next_request_peers(self.request_num_peers) {
                    // Quorum Store measurements
                    counters::SENT_BATCH_REQUEST_RETRY_COUNT.inc();
                    self.send_requests(digest, request_peers).await;
                    self.timeouts.add_digest(digest, self.request_timeout_ms);
                } else {
                    let state = self.digest_to_state.remove(&digest).unwrap();
                    state.serve_request(digest, None);
                }
            }
        }
    }

    pub(crate) fn serve_request(&mut self, digest: HashValue, payload: Vec<SignedTransaction>) {
        if self.digest_to_state.contains_key(&digest) {
            let mut hasher = DefaultHasher::new(b"QuorumStoreBatch");
            let serialized_payload: Vec<u8> = payload
                .iter()
                .flat_map(|txn| to_bytes(txn).unwrap())
                .collect();
            hasher.update(&serialized_payload);
            if hasher.finish() == digest {
                debug!("QS: serving batch digest = {}", digest);
                let state = self.digest_to_state.remove(&digest).unwrap();
                state.serve_request(digest, Some(payload));
            } else {
                debug!("Payload does not fit digest")
            }
        }
    }
}
