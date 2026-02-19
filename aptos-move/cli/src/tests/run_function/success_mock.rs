// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;
use aptos_cli_common::TransactionSummary;

fn fake_summary() -> TransactionSummary {
    TransactionSummary {
        transaction_hash: aptos_crypto::HashValue::zero().into(),
        gas_used: None,
        gas_unit_price: None,
        pending: None,
        sender: None,
        sequence_number: None,
        replay_protector: None,
        success: Some(true),
        timestamp_us: None,
        version: None,
        vm_status: None,
        deployed_object_address: None,
    }
}

#[test]
fn run_function_success_mock() {
    let env = common::env_with_mock(|ctx| {
        ctx.expect_submit_transaction()
            .returning(|_, _| Ok(fake_summary()));
    });

    let output = common::run_cli_with_env(
        &[
            "run",
            "--function-id",
            "0x1::coin::balance",
            "--type-args",
            "0x1::aptos_coin::AptosCoin",
            "--args",
            "address:0x1",
            "--assume-yes",
        ],
        env,
    );
    common::check_baseline(file!(), &output);
}
