// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

#[test]
fn view_success_mock() {
    let (env, buffer) = common::env_with_mock(|ctx| {
        ctx.expect_view()
            .returning(|_, _| Ok(vec![serde_json::json!(42)]));
    });

    let output = common::run_cli_with_env(
        &[
            "view",
            "--function-id",
            "0x1::coin::balance",
            "--type-args",
            "0x1::aptos_coin::AptosCoin",
            "--args",
            "address:0x1",
        ],
        env,
        buffer,
    );
    common::check_baseline(file!(), &output);
}
