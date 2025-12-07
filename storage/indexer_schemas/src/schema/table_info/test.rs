// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

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
