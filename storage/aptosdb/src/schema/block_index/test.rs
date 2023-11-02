// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::*;
use aptos_schemadb::{schema::fuzzing::assert_encode_decode, test_no_panic_decoding};
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_encode_decode(
        block_height in any::<u64>(),
        start_version in any::<u64>(),
    ) {
        assert_encode_decode::<BlockIndexSchema>(&block_height, &start_version);
    }
}

test_no_panic_decoding!(BlockIndexSchema);
