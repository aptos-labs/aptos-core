// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    network::QuorumStoreSender,
    network_interface::ConsensusMsg,
    quorum_store::types::{Batch, BatchRequest, BatchResponse},
};
use aptos_consensus_types::{
    common::Author,
    proof_of_store::{
        BatchInfo, BatchInfoExt, ProofOfStore, ProofOfStoreMsg, SignedBatchInfo, SignedBatchInfoMsg,
    },
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
    async fn request_batch(
        &self,
        _request: BatchRequest,
        _recipient: Author,
        _timeout: Duration,
    ) -> anyhow::Result<BatchResponse> {
        unimplemented!();
    }

    async fn send_signed_batch_info_msg(
        &self,
        signed_batch_infos: Vec<SignedBatchInfo<BatchInfo>>,
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

    async fn send_signed_batch_info_msg_v2(
        &self,
        signed_batch_infos: Vec<SignedBatchInfo<BatchInfoExt>>,
        recipients: Vec<Author>,
    ) {
        self.tx
            .send((
                ConsensusMsg::SignedBatchInfoMsgV2(Box::new(SignedBatchInfoMsg::new(
                    signed_batch_infos,
                ))),
                recipients,
            ))
            .await
            .expect("could not send");
    }

    async fn broadcast_batch_msg(&mut self, _batches: Vec<Batch<BatchInfo>>) {
        unimplemented!()
    }

    async fn broadcast_batch_msg_v2(&mut self, _batches: Vec<Batch<BatchInfoExt>>) {
        unimplemented!()
    }

    async fn broadcast_proof_of_store_msg(
        &mut self,
        proof_of_stores: Vec<ProofOfStore<BatchInfo>>,
    ) {
        self.tx
            .send((
                ConsensusMsg::ProofOfStoreMsg(Box::new(ProofOfStoreMsg::new(proof_of_stores))),
                vec![],
            ))
            .await
            .expect("We should be able to send the proof of store message");
    }

    async fn send_proof_of_store_msg_to_self(
        &mut self,
        _proof_of_stores: Vec<ProofOfStore<BatchInfoExt>>,
    ) {
        unimplemented!()
    }

    async fn broadcast_proof_of_store_msg_v2(
        &mut self,
        proof_of_stores: Vec<ProofOfStore<BatchInfoExt>>,
    ) {
        self.tx
            .send((
                ConsensusMsg::ProofOfStoreMsgV2(Box::new(ProofOfStoreMsg::new(proof_of_stores))),
                vec![],
            ))
            .await
            .expect("We should be able to send the proof of store message");
    }
}
