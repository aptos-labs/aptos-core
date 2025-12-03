// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Innovation-Enabling Source Code License

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
