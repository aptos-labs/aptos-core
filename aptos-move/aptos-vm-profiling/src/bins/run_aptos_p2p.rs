// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_language_e2e_tests::{account::AccountData, data_store::FakeDataStore};
use aptos_types::{transaction::Transaction, write_set::WriteSet};
use aptos_vm::{AptosVM, VMExecutor};
use std::{
    collections::HashMap,
    io::{self, Read},
};

fn main() -> Result<()> {
    let mut blob = vec![];
    io::stdin().read_to_end(&mut blob)?;
    let genesis_write_set: WriteSet = bcs::from_bytes(&blob)?;

    println!("Start running");

    let mut state_store = FakeDataStore::new(HashMap::new());
    state_store.add_write_set(&genesis_write_set);

    let alice = AccountData::new(100_000_000, 0);
    let bob = AccountData::new(100_000_000, 0);
    state_store.add_account_data(&alice);
    state_store.add_account_data(&bob);

    const NUM_TXNS: u64 = 100;

    let txns = (0..NUM_TXNS)
        .map(|seq_num| {
            Transaction::UserTransaction(
                alice
                    .account()
                    .transaction()
                    .gas_unit_price(100)
                    .payload(aptos_cached_packages::aptos_stdlib::aptos_coin_transfer(
                        *bob.account().address(),
                        1000,
                    ))
                    .sequence_number(seq_num)
                    .sign(),
            )
        })
        .collect();

    let res = AptosVM::execute_block(txns, &state_store, None)?;
    for i in 0..NUM_TXNS {
        assert!(res[i as usize].status().status().unwrap().is_success());
    }

    Ok(())
}
