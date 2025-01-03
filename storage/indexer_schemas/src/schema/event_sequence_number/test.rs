// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::*;
use aptos_schemadb::{schema::fuzzing::assert_encode_decode, test_no_panic_decoding};
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_encode_decode(
        event_key in any::<EventKey>(),
        seq_num in any::<u64>(),
    ) {
        assert_encode_decode::<EventSequenceNumberSchema>(&event_key, &seq_num);
    }
}

test_no_panic_decoding!(EventSequenceNumberSchema);
