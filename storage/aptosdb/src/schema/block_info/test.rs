// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::*;
use aptos_schemadb::{schema::fuzzing::assert_encode_decode, test_no_panic_decoding};
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_encode_decode(
        block_height in any::<u64>(),
        block_info in any::<BlockInfo>(),
    ) {
        assert_encode_decode::<BlockInfoSchema>(&block_height, &block_info);
    }
}

test_no_panic_decoding!(BlockInfoSchema);
