// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::network::QuorumStoreSender;
use crate::network_interface::ConsensusMsg;
use crate::quorum_store::types::Batch;
use consensus_types::common::Author;
use consensus_types::proof_of_store::SignedDigest;
use tokio::sync::mpsc::Sender;

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
}
