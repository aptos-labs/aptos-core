// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// This file is a copy of the file storage/indexer/src/schema/table_info/test.rs.
/// At the end of the migration to migrate table info mapping
/// from storage critical path to indexer, the other file will be removed.
use super::*;
use aptos_schemadb::{schema::fuzzing::assert_encode_decode, test_no_panic_decoding};
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_encode_decode(
        table_handle in any::<TableHandle>(),
        table_info in any::<TableInfo>(),
    ) {
        assert_encode_decode::<TableInfoSchema>(&table_handle, &table_info);
    }
}

test_no_panic_decoding!(TableInfoSchema);
