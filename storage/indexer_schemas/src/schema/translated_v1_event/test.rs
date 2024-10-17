// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

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
