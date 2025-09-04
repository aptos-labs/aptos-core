// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::*;
use velor_schemadb::{schema::fuzzing::assert_encode_decode, test_no_panic_decoding};
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
