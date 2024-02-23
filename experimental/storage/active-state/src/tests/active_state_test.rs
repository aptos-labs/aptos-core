// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Copyright © Aptos Foundation

#![allow(unused_imports)]

use crate::ActiveState;
use aptos_crypto::{
    hash::{CryptoHash, SPARSE_MERKLE_PLACEHOLDER_HASH},
    HashValue,
};
use aptos_scratchpad::{test_utils::proof_reader::ProofReader, SparseMerkleTree};
use aptos_types::state_store::{
    state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
};
use bytes::Bytes;
use std::sync::Arc;

// Unit test for active state operations
#[test]
fn test_active_state_lru() {
    let smt = SparseMerkleTree::<StateValue>::new(
        *SPARSE_MERKLE_PLACEHOLDER_HASH,
        StateStorageUsage::new_untracked(),
    );

    let mut active_state = ActiveState::new(smt, 3);
    let mut updates = vec![
        (
            StateKey::raw(b"00".to_vec()),
            Some(StateValue::new_legacy(Bytes::from("00"))),
        ),
        (
            StateKey::raw(b"01".to_vec()),
            Some(StateValue::new_legacy(Bytes::from("01"))),
        ),
        (
            StateKey::raw(b"10".to_vec()),
            Some(StateValue::new_legacy(Bytes::from("10"))),
        ),
        /*(
            StateKey::raw(b"11".to_vec()),
            StateValue::new_legacy(Bytes::from("11")),
        ),*/
    ];
    updates.sort_by(|(key1, _), (key2, _)| key1.hash().cmp(&key2.hash()));
    for x in updates
        .iter()
        .map(|(k, _)| k.hash().to_vec())
        .collect::<Vec<_>>()
    {
        println!("{:?}", x);
    }
    let _ = active_state.batch_put_value_set(updates.clone());
    // update the same tree again
    let _ = active_state.batch_put_value_set(updates);
    println!("{:?}", active_state.return_items());
}
