// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::quorum_store::{
    batch_generator::ProofError,
    proof_coordinator::{ProofCoordinator, ProofCoordinatorCommand},
    proof_manager::ProofManagerCommand,
    tests::utils::{compute_digest_from_signed_transaction, create_vec_signed_transactions},
    types::BatchId,
};
use aptos_consensus_types::proof_of_store::{LogicalTime, SignedDigest, SignedDigestInfo};
use aptos_types::validator_verifier::random_validator_verifier;
use futures::channel::oneshot;
use tokio::sync::mpsc::{channel, error::TryRecvError};

#[tokio::test(flavor = "multi_thread")]
async fn test_proof_coordinator_basic() {
    let (signers, verifier) = random_validator_verifier(4, None, true);
    let proof_coordinator = ProofCoordinator::new(100, signers[0].author());
    let (proof_coordinator_tx, proof_coordinator_rx) = channel(100);
    let (proof_manager_tx, mut proof_manager_rx) = channel(100);
    tokio::spawn(proof_coordinator.start(proof_coordinator_rx, proof_manager_tx, verifier.clone()));

    let batch_author = signers[0].author();
    let digest = compute_digest_from_signed_transaction(create_vec_signed_transactions(100));
    let signed_digest_info =
        SignedDigestInfo::new(batch_author, digest, LogicalTime::new(1, 20), 1, 1);
    let (proof_tx, proof_rx) = oneshot::channel();

    assert!(proof_coordinator_tx
        .send(ProofCoordinatorCommand::InitProof(
            signed_digest_info.clone(),
            BatchId::new_for_test(0),
            proof_tx
        ))
        .await
        .is_ok());
    for signer in &signers {
        let signed_digest = SignedDigest::new(
            batch_author,
            1,
            digest,
            LogicalTime::new(1, 20),
            1,
            1,
            signer,
        )
        .unwrap();
        assert!(proof_coordinator_tx
            .send(ProofCoordinatorCommand::AppendSignature(signed_digest))
            .await
            .is_ok());
    }

    // check normal path
    let (proof, batch_id) = proof_rx.await.expect("channel dropped").unwrap();
    assert_eq!(batch_id, BatchId::new_for_test(0));
    assert_eq!(proof.digest().clone(), digest);
    assert!(proof.verify(&verifier).is_ok());
    match proof_manager_rx.recv().await.expect("channel dropped") {
        ProofManagerCommand::LocalProof(cmd_proof) => assert_eq!(proof, cmd_proof),
        msg => panic!("Expected LocalProof but received: {:?}", msg),
    }

    // check that error path
    let (proof_tx, proof_rx) = oneshot::channel();
    assert!(proof_coordinator_tx
        .send(ProofCoordinatorCommand::InitProof(
            signed_digest_info.clone(),
            BatchId::new_for_test(4),
            proof_tx
        ))
        .await
        .is_ok());
    assert_eq!(
        proof_rx.await.expect("channel dropped"),
        Err(ProofError::Timeout(BatchId::new_for_test(4)))
    );
    match proof_manager_rx.try_recv() {
        Err(TryRecvError::Empty) => {},
        result => panic!("Expected Empty but instead: {:?}", result),
    }

    // check same digest after expiration
    let (proof_tx, proof_rx) = oneshot::channel();
    assert!(proof_coordinator_tx
        .send(ProofCoordinatorCommand::InitProof(
            signed_digest_info.clone(),
            BatchId::new_for_test(4),
            proof_tx
        ))
        .await
        .is_ok());
    for signer in &signers {
        let signed_digest = SignedDigest::new(
            batch_author,
            1,
            digest,
            LogicalTime::new(1, 20),
            1,
            1,
            signer,
        )
        .unwrap();
        assert!(proof_coordinator_tx
            .send(ProofCoordinatorCommand::AppendSignature(signed_digest))
            .await
            .is_ok());
    }
    let (proof, batch_id) = proof_rx.await.expect("channel dropped").unwrap();
    assert_eq!(batch_id, BatchId::new_for_test(4));
    assert_eq!(proof.digest().clone(), digest);
    assert!(proof.verify(&verifier).is_ok());
    match proof_manager_rx.recv().await.expect("channel dropped") {
        ProofManagerCommand::LocalProof(cmd_proof) => assert_eq!(proof, cmd_proof),
        msg => panic!("Expected LocalProof but received: {:?}", msg),
    }

    // check wrong signatures
    let (proof_tx, proof_rx) = oneshot::channel();
    assert!(proof_coordinator_tx
        .send(ProofCoordinatorCommand::InitProof(
            signed_digest_info,
            BatchId::new_for_test(10),
            proof_tx
        ))
        .await
        .is_ok());
    for _ in 0..signers.len() {
        let signed_digest = SignedDigest::new(
            batch_author,
            1,
            digest,
            LogicalTime::new(1, 20),
            1,
            1,
            &signers[1],
        )
        .unwrap();
        assert!(proof_coordinator_tx
            .send(ProofCoordinatorCommand::AppendSignature(signed_digest))
            .await
            .is_ok());
    }
    assert_eq!(
        proof_rx.await.expect("channel dropped"),
        Err(ProofError::Timeout(BatchId::new_for_test(10)))
    );
    match proof_manager_rx.try_recv() {
        Err(TryRecvError::Empty) => {},
        result => panic!("Expected Empty but instead: {:?}", result),
    }
}
