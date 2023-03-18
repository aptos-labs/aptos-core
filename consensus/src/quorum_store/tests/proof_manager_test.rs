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
        BatchInfo::new(PeerId::random(), batch_id, 0, 10, digest, 1, 1),
        AggregateSignature::empty(),
    );
    proof_manager.receive_proof(proof.clone());

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

#[tokio::test]
async fn test_block_timestamp_expiration() {
    let mut proof_manager = ProofManager::new(AccountAddress::random(), 10, 10);

    let digest = HashValue::random();
    let batch_id = BatchId::new_for_test(1);
    let proof = ProofOfStore::new(
        BatchInfo::new(PeerId::random(), batch_id, 0, 10, digest, 1, 1),
        AggregateSignature::empty(),
    );
    proof_manager.receive_proof(proof.clone());

    proof_manager.handle_commit_notification(1, vec![]);

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

    proof_manager.handle_commit_notification(20, vec![]);

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
        assert_eq!(proofs.proofs.len(), 0);
    } else {
        panic!("Unexpected variant")
    }
}

#[tokio::test]
async fn test_batch_commit() {
    let mut proof_manager = ProofManager::new(AccountAddress::random(), 10, 10);

    let digest = HashValue::random();
    let batch_id = BatchId::new_for_test(1);
    let proof0 = ProofOfStore::new(
        BatchInfo::new(PeerId::random(), batch_id, 0, 10, digest, 1, 1),
        AggregateSignature::empty(),
    );
    proof_manager.receive_proof(proof0.clone());

    let digest = HashValue::random();
    let batch_id = BatchId::new_for_test(1);
    let proof1 = ProofOfStore::new(
        BatchInfo::new(PeerId::random(), batch_id, 0, 11, digest, 1, 1),
        AggregateSignature::empty(),
    );
    proof_manager.receive_proof(proof1.clone());

    proof_manager.handle_commit_notification(1, vec![proof1.info().clone()]);

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
        assert!(proofs.proofs.contains(&proof0));
    } else {
        panic!("Unexpected variant")
    }
}

fn create_proof(author: PeerId, expiration: u64, batch_sequence: u64) -> ProofOfStore {
    let digest = HashValue::random();
    let batch_id = BatchId::new_for_test(batch_sequence);
    ProofOfStore::new(
        BatchInfo::new(author, batch_id, 0, expiration, digest, 1, 1),
        AggregateSignature::empty(),
    )
}

#[tokio::test]
async fn test_proposal_fairness() {
    let mut proof_manager = ProofManager::new(AccountAddress::random(), 10, 10);
    let peer0 = PeerId::random();
    let peer1 = PeerId::random();

    let mut peer0_proofs = vec![];
    for i in 0..4 {
        let proof = create_proof(peer0, 10 + i, 1 + i);
        proof_manager.receive_proof(proof.clone());
        peer0_proofs.push(proof);
    }

    let peer1_proof_0 = create_proof(peer1, 7, 1);
    proof_manager.receive_proof(peer1_proof_0.clone());

    // Without filter, and large max size, all proofs are retrieved
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
        assert_eq!(proofs.proofs.len(), 5);
    } else {
        panic!("Unexpected variant")
    }

    let (callback_tx, callback_rx) = oneshot::channel();
    let req = GetPayloadCommand::GetPayloadRequest(
        2,
        1000000,
        true,
        PayloadFilter::InQuorumStore(HashSet::new()),
        callback_tx,
    );
    proof_manager.handle_proposal_request(req);
    let GetPayloadResponse::GetPayloadResponse(payload) = callback_rx.await.unwrap().unwrap();
    if let Payload::InQuorumStore(proofs) = payload {
        assert_eq!(proofs.proofs.len(), 2);
        assert!(proofs.proofs.contains(&peer0_proofs[0]));
        assert!(proofs.proofs.contains(&peer1_proof_0));
    } else {
        panic!("Unexpected variant")
    }

    let mut filter = HashSet::new();
    filter.insert(peer0_proofs[0].info().clone());
    filter.insert(peer1_proof_0.info().clone());
    let (callback_tx, callback_rx) = oneshot::channel();
    let req = GetPayloadCommand::GetPayloadRequest(
        2,
        1000000,
        true,
        PayloadFilter::InQuorumStore(filter),
        callback_tx,
    );
    proof_manager.handle_proposal_request(req);
    let GetPayloadResponse::GetPayloadResponse(payload) = callback_rx.await.unwrap().unwrap();
    if let Payload::InQuorumStore(proofs) = payload {
        assert_eq!(proofs.proofs.len(), 2);
        assert!(proofs.proofs.contains(&peer0_proofs[1]));
        assert!(proofs.proofs.contains(&peer0_proofs[2]));
    } else {
        panic!("Unexpected variant")
    }

    let mut filter = HashSet::new();
    filter.insert(peer0_proofs[0].info().clone());
    filter.insert(peer1_proof_0.info().clone());
    filter.insert(peer0_proofs[1].info().clone());
    filter.insert(peer0_proofs[2].info().clone());
    let (callback_tx, callback_rx) = oneshot::channel();
    let req = GetPayloadCommand::GetPayloadRequest(
        2,
        1000000,
        true,
        PayloadFilter::InQuorumStore(filter),
        callback_tx,
    );
    proof_manager.handle_proposal_request(req);
    let GetPayloadResponse::GetPayloadResponse(payload) = callback_rx.await.unwrap().unwrap();
    if let Payload::InQuorumStore(proofs) = payload {
        assert_eq!(proofs.proofs.len(), 1);
        assert!(proofs.proofs.contains(&peer0_proofs[3]));
    } else {
        panic!("Unexpected variant")
    }
}
