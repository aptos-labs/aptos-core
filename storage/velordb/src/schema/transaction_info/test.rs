// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::*;
use velor_schemadb::{schema::fuzzing::assert_encode_decode, test_no_panic_decoding};
use velor_types::transaction::{TransactionInfo, Version};
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_encode_decode(version in any::<Version>(), txn_info in any::<TransactionInfo>()) {
        assert_encode_decode::<TransactionInfoSchema>(&version, &txn_info);
    }
}

test_no_panic_decoding!(TransactionInfoSchema);
