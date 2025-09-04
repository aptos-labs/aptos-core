// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::*;
use velor_schemadb::{schema::fuzzing::assert_encode_decode, test_no_panic_decoding};
use velor_types::transaction::{TransactionAuxiliaryData, Version};
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_encode_decode(version in any::<Version>(), txn in any::<TransactionAuxiliaryData>()) {
        assert_encode_decode::<TransactionAuxiliaryDataSchema>(&version, &txn);
    }
}

test_no_panic_decoding!(TransactionAuxiliaryDataSchema);
