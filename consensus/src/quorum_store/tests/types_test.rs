// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::quorum_store::{
    tests::utils::create_vec_signed_transactions,
    types::{Batch, BatchPayload, BatchRequest},
};
use aptos_consensus_types::proof_of_store::BatchId;
use aptos_crypto::hash::CryptoHash;
use aptos_types::account_address::AccountAddress;

#[test]
fn test_batch() {
    let epoch = 0;
    let source = AccountAddress::random();
    let signed_txns = create_vec_signed_transactions(500);

    let payload = BatchPayload::new(source, signed_txns.clone());
    let digest = payload.hash();

    let batch_request = BatchRequest::new(source, epoch, digest);

    assert_eq!(epoch, batch_request.epoch());
    assert!(batch_request.verify(source).is_ok());

    let batch = Batch::new(
        BatchId::new_for_test(1),
        signed_txns.clone(),
        epoch,
        1,
        source,
        0,
    );

    assert!(batch.verify().is_ok());
    assert_eq!(batch.into_transactions(), signed_txns);
}
