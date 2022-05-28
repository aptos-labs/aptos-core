// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::*;
use proptest::prelude::*;
use schemadb::{schema::fuzzing::assert_encode_decode, test_no_panic_decoding};

proptest! {
    #[test]
    fn test_encode_decode(
        state_key in any::<StateKey>(),
        v in any::<StateValue>(),
    ) {
        assert_encode_decode::<LatestStateValueSchema>(&(state_key), &v);
    }
}

test_no_panic_decoding!(LatestStateValueSchema);
