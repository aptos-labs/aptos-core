// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    network::QuorumStoreSender,
    quorum_store::{
        batch_requester::BatchRequester,
        types::{Batch, BatchRequest, BatchResponse},
    },
};
use aptos_consensus_types::{
    common::Author,
    proof_of_store::{BatchId, ProofOfStore, SignedBatchInfo},
};
use aptos_crypto::HashValue;
use aptos_types::{
    aggregate_signature::{AggregateSignature, PartialSignatures},
    block_info::BlockInfo,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    validator_signer::ValidatorSigner,
    validator_verifier::{ValidatorConsensusInfo, ValidatorVerifier},
};
use move_core_types::account_address::AccountAddress;
use std::time::{Duration, Instant};

#[derive(Clone)]
struct MockBatchRequester {
    return_value: BatchResponse,
}

impl MockBatchRequester {
    fn new(return_value: BatchResponse) -> Self {
        Self { return_value }
    }
}

#[async_trait::async_trait]
impl QuorumStoreSender for MockBatchRequester {
    async fn send_batch_request(&self, _request: BatchRequest, _recipients: Vec<Author>) {
        unimplemented!()
    }

    async fn request_batch(
        &self,
        _request: BatchRequest,
        _recipient: Author,
        _timeout: Duration,
    ) -> anyhow::Result<BatchResponse> {
        Ok(self.return_value.clone())
    }

    async fn send_batch(&self, _batch: Batch, _recipients: Vec<Author>) {
        unimplemented!()
    }

    async fn send_signed_batch_info_msg(
        &self,
        _signed_batch_infos: Vec<SignedBatchInfo>,
        _recipients: Vec<Author>,
    ) {
        unimplemented!()
    }

    async fn broadcast_batch_msg(&mut self, _batches: Vec<Batch>) {
        unimplemented!()
    }

    async fn broadcast_proof_of_store_msg(&mut self, _proof_of_stores: Vec<ProofOfStore>) {
        unimplemented!()
    }

    async fn send_proof_of_store_msg_to_self(&mut self, _proof_of_stores: Vec<ProofOfStore>) {
        unimplemented!()
    }
}

#[tokio::test]
async fn test_batch_request_exists() {
    let batch = Batch::new(
        BatchId::new_for_test(1),
        vec![],
        1,
        1,
        AccountAddress::random(),
        0,
    );
    let batch_response = BatchResponse::Batch(batch.clone());

    let validator_signer = ValidatorSigner::random(None);
    let (tx, mut rx) = tokio::sync::oneshot::channel();
    let batch_requester = BatchRequester::new(
        1,
        AccountAddress::random(),
        1,
        2,
        1_000,
        1_000,
        MockBatchRequester::new(batch_response),
        ValidatorVerifier::new_single(validator_signer.author(), validator_signer.public_key()),
    );

    let result = batch_requester
        .request_batch(
            ProofOfStore::new(
                batch.batch_info().clone(),
                AggregateSignature::new(vec![u8::MAX].into(), None),
            ),
            tx,
        )
        .await;
    assert!(result.is_some());
    if let Some((batch_info, _payload)) = result {
        assert_eq!(batch_info, *batch.batch_info());
    }
    assert!(rx.try_recv().is_ok());
}

fn create_ledger_info_with_timestamp(
    timestamp: u64,
) -> (LedgerInfoWithSignatures, ValidatorVerifier) {
    const NUM_SIGNERS: u8 = 1;
    // Generate NUM_SIGNERS random signers.
    let validator_signers: Vec<ValidatorSigner> = (0..NUM_SIGNERS)
        .map(|i| ValidatorSigner::random([i; 32]))
        .collect();
    let block_info = BlockInfo::new(
        1,
        1,
        HashValue::random(),
        HashValue::random(),
        0,
        timestamp,
        None,
    );
    let ledger_info = LedgerInfo::new(block_info, HashValue::random());

    // Create a map from authors to public keys with equal voting power.
    let mut validator_infos = vec![];
    for validator in validator_signers.iter() {
        validator_infos.push(ValidatorConsensusInfo::new(
            validator.author(),
            validator.public_key(),
            1,
        ));
    }

    // Create a map from author to signatures.
    let mut partial_signature = PartialSignatures::empty();
    for validator in validator_signers.iter() {
        partial_signature.add_signature(validator.author(), validator.sign(&ledger_info).unwrap());
    }

    // Let's assume our verifier needs to satisfy all NUM_SIGNERS
    let validator_verifier =
        ValidatorVerifier::new_with_quorum_voting_power(validator_infos, NUM_SIGNERS as u128)
            .expect("Incorrect quorum size.");
    let aggregated_signature = validator_verifier
        .aggregate_signatures(&partial_signature)
        .unwrap();
    let ledger_info_with_signatures =
        LedgerInfoWithSignatures::new(ledger_info, aggregated_signature);

    (ledger_info_with_signatures, validator_verifier)
}

#[tokio::test]
async fn test_batch_request_not_exists_not_expired() {
    let retry_interval_ms = 1_000;
    let expiration = 10_000;

    // Batch has not expired yet
    let (ledger_info_with_signatures, validator_verifier) =
        create_ledger_info_with_timestamp(expiration - 1);

    let batch = Batch::new(
        BatchId::new_for_test(1),
        vec![],
        1,
        expiration,
        AccountAddress::random(),
        0,
    );
    let (tx, mut rx) = tokio::sync::oneshot::channel();
    let batch_response = BatchResponse::NotFound(ledger_info_with_signatures);
    let batch_requester = BatchRequester::new(
        1,
        AccountAddress::random(),
        1,
        2,
        retry_interval_ms,
        1_000,
        MockBatchRequester::new(batch_response),
        validator_verifier,
    );

    let request_start = Instant::now();
    let result = batch_requester
        .request_batch(
            ProofOfStore::new(
                batch.batch_info().clone(),
                AggregateSignature::new(vec![u8::MAX].into(), None),
            ),
            tx,
        )
        .await;
    let request_duration = request_start.elapsed();
    assert!(result.is_none());
    assert!(rx.try_recv().is_ok());
    // Retried at least once
    assert!(request_duration > Duration::from_millis(retry_interval_ms as u64));
}

#[tokio::test]
async fn test_batch_request_not_exists_expired() {
    let retry_interval_ms = 1_000;
    let expiration = 10_000;

    // Batch has expired according to the ledger info that will be returned
    let (ledger_info_with_signatures, validator_verifier) =
        create_ledger_info_with_timestamp(expiration + 1);

    let batch = Batch::new(
        BatchId::new_for_test(1),
        vec![],
        1,
        expiration,
        AccountAddress::random(),
        0,
    );
    let (tx, mut rx) = tokio::sync::oneshot::channel();
    let batch_response = BatchResponse::NotFound(ledger_info_with_signatures);
    let batch_requester = BatchRequester::new(
        1,
        AccountAddress::random(),
        1,
        2,
        retry_interval_ms,
        1_000,
        MockBatchRequester::new(batch_response),
        validator_verifier,
    );

    let request_start = Instant::now();
    let result = batch_requester
        .request_batch(
            ProofOfStore::new(
                batch.batch_info().clone(),
                AggregateSignature::new(vec![u8::MAX].into(), None),
            ),
            tx,
        )
        .await;
    let request_duration = request_start.elapsed();
    assert!(result.is_none());
    assert!(rx.try_recv().is_ok());
    // No retry because of short-circuiting of expired batch
    assert!(request_duration < Duration::from_millis(retry_interval_ms as u64));
}
