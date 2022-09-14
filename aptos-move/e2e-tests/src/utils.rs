// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]
use crate::{account::Account, compile, executor::FakeExecutor};

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
    let executor = FakeExecutor::from_testnet_genesis();
    let mut dr_account = Account::new_aptos_root();

    let (private_key, public_key) = vm_genesis::GENESIS_KEYPAIR.clone();
    dr_account.rotate_key(private_key, public_key);

    (executor, dr_account)
}
