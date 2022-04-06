// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::*;
use aptos_crypto::HashValue;
use aptos_jellyfish_merkle::node_type::Node;
use aptos_types::state_store::state_key::StateKey;
use proptest::prelude::*;
use schemadb::{schema::fuzzing::assert_encode_decode, test_no_panic_decoding};

proptest! {
    #[test]
    fn test_jellyfish_merkle_node_schema(
        node_key in any::<NodeKey>(),
        key_hash in any::<HashValue>(),
        key in any::<StateKey>(),
        value in any::<StateValue>(),
    ) {
        assert_encode_decode::<JellyfishMerkleNodeSchema>(
            &node_key,
            &Node::new_leaf(key_hash, key, value),
        );
    }
}

test_no_panic_decoding!(JellyfishMerkleNodeSchema);
