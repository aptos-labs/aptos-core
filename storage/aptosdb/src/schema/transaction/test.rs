// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::*;
use proptest::prelude::*;
use schemadb::{schema::fuzzing::assert_encode_decode, test_no_panic_decoding};

proptest! {
    #[test]
    fn test_encode_decode(txn in any::<Transaction>()) {
        assert_encode_decode::<TransactionSchema>(&0u64, &txn);
    }
}

test_no_panic_decoding!(TransactionSchema);
