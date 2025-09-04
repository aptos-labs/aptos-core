// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    quorum_store::types::{Batch, BatchRequest},
    test_utils::create_vec_signed_transactions,
};
use velor_consensus_types::common::BatchPayload;
use velor_crypto::{hash::CryptoHash, HashValue};
use velor_types::{account_address::AccountAddress, quorum_store::BatchId};
use claims::{assert_err, assert_ok};

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

    assert_ok!(batch.verify());
    assert_ok!(batch.verify_with_digest(digest));
    // verify should fail if the digest does not match.
    assert_err!(batch.verify_with_digest(HashValue::random()));

    assert_eq!(batch.into_transactions(), signed_txns);
}
