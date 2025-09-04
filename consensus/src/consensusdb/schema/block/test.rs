// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::*;
use velor_schemadb::{schema::fuzzing::assert_encode_decode, test_no_panic_decoding};

#[test]
fn test_encode_decode() {
    let block = Block::make_genesis_block();
    assert_encode_decode::<BlockSchema>(&block.id(), &block);
}

test_no_panic_decoding!(BlockSchema);
