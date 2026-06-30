// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;
use std::io::Write;

/// A malformed package-local `Prover.toml` must surface a clear error rather
/// than being silently discarded — the previous `unwrap_or_default()` made
/// verification run with default options against the user's stated config.
#[tokio::test]
async fn move_package_verify_malformed_prover_toml() {
    let pkg = common::make_package("malformed_toml", &[(
        "hello",
        "module 0xCAFE::hello {
    public fun greet(): u64 { 42 }
    spec greet { ensures result == 42; }
}",
    )]);
    let toml_path = pkg.path().join("Prover.toml");
    let mut f = std::fs::File::create(&toml_path).expect("create Prover.toml");
    // Unterminated string is rejected by the toml parser.
    writeln!(f, "[backend]\nboogie_exe = \"unterminated").expect("write toml");

    let dir = pkg.path().to_str().unwrap();
    let client = common::make_client().await;
    let result = common::call_tool(
        &client,
        "move_package_verify",
        serde_json::json!({ "package_path": dir }),
    )
    .await;
    let formatted = common::format_tool_result(&result);
    common::check_baseline(file!(), &formatted);
}
