// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Innovation-Enabling Source Code License

use super::*;
use aptos_schemadb::{schema::fuzzing::assert_encode_decode, test_no_panic_decoding};
use aptos_types::transaction::{TransactionAuxiliaryData, Version};
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_encode_decode(version in any::<Version>(), txn in any::<TransactionAuxiliaryData>()) {
        assert_encode_decode::<TransactionAuxiliaryDataSchema>(&version, &txn);
    }
}

test_no_panic_decoding!(TransactionAuxiliaryDataSchema);
