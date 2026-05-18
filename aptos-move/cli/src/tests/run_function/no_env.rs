// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

#[test]
fn run_function_no_env() {
    let output = common::run_cli(&[
        "run",
        "--function-id",
        "0x1::coin::balance",
        "--type-args",
        "0x1::aptos_coin::AptosCoin",
        "--args",
        "address:0x1",
        "--assume-yes",
    ]);
    // With default MoveEnv (no AptosContext), run should fail
    common::check_baseline(file!(), &output);
}
