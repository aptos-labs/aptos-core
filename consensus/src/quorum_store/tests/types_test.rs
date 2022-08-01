// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use bcs::to_bytes;
use aptos_crypto::hash::DefaultHasher;
use aptos_types::account_address::AccountAddress;
use crate::quorum_store::tests::utils::create_vec_signed_transactions;
use crate::quorum_store::types::Batch;

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


