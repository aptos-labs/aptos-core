// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::*;
use proptest::prelude::*;
use schemadb::{schema::fuzzing::assert_encode_decode, test_no_panic_decoding};

proptest! {
    #[test]
    fn test_encode_decode(
        stale_state_value_index in any::<StaleStateValueIndex>(),
    ) {
        assert_encode_decode::<StaleStateValueIndexSchema>(&stale_state_value_index, &());
    }
}

test_no_panic_decoding!(StaleStateValueIndexSchema);
