// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::quorum_store::{
    tests::utils::create_vec_signed_transactions,
    types::{Batch, BatchId, BatchRequest, Fragment, SerializedTransaction},
};
use aptos_consensus_types::proof_of_store::LogicalTime;
use aptos_crypto::hash::DefaultHasher;
use aptos_types::account_address::AccountAddress;
use bcs::{from_bytes, to_bytes};

#[test]
fn test_batch() {
    let epoch = 0;
    let source = AccountAddress::random();
    let signed_txns = create_vec_signed_transactions(500);

    let mut hasher = DefaultHasher::new(b"QuorumStoreBatch");
    for txn in signed_txns.iter() {
        hasher.update(&to_bytes(txn).unwrap());
    }
    let digest = hasher.finish();

    let batch_request = BatchRequest::new(source, epoch, digest);

    assert_eq!(epoch, batch_request.epoch());
    assert!(batch_request.verify(source).is_ok());

    let batch = Batch::new(source, epoch, digest, signed_txns.clone());

    assert!(batch.verify().is_ok());
    assert_eq!(batch.into_payload(), signed_txns);
}

#[test]
fn test_fragment() {
    let epoch = 0;
    let batch_id = BatchId::new_for_test(0);
    let fragment_id = 0;
    let mut data = Vec::new();
    let mut maybe_expiration = None;
    let source = AccountAddress::random();

    let signed_txns = create_vec_signed_transactions(500);
    for txn in signed_txns.iter() {
        data.push(SerializedTransaction::from_signed_txn(txn));
    }

    let fragment = Fragment::new(
        epoch,
        batch_id,
        fragment_id,
        data.clone(),
        maybe_expiration,
        source,
    );
    assert!(fragment.verify(source).is_ok());

    maybe_expiration = Some(LogicalTime::new(epoch, 0));
    let fragment = Fragment::new(
        epoch,
        batch_id,
        fragment_id,
        data.clone(),
        maybe_expiration,
        source,
    );
    assert!(fragment.verify(source).is_ok());

    maybe_expiration = Some(LogicalTime::new(epoch + 1, 0));
    let fragment = Fragment::new(
        epoch,
        batch_id,
        fragment_id,
        data.clone(),
        maybe_expiration,
        source,
    );
    assert!(fragment.verify(source).is_err());

    maybe_expiration = None;
    let mut wrong_source = AccountAddress::random();
    while wrong_source == source {
        wrong_source = AccountAddress::random();
    }
    let fragment = Fragment::new(
        epoch,
        batch_id,
        fragment_id,
        data.clone(),
        maybe_expiration,
        wrong_source,
    );
    assert!(fragment.verify(source).is_err());

    let fragment = Fragment::new(
        epoch,
        batch_id,
        fragment_id,
        data.clone(),
        maybe_expiration,
        source,
    );

    assert_eq!(fragment.epoch(), epoch);
    assert_eq!(fragment.fragment_id(), fragment_id);
    assert_eq!(fragment.source(), source);
    assert_eq!(fragment.batch_id(), batch_id);

    let serialized_txns = fragment.into_transactions();
    assert_eq!(serialized_txns, data.clone());

    let mut returned_signed_transactions = Vec::new();
    for mut txn in data {
        match from_bytes(&txn.take_bytes()) {
            Ok(signed_txn) => returned_signed_transactions.push(signed_txn),
            Err(_) => {
                panic!();
            },
        }
    }
    assert_eq!(signed_txns, returned_signed_transactions);
}
