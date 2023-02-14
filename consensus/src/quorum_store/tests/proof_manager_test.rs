// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::quorum_store::{batch_reader::BatchReader, proof_manager::ProofManager};
use aptos_consensus_types::{
    common::{Payload, PayloadFilter},
    proof_of_store::{LogicalTime, ProofOfStore, SignedDigestInfo},
    request_response::{BlockProposalCommand, ConsensusResponse},
};
use aptos_crypto::HashValue;
use aptos_types::{account_address::AccountAddress, aggregate_signature::AggregateSignature};
use futures::channel::oneshot;
use std::collections::{HashMap, HashSet};
use tokio::sync::mpsc::channel;

#[tokio::test]
async fn test_block_request() {
    let num_experiments = 2000;

    let (tx1, _rx1) = channel(num_experiments + 50);
    let (self_tx, _rx2) = channel(1); // Shouldn't send anything to self in this test.
    let (batch_reader, _) = BatchReader::new(
        10,                       // epoch
        10,                       // last committed round
        HashMap::new(),           // db recovery state
        AccountAddress::random(), // self peer id
        tx1,                      // batch store sender
        self_tx,                  // self sender
        0,
        0,
        2100,
        0,    // grace period rounds
        0,    // memory_quota
        1000, // db quota
    );

    let mut proof_manager = ProofManager::new(0, 10, batch_reader);

    let digest = HashValue::random();
    let proof = ProofOfStore::new(
        SignedDigestInfo::new(digest, LogicalTime::new(0, 10), 1, 1),
        AggregateSignature::empty(),
    );
    proof_manager.handle_remote_proof(proof.clone());

    let (callback_tx, callback_rx) = oneshot::channel();
    let req = BlockProposalCommand::GetBlockRequest(
        1,
        100,
        1000000,
        PayloadFilter::InQuorumStore(HashSet::new()),
        callback_tx,
    );
    proof_manager.handle_proposal_request(req).await;
    let ConsensusResponse::GetBlockResponse(payload) = callback_rx.await.unwrap().unwrap();
    if let Payload::InQuorumStore(proofs) = payload {
        assert_eq!(proofs.proofs.len(), 1);
        assert_eq!(proofs.proofs[0], proof);
    } else {
        panic!("Unexpected variant")
    }
}
