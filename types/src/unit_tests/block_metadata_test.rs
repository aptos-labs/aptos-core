// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::block_metadata::BlockMetadata;
use bcs::test_helpers::assert_canonical_encode_decode;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(20))]

    #[test]
    fn test_block_metadata_canonical_serialization(data in any::<BlockMetadata>()) {
        assert_canonical_encode_decode(data);
    }
}
