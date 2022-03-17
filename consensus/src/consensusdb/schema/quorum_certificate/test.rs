// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::*;
use consensus_types::block::block_test_utils::certificate_for_genesis;
use schemadb::{schema::fuzzing::assert_encode_decode, test_no_panic_decoding};

#[test]
fn test_encode_decode() {
    let qc = certificate_for_genesis();
    assert_encode_decode::<QCSchema>(&qc.certified_block().id(), &qc);
}

test_no_panic_decoding!(QCSchema);
