// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]
use crate::{account::Account, compile, executor::FakeExecutor};
use aptos_transaction_builder::aptos_stdlib;

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
    for (bytes, module) in framework::head_release_bundle().code_and_compiled_modules() {
        executor.add_module(&module.self_id(), bytes.to_vec());
    }

    if let Some(version_number) = update_version_number {
        executor.execute_and_apply(
            dr_account
                .transaction()
                .payload(aptos_stdlib::version_set_version(version_number))
                .sequence_number(*dr_seqno)
                .sign(),
        );
        *dr_seqno = dr_seqno.checked_add(1).unwrap();
    }
}
