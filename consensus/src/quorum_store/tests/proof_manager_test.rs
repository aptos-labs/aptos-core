// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::quorum_store::proof_manager::ProofManager;
use aptos_consensus_types::{
    common::{Payload, PayloadFilter},
    proof_of_store::{BatchId, BatchInfo, ProofOfStore},
    request_response::{GetPayloadCommand, GetPayloadResponse},
};
use aptos_crypto::HashValue;
use aptos_types::{aggregate_signature::AggregateSignature, PeerId};
use futures::channel::oneshot;
use move_core_types::account_address::AccountAddress;
use std::collections::HashSet;

#[tokio::test]
async fn test_block_request() {
    let mut proof_manager = ProofManager::new(AccountAddress::random(), 10, 10);

    let digest = HashValue::random();
    let batch_id = BatchId::new_for_test(1);
    let proof = ProofOfStore::new(
        BatchInfo::new(PeerId::random(), batch_id, 0, 10, digest, 1, 1, 0),
        AggregateSignature::empty(),
    );
    proof_manager.receive_proofs(vec![proof.clone()]);

    let (callback_tx, callback_rx) = oneshot::channel();
    let req = GetPayloadCommand::GetPayloadRequest(
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
