// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::*;
use aptos_schemadb::{schema::fuzzing::assert_encode_decode, test_no_panic_decoding};
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_encode_decode(
        stale_node_index in any::<StaleNodeIndex>(),
    ) {
        assert_encode_decode::<StaleNodeIndexCrossEpochSchema>(&stale_node_index, &());
    }
}

test_no_panic_decoding!(StaleNodeIndexCrossEpochSchema);
