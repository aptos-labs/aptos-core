// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use super::*;
use proptest::prelude::*;
use schemadb::{schema::fuzzing::assert_encode_decode, test_no_panic_decoding};

proptest! {
    #[test]
    fn test_encode_decode(pos in any::<u64>(), hash in any::<HashValue>()) {
        assert_encode_decode::<TransactionAccumulatorSchema>(
            &Position::from_inorder_index(pos),
            &hash,
        );
    }
}

test_no_panic_decoding!(TransactionAccumulatorSchema);
