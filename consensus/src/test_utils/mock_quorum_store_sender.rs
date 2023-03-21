// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    network::QuorumStoreSender,
    network_interface::ConsensusMsg,
    quorum_store::types::{Batch, BatchRequest},
};
use aptos_consensus_types::{
    common::Author,
    proof_of_store::{ProofOfStore, SignedBatchInfo},
};
use std::time::Duration;
use tokio::sync::mpsc::Sender;

#[derive(Clone)]
pub struct MockQuorumStoreSender {
    tx: Sender<(ConsensusMsg, Vec<Author>)>,
}

impl MockQuorumStoreSender {
    pub fn new(tx: Sender<(ConsensusMsg, Vec<Author>)>) -> Self {
        Self { tx }
    }
}

#[async_trait::async_trait]
impl QuorumStoreSender for MockQuorumStoreSender {
    async fn send_batch_request(&self, request: BatchRequest, recipients: Vec<Author>) {
        self.tx
            .send((ConsensusMsg::BatchRequestMsg(Box::new(request)), recipients))
            .await
            .expect("could not send");
    }

    async fn request_batch(
        &self,
        _request: BatchRequest,
        _recipient: Author,
        _timeout: Duration,
    ) -> anyhow::Result<Batch> {
        unimplemented!();
    }

    async fn send_batch(&self, batch: Batch, recipients: Vec<Author>) {
        self.tx
            .send((ConsensusMsg::BatchResponse(Box::new(batch)), recipients))
            .await
            .expect("could not send");
    }

    async fn send_signed_batch_info(
        &self,
        signed_batch_info: SignedBatchInfo,
        recipients: Vec<Author>,
    ) {
        self.tx
            .send((
                ConsensusMsg::SignedBatchInfo(Box::new(signed_batch_info)),
                recipients,
            ))
            .await
            .expect("could not send");
    }

    async fn broadcast_batch_msg(&mut self, _batch: Batch) {
        unimplemented!()
    }

    async fn broadcast_proof_of_store(&mut self, proof_of_store: ProofOfStore) {
        self.tx
            .send((
                ConsensusMsg::ProofOfStoreMsg(Box::new(proof_of_store)),
                vec![],
            ))
            .await
            .unwrap();
    }
}
