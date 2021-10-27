// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use diem_types::{
    account_address::{AccountAddress, HashAccountAddress},
    account_state_blob::AccountStateBlob,
    transaction::{Transaction, WriteSetPayload},
};
use executor::process_write_set;
use executor_types::ProofReader;
use scratchpad::SparseMerkleTree;
use std::{collections::HashMap, convert::TryFrom};
use vm_genesis::{generate_genesis_change_set_for_testing, GenesisOptions};

// generate genesis state blob
pub fn generate_genesis_state() -> (
    HashMap<AccountAddress, AccountStateBlob>,
    SparseMerkleTree<AccountStateBlob>,
) {
    let change_set = generate_genesis_change_set_for_testing(GenesisOptions::Compiled);
    let txn = Transaction::GenesisTransaction(WriteSetPayload::Direct(change_set.clone()));
    let proof_reader = ProofReader::new(HashMap::new());
    let tree = SparseMerkleTree::default();
    let mut account_states = HashMap::new();

    let states =
        process_write_set(&txn, &mut account_states, change_set.write_set().clone()).unwrap();
    let blobs = states
        .into_iter()
        .map(|(addr, state)| (addr, AccountStateBlob::try_from(&state).unwrap()))
        .collect::<HashMap<_, _>>();
    let new_tree = tree
        .batch_update(
            blobs
                .iter()
                .map(|(addr, value)| (addr.hash(), value))
                .collect(),
            &proof_reader,
        )
        .expect("Failed to update state tree.");
    (blobs, new_tree)
}
