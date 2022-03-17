// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::*;
use proptest::prelude::*;
use schemadb::{schema::fuzzing::assert_encode_decode, test_no_panic_decoding};

proptest! {
    #[test]
    fn test_encode_decode(
        address in any::<AccountAddress>(),
        seq_num in any::<u64>(),
        version in any::<Version>(),
    ) {
        assert_encode_decode::<TransactionByAccountSchema>(&(address, seq_num), &version);
    }
}

test_no_panic_decoding!(TransactionByAccountSchema);
