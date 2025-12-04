// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::*;
use aptos_schemadb::{schema::fuzzing::assert_encode_decode, test_no_panic_decoding};
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_encode_decode(
        tag in any::<MetadataKey>(),
        metadata in any::<MetadataValue>(),
    ) {
        assert_encode_decode::<IndexerMetadataSchema>(&tag, &metadata);
    }

    #[test]
    fn test_encode_decode_internal_indexer_metadata(
        key in any::<MetadataKey>(),
        metadata in any::<MetadataValue>(),
    ) {
        assert_encode_decode::<InternalIndexerMetadataSchema>(&key, &metadata);
    }
}

test_no_panic_decoding!(IndexerMetadataSchema);
