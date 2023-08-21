// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::*;
use aptos_schemadb::{schema::fuzzing::assert_encode_decode, test_no_panic_decoding};
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_encode_decode(
        state_key in any::<StateKey>(),
        version in any::<Version>(),
    ) {
        assert_encode_decode::<StateValueIndexSchema>(&(state_key, version), &());
    }
}

test_no_panic_decoding!(StateValueIndexSchema);
