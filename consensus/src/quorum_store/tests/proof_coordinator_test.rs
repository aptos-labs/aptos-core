// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    network_interface::ConsensusMsg,
    quorum_store::{
        batch_store::BatchReader,
        proof_coordinator::{ProofCoordinator, ProofCoordinatorCommand},
        types::Batch,
    },
    test_utils::{create_vec_signed_transactions, mock_quorum_store_sender::MockQuorumStoreSender},
};
use aptos_consensus_types::proof_of_store::{BatchInfo, SignedBatchInfo, SignedBatchInfoMsg};
use aptos_crypto::HashValue;
use aptos_executor_types::ExecutorResult;
use aptos_types::{
    quorum_store::BatchId, transaction::SignedTransaction,
    validator_verifier::random_validator_verifier, PeerId,
};
use futures::future::Shared;
use mini_moka::sync::Cache;
use std::{future::Future, pin::Pin, sync::Arc};
use tokio::sync::mpsc::channel;

pub struct MockBatchReader {
    peer: PeerId,
}

impl BatchReader for MockBatchReader {
    fn exists(&self, _digest: &HashValue) -> Option<PeerId> {
        Some(self.peer)
    }

    fn get_batch(
        &self,
        _batch_info: BatchInfo,
        _signers: Vec<PeerId>,
    ) -> Shared<Pin<Box<dyn Future<Output = ExecutorResult<Vec<SignedTransaction>>> + Send>>> {
        unimplemented!()
    }

    fn update_certified_timestamp(&self, _certified_time: u64) {
        unimplemented!()
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_proof_coordinator_basic() {
    aptos_logger::Logger::init_for_testing();
    let (signers, verifier) = random_validator_verifier(4, None, true);
    let (tx, _rx) = channel(100);
    let proof_cache = Cache::builder().build();
    let proof_coordinator = ProofCoordinator::new(
        100,
        signers[0].author(),
        Arc::new(MockBatchReader {
            peer: signers[0].author(),
        }),
        tx,
        proof_cache.clone(),
        true,
        10,
    );
    let (proof_coordinator_tx, proof_coordinator_rx) = channel(100);
    let (tx, mut rx) = channel(100);
    let network_sender = MockQuorumStoreSender::new(tx);
    let verifier = Arc::new(verifier);
    tokio::spawn(proof_coordinator.start(proof_coordinator_rx, network_sender, verifier.clone()));

    let batch_author = signers[0].author();
    let batch_id = BatchId::new_for_test(1);
    let payload = create_vec_signed_transactions(100);
    let batch = Batch::new(batch_id, payload, 1, 20, batch_author, 0);
    let digest = batch.digest();

    for signer in &signers {
        let signed_batch_info = SignedBatchInfo::new(batch.batch_info().clone(), signer).unwrap();
        assert!(proof_coordinator_tx
            .send(ProofCoordinatorCommand::AppendSignature(
                signer.author(),
                SignedBatchInfoMsg::new(vec![signed_batch_info])
            ))
            .await
            .is_ok());
    }

    let proof_msg = match rx.recv().await.expect("channel dropped") {
        (ConsensusMsg::ProofOfStoreMsg(proof_msg), _) => *proof_msg,
        msg => panic!("Expected LocalProof but received: {:?}", msg),
    };
    // check normal path
    assert!(proof_msg.verify(100, &verifier, &proof_cache).is_ok());
    let proofs = proof_msg.take();
    assert_eq!(proofs[0].digest(), digest);
}

// This is testing the bad path where some of the signatures in signed batch infos are bad.
#[tokio::test(flavor = "multi_thread")]
async fn test_proof_coordinator_with_unverified_signatures() {
    aptos_logger::Logger::init_for_testing();
    let (signers, verifier) = random_validator_verifier(10, Some(4), true);
    let verifier = Arc::new(verifier);
    let (tx, _rx) = channel(100);
    let proof_cache = Cache::builder().build();
    let proof_coordinator = ProofCoordinator::new(
        100,
        signers[0].author(),
        Arc::new(MockBatchReader {
            peer: signers[0].author(),
        }),
        tx,
        proof_cache.clone(),
        true,
        10,
    );
    let (proof_coordinator_tx, proof_coordinator_rx) = channel(100);
    let (tx, mut rx) = channel(100);
    let network_sender = MockQuorumStoreSender::new(tx);
    tokio::spawn(proof_coordinator.start(proof_coordinator_rx, network_sender, verifier.clone()));

    let batch_author = signers[0].author();
    for batch_index in 1..10 {
        let batch_id = BatchId::new_for_test(batch_index);
        let payload = create_vec_signed_transactions(100);
        let batch = Batch::new(batch_id, payload, 1, 20, batch_author, 0);
        let digest = batch.digest();

        for (signer_index, signer) in signers.iter().enumerate() {
            if signer_index > 2 {
                let signed_batch_info = SignedBatchInfo::new(batch.batch_info().clone(), signer)
                    .expect("Failed to create SignedBatchInfo");
                assert!(proof_coordinator_tx
                    .send(ProofCoordinatorCommand::AppendSignature(
                        signer.author(),
                        SignedBatchInfoMsg::new(vec![signed_batch_info]),
                    ))
                    .await
                    .is_ok())
            } else {
                let signed_batch_info =
                    SignedBatchInfo::dummy(batch.batch_info().clone(), signer.author());
                assert!(proof_coordinator_tx
                    .send(ProofCoordinatorCommand::AppendSignature(
                        signer.author(),
                        SignedBatchInfoMsg::new(vec![signed_batch_info]),
                    ))
                    .await
                    .is_ok());
            }
        }

        let proof_msg = match rx.recv().await.expect("channel dropped") {
            (ConsensusMsg::ProofOfStoreMsg(proof_msg), _) => *proof_msg,
            msg => panic!("Expected LocalProof but received: {:?}", msg),
        };

        let proofs = proof_msg.take();
        assert_eq!(proofs[0].digest(), digest);
        assert_eq!(verifier.pessimistic_verify_set().len(), 3);
    }
}
