// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use super::*;
use diem_types::transaction::{TransactionInfo, Version};
use proptest::prelude::*;
use schemadb::{schema::fuzzing::assert_encode_decode, test_no_panic_decoding};

proptest! {
    #[test]
    fn test_encode_decode(version in any::<Version>(), txn_info in any::<TransactionInfo>()) {
        assert_encode_decode::<TransactionInfoSchema>(&version, &txn_info);
    }
}

test_no_panic_decoding!(TransactionInfoSchema);
