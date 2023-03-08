// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    network::QuorumStoreSender,
    network_interface::ConsensusMsg,
    quorum_store::types::{Batch, BatchRequest, Fragment},
};
use aptos_consensus_types::{
    common::Author,
    proof_of_store::{ProofOfStore, SignedDigest},
};
use std::time::Duration;
use tokio::sync::mpsc::Sender;

#[derive(Clone)]
pub struct MockQuorumStoreSender {
    tx: Sender<(ConsensusMsg, Vec<Author>)>,
}

impl MockQuorumStoreSender {
    pub fn new(rx: Sender<(ConsensusMsg, Vec<Author>)>) -> Self {
        Self { tx: rx }
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
            .send((ConsensusMsg::BatchMsg(Box::new(batch)), recipients))
            .await
            .expect("could not send");
    }

    async fn send_signed_digest(&self, signed_digest: SignedDigest, recipients: Vec<Author>) {
        self.tx
            .send((
                ConsensusMsg::SignedDigestMsg(Box::new(signed_digest)),
                recipients,
            ))
            .await
            .expect("could not send");
    }

    async fn broadcast_fragment(&mut self, _fragment: Fragment) {
        unimplemented!()
    }

    async fn broadcast_proof_of_store(&mut self, _proof_of_store: ProofOfStore) {
        unimplemented!()
    }
}
