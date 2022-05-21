// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::*;
use aptos_crypto::HashValue;
use aptos_jellyfish_merkle::node_type::Node;
use proptest::prelude::*;
use schemadb::{schema::fuzzing::assert_encode_decode, test_no_panic_decoding};

proptest! {
    #[test]
    fn test_jellyfish_merkle_node_schema(
        node_key in any::<NodeKey>(),
        account_key in any::<HashValue>(),
        value_hash in any::<HashValue>(),
        state_key in any::<StateKey>(),
        version in any::<Version>()
    ) {
        assert_encode_decode::<JellyfishMerkleNodeSchema>(
            &node_key,
            &Node::new_leaf(account_key, value_hash, (state_key, version)),
        );
    }
}

test_no_panic_decoding!(JellyfishMerkleNodeSchema);
