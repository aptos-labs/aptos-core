// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

#[test]
fn view_no_env() {
    // ViewFunction uses TransactionOptions::rest_client() directly, not MoveEnv.
    // Without a profile or --url, it should fail with a missing-URL error.
    let output = common::run_cli(&[
        "view",
        "--function-id",
        "0x1::coin::balance",
        "--type-args",
        "0x1::aptos_coin::AptosCoin",
        "--args",
        "address:0x1",
    ]);
    common::check_baseline(file!(), &output);
}
