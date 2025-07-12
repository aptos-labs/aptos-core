// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_block_executor::txn_provider::default::DefaultTxnProvider;
use aptos_transaction_simulation::{AccountData, InMemoryStateStore, SimulationStateStore};
use aptos_types::{
    transaction::{signature_verified_transaction::SignatureVerifiedTransaction, Transaction},
    write_set::WriteSet,
};
use aptos_vm::{aptos_vm::AptosVMBlockExecutor, VMBlockExecutor};
use std::io::{self, Read};

fn main() -> Result<()> {
    let mut blob = vec![];
    io::stdin().read_to_end(&mut blob)?;
    let genesis_write_set: WriteSet = bcs::from_bytes(&blob)?;

    println!("Start running");

    let state_store = InMemoryStateStore::new();
    state_store.apply_write_set(&genesis_write_set)?;

    let alice = AccountData::new(100_000_000, Some(0));
    let bob = AccountData::new(100_000_000, Some(0));
    state_store.add_account_data(&alice)?;
    state_store.add_account_data(&bob)?;

    const NUM_TXNS: u64 = 100;

    let txns: Vec<SignatureVerifiedTransaction> = (0..NUM_TXNS)
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
            .into()
        })
        .collect();

    let txn_provider = DefaultTxnProvider::new_without_info(txns);
    let outputs =
        AptosVMBlockExecutor::new().execute_block_no_limit(&txn_provider, &state_store)?;
    for i in 0..NUM_TXNS {
        assert!(outputs[i as usize].status().status().unwrap().is_success());
    }

    Ok(())
}
