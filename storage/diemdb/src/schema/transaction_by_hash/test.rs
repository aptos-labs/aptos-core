// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use super::*;
use proptest::prelude::*;
use schemadb::schema::assert_encode_decode;

proptest! {
    #[test]
    fn test_encode_decode(
        hash in any::<HashValue>(),
        version in any::<Version>(),
    ) {
        assert_encode_decode::<TransactionByHashSchema>(&hash, &version);
    }
}
