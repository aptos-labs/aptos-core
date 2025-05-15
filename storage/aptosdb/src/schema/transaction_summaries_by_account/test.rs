// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::*;
use aptos_schemadb::{schema::fuzzing::assert_encode_decode, test_no_panic_decoding};
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_encode_decode(
        address in any::<AccountAddress>(),
        version in any::<Version>(),
        summary in any::<IndexedTransactionSummary>(),
    ) {
        assert_encode_decode::<TransactionSummariesByAccountSchema>(&(address, version), &summary);
    }
}

test_no_panic_decoding!(TransactionSummariesByAccountSchema);
