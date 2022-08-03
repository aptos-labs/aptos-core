// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{account::Account, executor::FakeExecutor};
use aptos_transaction_builder::aptos_stdlib;
use aptos_types::{account_config::aptos_root_address, on_chain_config::Version};
use aptos_vm::AptosVM;

pub fn set_aptos_version(executor: &mut FakeExecutor, version: Version) {
    let account = Account::new_genesis_account(aptos_root_address());
    let txn = account
        .transaction()
        .payload(aptos_stdlib::version_set_version(version.major))
        .sequence_number(0)
        .sign();
    executor.new_block();
    executor.execute_and_apply(txn);

    let new_vm = AptosVM::new(executor.get_state_view());
    assert_eq!(new_vm.internals().version().unwrap(), version);
}
