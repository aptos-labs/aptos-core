// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::*;
use aptos_schemadb::{schema::fuzzing::assert_encode_decode, test_no_panic_decoding};
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_encode_decode(version in any::<u64>(), hash in any::<HashValue>()) {
        assert_encode_decode::<TransactionAccumulatorRootHashSchema>(
            &version,
            &hash,
        );
    }
}

test_no_panic_decoding!(TransactionAccumulatorRootHashSchema);
