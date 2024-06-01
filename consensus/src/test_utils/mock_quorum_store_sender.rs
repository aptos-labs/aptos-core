// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    network::QuorumStoreSender,
    network_interface::ConsensusMsg,
    quorum_store::types::{Batch, BatchRequest, BatchResponse},
};
use aptos_consensus_types::{
    common::Author,
    proof_of_store::{ProofOfStore, ProofOfStoreMsg, SignedBatchInfo, SignedBatchInfoMsg},
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
    ) -> anyhow::Result<BatchResponse> {
        unimplemented!();
    }

    async fn send_batch(&self, batch: Batch, recipients: Vec<Author>) {
        self.tx
            .send((ConsensusMsg::BatchResponse(Box::new(batch)), recipients))
            .await
            .expect("could not send");
    }

    async fn send_signed_batch_info_msg(
        &self,
        signed_batch_infos: Vec<SignedBatchInfo>,
        recipients: Vec<Author>,
    ) {
        self.tx
            .send((
                ConsensusMsg::SignedBatchInfo(Box::new(SignedBatchInfoMsg::new(
                    signed_batch_infos,
                ))),
                recipients,
            ))
            .await
            .expect("could not send");
    }

    async fn broadcast_batch_msg(&mut self, _batches: Vec<Batch>) {
        unimplemented!()
    }

    async fn broadcast_proof_of_store_msg(&mut self, proof_of_stores: Vec<ProofOfStore>) {
        self.tx
            .send((
                ConsensusMsg::ProofOfStoreMsg(Box::new(ProofOfStoreMsg::new(proof_of_stores))),
                vec![],
            ))
            .await
            .unwrap();
    }

    async fn send_proof_of_store_msg_to_self(&mut self, _proof_of_stores: Vec<ProofOfStore>) {
        unimplemented!()
    }
}
