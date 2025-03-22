// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::*;
use aptos_schemadb::{schema::fuzzing::assert_encode_decode, test_no_panic_decoding};
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_encode_decode(
        type_tag in any::<TypeTag>(),
        version in any::<Version>(),
        index in any::<u16>(),
    ) {
        assert_encode_decode::<EventByTypeSchema>(&(type_tag, version, index), &());
    }
}

test_no_panic_decoding!(EventByTypeSchema);
