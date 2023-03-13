// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;
use crate::{
    network_interface::ConsensusMsg,
    quorum_store::{
        batch_store::BatchStore,
        proof_coordinator::{ProofCoordinator, ProofCoordinatorCommand},
        tests::utils::{compute_digest_from_signed_transaction, create_vec_signed_transactions},
    },
    test_utils::mock_quorum_store_sender::MockQuorumStoreSender,
};
use aptos_consensus_types::proof_of_store::{LogicalTime, ProofOfStore, SignedDigest};
use aptos_types::validator_verifier::random_validator_verifier;
use tokio::sync::mpsc::channel;
use tokio::sync::oneshot::Receiver;
use aptos_crypto::HashValue;
use aptos_executor_types::Error;
use aptos_types::transaction::SignedTransaction;
use crate::quorum_store::batch_store::BatchReader;

pub struct MockBatchReader {};

impl BatchReader for MockBatchReader {
    fn exists(&self, digest: &HashValue) -> bool {
        true
    }

    fn get_batch(&self, proof: ProofOfStore) -> Receiver<Result<Vec<SignedTransaction>, Error>> {
        unimplemented!();
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_proof_coordinator_basic() {
    aptos_logger::Logger::init_for_testing();
    let (signers, verifier) = random_validator_verifier(4, None, true);
    let proof_coordinator = ProofCoordinator::new(100, signers[0].author(), Arc::new(MockBatchReader));
    let (proof_coordinator_tx, proof_coordinator_rx) = channel(100);
    let (tx, mut rx) = channel(100);
    let network_sender = MockQuorumStoreSender::new(tx);
    tokio::spawn(proof_coordinator.start(proof_coordinator_rx, network_sender, verifier.clone()));

    let batch_author = signers[0].author();
    let digest = compute_digest_from_signed_transaction(create_vec_signed_transactions(100));

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

    let proof = match rx.recv().await.expect("channel dropped") {
        (ConsensusMsg::ProofOfStoreMsg(proof), _) => *proof,
        msg => panic!("Expected LocalProof but received: {:?}", msg),
    };
    // check normal path
    assert_eq!(proof.digest().clone(), digest);
    assert!(proof.verify(&verifier).is_ok());
}
