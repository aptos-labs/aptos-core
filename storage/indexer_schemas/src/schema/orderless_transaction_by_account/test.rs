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
        nonce in any::<u64>(),
        version in any::<Version>(),
    ) {
        assert_encode_decode::<OrderlessTransactionByAccountSchema>(&(address, nonce), &version);
    }
}

test_no_panic_decoding!(OrderlessTransactionByAccountSchema);
