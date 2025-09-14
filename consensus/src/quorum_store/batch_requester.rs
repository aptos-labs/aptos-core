// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    monitor,
    network::QuorumStoreSender,
    quorum_store::{
        counters,
        types::{BatchRequest, BatchResponse, PersistedValue},
    },
};
use aptos_crypto::HashValue;
use aptos_executor_types::*;
use aptos_infallible::Mutex;
use aptos_logger::prelude::*;
use aptos_types::{transaction::SignedTransaction, validator_verifier::ValidatorVerifier, PeerId};
use futures::{stream::FuturesUnordered, StreamExt};
use rand::Rng;
use std::{collections::BTreeSet, sync::Arc, time::Duration};
use tokio::{sync::oneshot, time};

struct BatchRequesterState {
    signers: Arc<Mutex<BTreeSet<PeerId>>>,
    next_index: usize,
    num_retries: usize,
    retry_limit: usize,
}

impl BatchRequesterState {
    fn new(signers: Arc<Mutex<BTreeSet<PeerId>>>, retry_limit: usize) -> Self {
        Self {
            signers,
            next_index: 0,
            num_retries: 0,
            retry_limit,
        }
    }

    fn next_request_peers(&mut self, num_peers: usize) -> Option<Vec<PeerId>> {
        let signers = self.signers.lock();
        if self.num_retries == 0 {
            let mut rng = rand::thread_rng();
            // make sure nodes request from the different set of nodes
            self.next_index = rng.r#gen::<usize>() % signers.len();
            counters::SENT_BATCH_REQUEST_COUNT.inc_by(num_peers as u64);
        } else {
            counters::SENT_BATCH_REQUEST_RETRY_COUNT.inc_by(num_peers as u64);
        }
        if self.num_retries < self.retry_limit {
            self.num_retries += 1;
            let ret = signers
                .iter()
                .cycle()
                .skip(self.next_index)
                .take(num_peers)
                .cloned()
                .collect();
            self.next_index = (self.next_index + num_peers) % signers.len();
            Some(ret)
        } else {
            None
        }
    }
}

pub(crate) struct BatchRequester<T> {
    epoch: u64,
    my_peer_id: PeerId,
    request_num_peers: usize,
    retry_limit: usize,
    retry_interval_ms: usize,
    rpc_timeout_ms: usize,
    network_sender: T,
    validator_verifier: Arc<ValidatorVerifier>,
}

impl<T: QuorumStoreSender + Sync + 'static> BatchRequester<T> {
    pub(crate) fn new(
        epoch: u64,
        my_peer_id: PeerId,
        request_num_peers: usize,
        retry_limit: usize,
        retry_interval_ms: usize,
        rpc_timeout_ms: usize,
        network_sender: T,
        validator_verifier: Arc<ValidatorVerifier>,
    ) -> Self {
        Self {
            epoch,
            my_peer_id,
            request_num_peers,
            retry_limit,
            retry_interval_ms,
            rpc_timeout_ms,
            network_sender,
            validator_verifier,
        }
    }

    pub(crate) async fn request_batch(
        &self,
        digest: HashValue,
        expiration: u64,
        responders: Arc<Mutex<BTreeSet<PeerId>>>,
        mut subscriber_rx: oneshot::Receiver<PersistedValue>,
    ) -> ExecutorResult<Vec<SignedTransaction>> {
        let validator_verifier = self.validator_verifier.clone();
        let mut request_state = BatchRequesterState::new(responders, self.retry_limit);
        let network_sender = self.network_sender.clone();
        let request_num_peers = self.request_num_peers;
        let my_peer_id = self.my_peer_id;
        let epoch = self.epoch;
        let retry_interval = Duration::from_millis(self.retry_interval_ms as u64);
        let rpc_timeout = Duration::from_millis(self.rpc_timeout_ms as u64);

        monitor!("batch_request", {
            let mut interval = time::interval(retry_interval);
            let mut futures = FuturesUnordered::new();
            let request = BatchRequest::new(my_peer_id, epoch, digest);
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // send batch request to a set of peers of size request_num_peers
                        if let Some(request_peers) = request_state.next_request_peers(request_num_peers) {
                            for peer in request_peers {
                                futures.push(network_sender.request_batch(request.clone(), peer, rpc_timeout));
                            }
                        } else if futures.is_empty() {
                            // end the loop when the futures are drained
                            break;
                        }
                    },
                    Some(response) = futures.next() => {
                        match response {
                            Ok(BatchResponse::Batch(batch)) => {
                                counters::RECEIVED_BATCH_RESPONSE_COUNT.inc();
                                let payload = batch.into_transactions();
                                return Ok(payload);
                            }
                            // Short-circuit if the chain has moved beyond expiration
                            Ok(BatchResponse::NotFound(ledger_info)) => {
                                counters::RECEIVED_BATCH_NOT_FOUND_COUNT.inc();
                                if ledger_info.commit_info().epoch() == epoch
                                    && ledger_info.commit_info().timestamp_usecs() > expiration
                                    && ledger_info.verify_signatures(&validator_verifier).is_ok()
                                {
                                    counters::RECEIVED_BATCH_EXPIRED_COUNT.inc();
                                    debug!("QS: batch request expired, digest:{}", digest);
                                    return Err(ExecutorError::CouldNotGetData);
                                }
                            }
                            Err(e) => {
                                counters::RECEIVED_BATCH_RESPONSE_ERROR_COUNT.inc();
                                debug!("QS: batch request error, digest:{}, error:{:?}", digest, e);
                            }
                        }
                    },
                    result = &mut subscriber_rx => {
                        match result {
                            Ok(persisted_value) => {
                                counters::RECEIVED_BATCH_FROM_SUBSCRIPTION_COUNT.inc();
                                let (_, maybe_payload) = persisted_value.unpack();
                                return Ok(maybe_payload.expect("persisted value must exist"));
                            }
                            Err(err) => {
                                debug!("channel closed: {}", err);
                            }
                        };
                    },
                }
            }
            counters::RECEIVED_BATCH_REQUEST_TIMEOUT_COUNT.inc();
            debug!("QS: batch request timed out, digest:{}", digest);
            Err(ExecutorError::CouldNotGetData)
        })
    }
}
