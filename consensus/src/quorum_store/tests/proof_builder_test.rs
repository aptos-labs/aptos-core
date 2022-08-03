// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::quorum_store::proof_builder::{ProofBuilder, ProofBuilderCommand};
use crate::quorum_store::tests::utils::{
    compute_digest_from_signed_transaction, create_vec_signed_transactions,
};
use aptos_types::validator_signer::ValidatorSigner;
use aptos_types::validator_verifier::random_validator_verifier;
use consensus_types::proof_of_store::{LogicalTime, SignedDigest};
use futures::channel::oneshot;
use std::sync::Arc;
use tokio::runtime;
use tokio::sync::mpsc::channel;

#[tokio::test(flavor = "multi_thread")]
async fn test_proof_builder_basic() {

    let (signers, verifier) = random_validator_verifier(4, None, true);
    let arc_signers: Vec<Arc<ValidatorSigner>> = signers.into_iter().map(|s| Arc::new(s)).collect();
    let proof_builder = ProofBuilder::new(100);
    let (proof_builder_tx, proof_builder_rx) = channel(100);
    tokio::spawn(proof_builder.start(proof_builder_rx, verifier.clone()));

    let digest = compute_digest_from_signed_transaction(create_vec_signed_transactions(100));
    let signed_digest =
        SignedDigest::new(1, digest, LogicalTime::new(1, 20), arc_signers[0].clone());
    let (proof_tx, proof_rx) = oneshot::channel();

    assert!(proof_builder_tx.send(ProofBuilderCommand::InitProof(signed_digest.clone(), 0, proof_tx)).await.is_ok());
    for i in 1..arc_signers.len() {
        let signed_digest =
            SignedDigest::new(1, digest, LogicalTime::new(1, 20), arc_signers[i].clone());
        assert!(proof_builder_tx.send(ProofBuilderCommand::AppendSignature(signed_digest)).await.is_ok());
    }

    let (proof, batch_id) = proof_rx
        .await
        .expect("channel dropped")
        .unwrap();
    assert_eq!(batch_id, 0);
    assert_eq!(proof.digest().clone(), digest);
    assert!(proof.verify(&verifier).is_ok());


    let (proof_tx, proof_rx) = oneshot::channel();
    assert!(proof_builder_tx.send(ProofBuilderCommand::InitProof(signed_digest, 4, proof_tx)).await.is_ok());
    assert!(proof_rx.await.expect("channel dropped").is_err());
}
