// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::*;
use aptos_consensus_types::block::block_test_utils::certificate_for_genesis;
use aptos_schemadb::{schema::fuzzing::assert_encode_decode, test_no_panic_decoding};

// Tests that the DB can encode / decode data
#[test]
fn test_encode_decode() {
    let wli = certificate_for_genesis().into_wrapped_ledger_info();
    assert_encode_decode::<WLISchema>(&wli.ledger_info().ledger_info().consensus_block_id(), &wli);
}

test_no_panic_decoding!(WLISchema);
