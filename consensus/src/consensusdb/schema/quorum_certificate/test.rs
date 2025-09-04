// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::*;
use velor_consensus_types::block::block_test_utils::certificate_for_genesis;
use velor_schemadb::{schema::fuzzing::assert_encode_decode, test_no_panic_decoding};

#[test]
fn test_encode_decode() {
    let qc = certificate_for_genesis();
    assert_encode_decode::<QCSchema>(&qc.certified_block().id(), &qc);
}

test_no_panic_decoding!(QCSchema);
