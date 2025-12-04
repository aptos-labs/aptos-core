// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::*;
use aptos_schemadb::{schema::fuzzing::assert_encode_decode, test_no_panic_decoding};
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_encode_decode(
        block_start_version in any::<u64>(),
        block_height in any::<u64>(),
    ) {
        assert_encode_decode::<BlockByVersionSchema>(&block_start_version, &block_height);
    }
}

test_no_panic_decoding!(BlockByVersionSchema);
