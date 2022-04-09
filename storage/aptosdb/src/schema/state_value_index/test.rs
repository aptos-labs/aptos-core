// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::*;
use proptest::prelude::*;
use schemadb::{schema::fuzzing::assert_encode_decode, test_no_panic_decoding};

proptest! {
    #[test]
    fn test_encode_decode(
        state_key in any::<StateKey>(),
        version in any::<Version>(),
        num_nibbles in any::<u8>(),
    ) {
        assert_encode_decode::<StateValueIndexSchema>(&(state_key, version), &num_nibbles);
    }
}

test_no_panic_decoding!(StateValueIndexSchema);
