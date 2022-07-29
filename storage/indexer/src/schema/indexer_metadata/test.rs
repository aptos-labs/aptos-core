// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::*;
use proptest::prelude::*;
use schemadb::{schema::fuzzing::assert_encode_decode, test_no_panic_decoding};

proptest! {
    #[test]
    fn test_encode_decode(
        tag in any::<MetadataTag>(),
        metadata in any::<Metadata>(),
    ) {
        assert_encode_decode::<IndexerMetadataSchema>(&tag, &metadata);
    }
}

test_no_panic_decoding!(IndexerMetadataSchema);
