// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Regression tests for the deny-by-default sanitization applied to a
//! package-local `Prover.toml`. The MCP wrapper treats the package as
//! attacker-controlled, so any field that could redirect execution, panic
//! the request path, or exhaust host resources must either be clamped or
//! reset.

use crate::tests::common;
use std::io::Write;

/// `move_sources = ["/"]` would propagate into the prover, where
/// `Path::new(s).file_name().unwrap()` panics on a root path with no file
/// component. The sanitizer drops package-supplied `move_sources` so the
/// MCP-injected list is used instead.
#[tokio::test]
async fn move_package_verify_prover_toml_move_sources_does_not_panic() {
    let pkg = common::make_package("toml_move_sources", &[(
        "hello",
        "module 0xCAFE::hello {
    public fun greet(): u64 { 42 }
    spec greet { ensures result == 42; }
}",
    )]);
    let mut f = std::fs::File::create(pkg.path().join("Prover.toml")).expect("create toml");
    writeln!(f, "move_sources = [\"/\"]").expect("write toml");

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

/// A package-supplied `shards = 1024` (or any fan-out knob set arbitrarily
/// high) must be capped — `tokio::time::timeout` does not interrupt shard
/// loops or worker subprocesses once verification is running. Verification
/// should still complete normally with the clamped value.
#[tokio::test]
async fn move_package_verify_prover_toml_fanout_clamped() {
    let pkg = common::make_package("toml_fanout", &[(
        "hello",
        "module 0xCAFE::hello {
    public fun greet(): u64 { 42 }
    spec greet { ensures result == 42; }
}",
    )]);
    let mut f = std::fs::File::create(pkg.path().join("Prover.toml")).expect("create toml");
    // Each of these exceeds the sanitizer's cap; if the cap regressed,
    // the prover would honor the package value and exhaust resources.
    writeln!(
        f,
        "[backend]\nshards = 1024\nnum_instances = 1024\nproc_cores = 1024\n[prover]\nnum_instances = 1024"
    )
    .expect("write toml");

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
