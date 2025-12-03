// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Innovation-Enabling Source Code License

use super::*;
use aptos_schemadb::{schema::fuzzing::assert_encode_decode, test_no_panic_decoding};

#[test]
fn test_encode_decode() {
    let block = Block::make_genesis_block();
    assert_encode_decode::<BlockSchema>(&block.id(), &block);
}

test_no_panic_decoding!(BlockSchema);
