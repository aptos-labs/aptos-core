// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::*;
use velor_crypto::HashValue;
use velor_schemadb::{schema::fuzzing::assert_encode_decode, test_no_panic_decoding};
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
