// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// This file is a copy of the file storage/indexer/src/schema/indexer_metadata/test.rs.
/// At the end of the migration to migrate table info mapping
/// from storage critical path to indexer, the other file will be removed.
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
}

test_no_panic_decoding!(IndexerMetadataSchema);
