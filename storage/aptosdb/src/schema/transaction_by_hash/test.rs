// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::*;
use proptest::prelude::*;
use schemadb::{schema::fuzzing::assert_encode_decode, test_no_panic_decoding};

proptest! {
    #[test]
    fn test_encode_decode(
        hash in any::<HashValue>(),
        version in any::<Version>(),
    ) {
        assert_encode_decode::<TransactionByHashSchema>(&hash, &version);
    }
}

test_no_panic_decoding!(TransactionByHashSchema);
