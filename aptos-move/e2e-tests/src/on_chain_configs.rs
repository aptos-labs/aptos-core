// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{account::Account, executor::FakeExecutor};
use aptos_types::{account_config::CORE_CODE_ADDRESS, on_chain_config::Version};
use aptos_vm::AptosVM;
use cached_packages::aptos_stdlib;

pub fn set_aptos_version(executor: &mut FakeExecutor, version: Version) {
    let account = Account::new_genesis_account(CORE_CODE_ADDRESS);
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
