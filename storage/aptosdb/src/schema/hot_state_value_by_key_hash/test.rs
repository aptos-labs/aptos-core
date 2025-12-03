// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Innovation-Enabling Source Code License

use super::*;
use aptos_crypto::HashValue;
use aptos_schemadb::{schema::fuzzing::assert_encode_decode, test_no_panic_decoding};
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_encode_decode(
        state_key in any::<HashValue>(),
        version in any::<Version>(),
        v in any::<Option<HotStateValue>>(),
    ) {
        assert_encode_decode::<HotStateValueByKeyHashSchema>(&(state_key, version), &v);
    }
}

test_no_panic_decoding!(HotStateValueByKeyHashSchema);
