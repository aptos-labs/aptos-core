// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::*;
use velor_schemadb::{schema::fuzzing::assert_encode_decode, test_no_panic_decoding};
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_encode_decode(txn in any::<Transaction>()) {
        assert_encode_decode::<TransactionSchema>(&0u64, &txn);
    }
}

test_no_panic_decoding!(TransactionSchema);
