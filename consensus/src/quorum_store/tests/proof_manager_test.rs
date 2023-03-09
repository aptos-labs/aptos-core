// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::quorum_store::proof_manager::ProofManager;
use aptos_consensus_types::{
    common::{Payload, PayloadFilter},
    proof_of_store::{LogicalTime, ProofOfStore, SignedDigestInfo},
    request_response::{GetPayloadCommand, GetPayloadResponse},
};
use aptos_crypto::HashValue;
use aptos_types::{aggregate_signature::AggregateSignature, PeerId};
use futures::channel::oneshot;
use std::collections::HashSet;

#[tokio::test]
async fn test_block_request() {
    let mut proof_manager = ProofManager::new(0, 10, 10);

    let digest = HashValue::random();
    let proof = ProofOfStore::new(
        SignedDigestInfo::new(PeerId::random(), digest, LogicalTime::new(0, 10), 1, 1),
        AggregateSignature::empty(),
    );
    proof_manager.handle_remote_proof(proof.clone());

    let (callback_tx, callback_rx) = oneshot::channel();
    let req = GetPayloadCommand::GetPayloadRequest(
        1,
        100,
        1000000,
        true,
        PayloadFilter::InQuorumStore(HashSet::new()),
        callback_tx,
    );
    proof_manager.handle_proposal_request(req);
    let GetPayloadResponse::GetPayloadResponse(payload) = callback_rx.await.unwrap().unwrap();
    if let Payload::InQuorumStore(proofs) = payload {
        assert_eq!(proofs.proofs.len(), 1);
        assert_eq!(proofs.proofs[0], proof);
    } else {
        panic!("Unexpected variant")
    }
}
