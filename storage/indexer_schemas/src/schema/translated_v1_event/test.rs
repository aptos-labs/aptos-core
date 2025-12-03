// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Innovation-Enabling Source Code License

use super::*;
use aptos_schemadb::{schema::fuzzing::assert_encode_decode, test_no_panic_decoding};
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_encode_decode(
        version in any::<Version>(),
        index in any::<u64>(),
        event in any::<ContractEventV1>(),
    ) {
        assert_encode_decode::<TranslatedV1EventSchema>(&(version, index), &event);
    }
}

test_no_panic_decoding!(TranslatedV1EventSchema);
