// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::*;
use velor_schemadb::{schema::fuzzing::assert_encode_decode, test_no_panic_decoding};
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
