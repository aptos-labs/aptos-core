// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Copyright © Aptos Foundation

#![allow(unused_imports)]

use crate::{ActiveState, Value};
use aptos_crypto::{
    hash::{CryptoHash, SPARSE_MERKLE_PLACEHOLDER_HASH},
    HashValue,
};
use aptos_experimental_scratchpad::sparse_merkle::{
    test_utils::proof_reader::ProofReader, SparseMerkleTree,
};
use aptos_types::state_store::{
    state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
};
use bitvec::vec;
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
            StateKey::raw(b"00"),
            Some(StateValue::new_legacy(Bytes::from("00"))),
        ),
        (
            StateKey::raw(b"01"),
            Some(StateValue::new_legacy(Bytes::from("01"))),
        ),
        (
            StateKey::raw(b"10"),
            Some(StateValue::new_legacy(Bytes::from("10"))),
        ),
    ];
    updates.sort_by(|(key1, _), (key2, _)| key1.hash().cmp(&key2.hash()));
    let _ = [b"00", b"01", b"10", b"11"]
        .into_iter()
        .map(|e| {
            println!(
                "{:?}, {:?}",
                StateKey::raw(e).hash(),
                StateKey::raw(e).hash().to_vec()
            )
        })
        .collect::<Vec<_>>();

    println!("Test basic input");
    active_state.batch_put_value_set(updates.clone()).unwrap();
    updates = vec![
        (
            StateKey::raw(b"00"),
            Some(StateValue::new_legacy(Bytes::from("a"))),
        ),
        (
            StateKey::raw(b"01"),
            Some(StateValue::new_legacy(Bytes::from("b"))),
        ),
        (
            StateKey::raw(b"10"),
            Some(StateValue::new_legacy(Bytes::from("c"))),
        ),
    ];

    // test add and update
    println!("Test basic update");
    let _ = active_state.batch_put_value_set(updates);
    assert!(active_state.get_oldest_item() == Some(0));
    assert!(active_state.get_latest_item() == Some(2));
    let items = active_state.return_items();
    assert!(items[0].inner.as_ref().unwrap().get_value().get_bytes() == *"a");

    updates = vec![(
        StateKey::raw(b"11"),
        Some(StateValue::new_legacy(Bytes::from("11"))),
    )];
    // test evict and add
    println!("Test evict and add");
    active_state.batch_put_value_set(updates).unwrap();
    assert!(active_state.get_oldest_item() == Some(1));
    assert!(active_state.get_latest_item() == Some(0));
    let items = active_state.return_items();
    assert!(items.len() == 3);
    assert!(items[0].inner.as_ref().unwrap().get_key() == StateKey::raw(b"11"));

    // test delete
    println!("Test delete");
    updates = vec![(StateKey::raw(b"11"), None)];

    active_state.batch_put_value_set(updates).unwrap();
    assert!(active_state.get_oldest_item() == Some(1));
    assert!(active_state.get_latest_item() == Some(2));
    assert!(active_state.get_used_slots_cnt() == 2);

    // test using the empty slot
    updates = vec![(
        StateKey::raw(b"00"),
        Some(StateValue::new_legacy(Bytes::from("00"))),
    )];
    println!("Test reuse the evicted slot");
    active_state.batch_put_value_set(updates).unwrap();
    assert!(active_state.get_oldest_item() == Some(1));
    assert!(active_state.get_latest_item() == Some(0));
    assert!(active_state.get_used_slots_cnt() == 3);
}
