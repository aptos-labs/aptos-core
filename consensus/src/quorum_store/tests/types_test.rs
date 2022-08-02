// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use bcs::{from_bytes, to_bytes};
use aptos_crypto::hash::DefaultHasher;
use aptos_types::account_address::AccountAddress;
use consensus_types::proof_of_store::LogicalTime;
use crate::quorum_store::tests::utils::create_vec_signed_transactions;
use crate::quorum_store::types::{Batch, Fragment, SerializedTransaction};

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

    let empty_batch = Batch::new(
        epoch,
        source,
        digest,
        None,
    );

    assert_eq!(epoch, empty_batch.epoch());
    assert!(empty_batch.verify(source).is_ok());

    let batch = Batch::new(
        epoch,
        source,
        digest,
        Some(signed_txns.clone()),
    );

    assert!(batch.verify(source).is_ok());
    assert_eq!(batch.get_payload(), signed_txns);
}


#[test]
fn test_fragment(){
    let mut epoch = 0;
    let mut batch_id = 0;
    let mut fragment_id = 0;
    let mut data = Vec::new();
    let mut maybe_expiration = None;
    let mut source = AccountAddress::random();

    let signed_txns = create_vec_signed_transactions(500);
    for txn in signed_txns.iter(){
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
    while wrong_source == source{
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

    let serialized_txns = fragment.take_transactions();
    assert_eq!(serialized_txns, data.clone());

    let mut returned_signed_transactions = Vec::new();
    for mut txn in data{
        match from_bytes(&txn.take_bytes()) {
            Ok(signed_txn) => returned_signed_transactions.push(signed_txn),
            Err(_) => {
                panic!();
            }
        }
    }
    assert_eq!(signed_txns, returned_signed_transactions);
}