// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]
use crate::{account::Account, compile, executor::FakeExecutor};
use aptos_transaction_builder::aptos_stdlib;
use move_deps::move_binary_format::file_format::CompiledModule;

pub fn close_module_publishing(
    executor: &mut FakeExecutor,
    dr_account: &Account,
    dr_seqno: &mut u64,
) {
    let compiled_script = {
        let script = "
            import 0x1.TransactionPublishingOption;
        main(config: signer) {
        label b0:
            TransactionPublishingOption.set_open_module(&config, false);
            return;
        }
        ";
        compile::compile_script(script, vec![])
    };

    let txn = dr_account
        .transaction()
        .script(compiled_script)
        .sequence_number(*dr_seqno)
        .sign();

    executor.execute_and_apply(txn);
    *dr_seqno = dr_seqno.checked_add(1).unwrap();
}

pub fn start_with_released_df() -> (FakeExecutor, Account) {
    let executor = FakeExecutor::from_fresh_genesis();
    let mut dr_account = Account::new_aptos_root();

    let (private_key, public_key) = vm_genesis::GENESIS_KEYPAIR.clone();
    dr_account.rotate_key(private_key, public_key);

    (executor, dr_account)
}

pub fn upgrade_df(
    executor: &mut FakeExecutor,
    dr_account: &Account,
    dr_seqno: &mut u64,
    update_version_number: Option<u64>,
) {
    close_module_publishing(executor, dr_account, dr_seqno);
    for compiled_module_bytes in cached_framework_packages::module_blobs().iter().cloned() {
        let compiled_module_id = CompiledModule::deserialize(&compiled_module_bytes)
            .unwrap()
            .self_id();
        executor.add_module(&compiled_module_id, compiled_module_bytes);
    }

    if let Some(version_number) = update_version_number {
        executor.execute_and_apply(
            dr_account
                .transaction()
                .payload(aptos_stdlib::encode_version_set_version(version_number))
                .sequence_number(*dr_seqno)
                .sign(),
        );
        *dr_seqno = dr_seqno.checked_add(1).unwrap();
    }
}
