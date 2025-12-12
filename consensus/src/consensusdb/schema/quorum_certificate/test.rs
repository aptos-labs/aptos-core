// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::*;
use aptos_consensus_types::block::block_test_utils::certificate_for_genesis;
use aptos_schemadb::{schema::fuzzing::assert_encode_decode, test_no_panic_decoding};

#[test]
fn test_encode_decode() {
    let qc = certificate_for_genesis();
    assert_encode_decode::<QCSchema>(&qc.certified_block().id(), &qc);
}

test_no_panic_decoding!(QCSchema);
