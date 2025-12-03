// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Innovation-Enabling Source Code License

use super::*;
use aptos_schemadb::{schema::fuzzing::assert_encode_decode, test_no_panic_decoding};
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_encode_decode(
        address in any::<AccountAddress>(),
        seq_num in any::<u64>(),
        version in any::<Version>(),
    ) {
        assert_encode_decode::<OrderedTransactionByAccountSchema>(&(address, seq_num), &version);
    }
}

test_no_panic_decoding!(OrderedTransactionByAccountSchema);
