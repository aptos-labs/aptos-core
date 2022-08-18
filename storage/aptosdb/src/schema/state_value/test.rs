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
        v in any::<Option<StateValue>>(),
    ) {
        assert_encode_decode::<StateValueSchema>(&(state_key, version), &v);
    }
}

test_no_panic_decoding!(StateValueSchema);
