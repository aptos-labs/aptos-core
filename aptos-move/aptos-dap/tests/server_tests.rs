// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

mod helpers;

use aptos_dap::server::{variables::frame_locals_ref_id, RunCommand};
use helpers::{build_test_package, DapTestServer, RECV_TIMEOUT};
use std::time::Duration;

#[test]
fn test_per_frame_locals() {
    // language=Move
    let pkg = build_test_package(
        r#"
module 0x42::test {
    fun inner(a: u64, _val: signer): u64 {
        a
      //^
    }

    #[test(acc = @0x1)]
    fun test_it(acc: signer) {
        let _result = inner(42, acc);
    }
}
"#,
    );
    let mode = RunCommand::Test {
        filter: String::new(),
        package_path: pkg.path.clone(),
        skip_fetch_latest_git_deps: true,
    };
    let mut t = DapTestServer::start(mode);
    t.initialize_and_launch_test(&pkg);

    let frames = t.get_stack_frames();
    assert!(
        frames.len() >= 2,
        "expected at least 2 stack frames, got {}: {frames:?}",
        frames.len()
    );

    // scopes for frame 0
    let scopes0 = t.get_frame_scopes(0);
    assert_eq!(
        scopes0["Locals"]["variablesReference"],
        frame_locals_ref_id(0)
    );

    // scopes for frame 1
    let scopes1 = t.get_frame_scopes(1);
    assert_eq!(
        scopes1["Locals"]["variablesReference"],
        frame_locals_ref_id(1)
    );

    // variables for frame 0 (inner)
    let vars0 = t.get_frame_variables(0);
    assert!(vars0.contains_key("_val"), "frame 0 should have '_val'");

    // variables for frame 1 (test_it)
    let vars1 = t.get_frame_variables(1);
    assert!(vars1.contains_key("acc"), "frame 1 should have 'acc'");
}

#[test]
fn test_source_line_breakpoint() {
    // language=Move
    let pkg = build_test_package(
        r#"
module 0x42::test {
    fun add(a: u64, b: u64): u64 { a + b }

    #[test]
    fun test_bp() {
        let a = add(1, 1);
        assert!(a == 2);
        //^
    }
}
"#,
    );
    let mode = RunCommand::Test {
        filter: String::new(),
        package_path: pkg.path.clone(),
        skip_fetch_latest_git_deps: true,
    };
    let mut t = DapTestServer::start(mode);
    t.initialize_and_launch_test(&pkg);

    let frames = t.get_stack_frames();
    let top_frame = &frames[0];
    assert!(
        top_frame["name"].as_str().unwrap().contains("test_bp"),
        "expected top frame to be test_bp, got: {}",
        top_frame["name"]
    );
    assert!(top_frame["source"]["path"]
        .as_str()
        .unwrap()
        .contains("test.move"));
}

#[test]
fn test_stack_frame_names() {
    // language=Move
    let pkg = build_test_package(
        r#"
module 0x42::test {
    fun inner(val: u64): u64 {
        assert!(val > 0);
        //^
        val
    }

    #[test]
    fun test_it() {
        let result = inner(42);
        assert!(result == 42);
    }
}
"#,
    );
    let mode = RunCommand::Test {
        filter: String::new(),
        package_path: pkg.path.clone(),
        skip_fetch_latest_git_deps: true,
    };
    let mut t = DapTestServer::start(mode);
    t.initialize_and_launch_test(&pkg);

    let frames = t.get_stack_frames();
    assert!(
        frames.len() >= 2,
        "expected at least 2 frames, got {}",
        frames.len()
    );

    for frame in &frames {
        let name = frame["name"].as_str().unwrap();
        let parts: Vec<&str> = name.splitn(3, "::").collect();
        assert_eq!(
            parts.len(),
            3,
            "frame name should have 3 parts (address::module::function), got: {name}"
        );
        let func_name = parts[2];
        assert!(
            !func_name.contains("::"),
            "function name should not contain '::': {name}"
        );
        assert!(
            func_name.chars().all(|c| c.is_alphanumeric() || c == '_'),
            "function name should be alphanumeric/underscore only: {name}"
        );
    }

    let names: Vec<&str> = frames.iter().map(|f| f["name"].as_str().unwrap()).collect();
    assert!(names[0].ends_with("::test::inner"));
    assert!(names[1].ends_with("::test::test_it"));
}

#[test]
fn test_snapshot_no_stale() {
    // language=Move
    let pkg = build_test_package(
        r#"
module 0x42::test {
    fun consume_u64(_val: u64) {}

    fun first_helper(x: u64) {
        consume_u64(x);
    }

    fun second_helper(y: u64): u64 {
        consume_u64(y);
        //^
        y
    }

    #[test]
    fun test_it() {
        first_helper(111);
        let _ = second_helper(222);
    }
}
"#,
    );
    let mode = RunCommand::Test {
        filter: String::new(),
        package_path: pkg.path.clone(),
        skip_fetch_latest_git_deps: true,
    };
    let mut t = DapTestServer::start(mode);
    t.initialize_and_launch_test(&pkg);

    let variables = t.get_frame_variables(0);
    assert_eq!(
        variables["y"]["value"].as_str().unwrap(),
        "222",
        "y should be 222 (from second_helper), not stale from first_helper"
    );
}

#[test]
fn test_value_types() {
    // language=Move
    let pkg = build_test_package(
        r#"
module 0x42::test {
    fun consume(
        _a: bool, _b: u8, _c: u64, _d: u128, _e: address, _f: vector<u64>
    ) {}

    fun helper(
        my_bool: bool, my_u8: u8, my_u64: u64,
        my_u128: u128, my_address: address, my_vec: vector<u64>
    ): u64 {
        consume(my_bool, my_u8, my_u64, my_u128, my_address, my_vec);
        //^
        my_u64
    }

    #[test]
    fun test_types() {
        let _ = helper(true, 42, 1000, 99999, @0xCAFE, vector[1u64, 2, 3]);
    }
}
"#,
    );
    let mode = RunCommand::Test {
        filter: String::new(),
        package_path: pkg.path.clone(),
        skip_fetch_latest_git_deps: true,
    };
    let mut t = DapTestServer::start(mode);
    t.initialize_and_launch_test(&pkg);

    let variables = t.get_frame_variables(0);

    assert_eq!(variables["my_bool"]["value"].as_str().unwrap(), "true");
    assert_eq!(variables["my_u8"]["value"].as_str().unwrap(), "42");
    assert_eq!(variables["my_u64"]["value"].as_str().unwrap(), "1000");
    assert_eq!(variables["my_u128"]["value"].as_str().unwrap(), "99999");
    assert!(variables["my_address"]["value"]
        .as_str()
        .unwrap()
        .contains("cafe"));
    assert!(variables["my_vec"]["value"].as_str().unwrap().contains("1"));
}

#[test]
#[ignore] // nested struct field names not yet resolved
fn test_nested_struct_fields() {
    // language=Move
    let pkg = build_test_package(
        r#"
module 0x42::test {
    struct Inner has drop { x: u64, y: bool }
    struct Outer has drop { inner: Inner, tag: u64 }

    fun consume(_o: Outer) {}

    fun helper(o: Outer): u64 {
        consume(o);
        //^
        42
    }

    #[test]
    fun test_it() {
        let o = Outer { inner: Inner { x: 100, y: true }, tag: 7 };
        let _ = helper(o);
    }
}
"#,
    );
    let mode = RunCommand::Test {
        filter: String::new(),
        package_path: pkg.path.clone(),
        skip_fetch_latest_git_deps: true,
    };
    let mut t = DapTestServer::start(mode);
    t.initialize_and_launch_test(&pkg);

    let variables = t.get_frame_variables(0);
    let o_var = &variables["o"];
    assert_eq!(
        o_var["value"].as_str().unwrap(),
        "{ inner: { x: 100, y: true }, tag: 7 }"
    );

    let o_ref = o_var["variablesReference"].as_i64().unwrap();
    assert!(o_ref > 0, "Outer struct should be expandable");

    let outer_fields = t.get_variables_by_reference(o_ref);
    assert_eq!(outer_fields.len(), 2);
    assert_eq!(
        outer_fields["inner"]["value"].as_str().unwrap(),
        "{ x: 100, y: true }"
    );
    assert_eq!(outer_fields["tag"]["value"].as_str().unwrap(), "7");

    let inner_ref = outer_fields["inner"]["variablesReference"]
        .as_i64()
        .unwrap();
    assert!(inner_ref > 0, "Inner struct should be expandable");

    let inner_fields = t.get_variables_by_reference(inner_ref);
    assert_eq!(inner_fields.len(), 2);
    assert_eq!(inner_fields["x"]["value"].as_str().unwrap(), "100");
    assert_eq!(inner_fields["y"]["value"].as_str().unwrap(), "true");
}

#[test]
fn test_no_duplicate_copy_vars() {
    // language=Move
    let pkg = build_test_package(
        r#"
module 0x42::test {
    struct CopyStruct has copy, drop { val: u64 }

    fun consume_copy_struct(_s: CopyStruct) {}

    fun use_three_times(s: CopyStruct): u64 {
        consume_copy_struct(s);
        consume_copy_struct(s);
        consume_copy_struct(s);
        s.val
        //^
    }

    #[test]
    fun test_no_dup() {
        let s = CopyStruct { val: 42 };
        let _ = use_three_times(s);
    }
}
"#,
    );
    let mode = RunCommand::Test {
        filter: String::new(),
        package_path: pkg.path.clone(),
        skip_fetch_latest_git_deps: true,
    };
    let mut t = DapTestServer::start(mode);
    t.initialize_and_launch_test(&pkg);

    let variables = t.get_frame_variables(0);
    assert!(variables.contains_key("s"), "expected variable named 's'");
}

#[test]
fn test_source_bp_no_duplicate_hit() {
    // language=Move
    let pkg = build_test_package(
        r#"
module 0x42::test {
    fun multi_bytecode_line(a: u64, b: u64): u64 {
        let result = a + b;
        //^
        result
    }

    #[test]
    fun test_bp() {
        let x = multi_bytecode_line(10, 20);
        assert!(x == 30);
    }
}
"#,
    );
    let mode = RunCommand::Test {
        filter: String::new(),
        package_path: pkg.path.clone(),
        skip_fetch_latest_git_deps: true,
    };
    let mut t = DapTestServer::start(mode);
    t.initialize_and_launch_test(&pkg);

    t.get_stack_frames();

    // Continue — should run to completion without hitting the breakpoint again
    t.send("continue", Some(serde_json::json!({ "threadId": 1 })));
    t.collect_until_event("terminated", 30);
}

#[test]
fn test_source_bp_no_duplicate_hit_with_function_call() {
    // language=Move
    let pkg = build_test_package(
        r#"
module 0x42::test {
    fun is_valid(): bool { true }

    fun guarded_action(): u64 {
        assert!(is_valid(), 42);
        //^
        100
    }

    #[test]
    fun test_bp() {
        let x = guarded_action();
        assert!(x == 100);
    }
}
"#,
    );
    let mode = RunCommand::Test {
        filter: String::new(),
        package_path: pkg.path.clone(),
        skip_fetch_latest_git_deps: true,
    };
    let mut t = DapTestServer::start(mode);
    t.initialize_and_launch_test(&pkg);

    t.get_stack_frames();

    // Continue — the assert! line has a function call (is_valid()) which
    // temporarily leaves and returns to the same source line. The breakpoint
    // should NOT fire a second time.
    t.send("continue", Some(serde_json::json!({ "threadId": 1 })));
    t.collect_until_event("terminated", 30);
}

#[test]
fn test_source_bp_rehits_in_loop_statement() {
    // language=Move
    let pkg = build_test_package(
        r#"
module 0x42::test {
    fun loop_body(i: u64): u64 {
        i
    }

    #[test]
    fun test_loop() {
        let i = 0;
        while (i < 3) {
            i = loop_body(i);
                  //^
        };
    }
}
"#,
    );
    let mode = RunCommand::Test {
        filter: String::new(),
        package_path: pkg.path.clone(),
        skip_fetch_latest_git_deps: true,
    };
    let mut t = DapTestServer::start(mode);
    t.initialize_and_launch_test(&pkg);

    t.get_stack_frames();

    t.continue_until_breakpoint();
    t.get_stack_frames();

    t.continue_until_breakpoint();
    t.get_stack_frames();
}

#[test]
fn test_source_bp_rehits_in_loop_inner_function() {
    // language=Move
    let pkg = build_test_package(
        r#"
module 0x42::test {
    fun loop_body(i: u64): u64 {
        let next = i + 1;
        //^
        next
    }

    #[test]
    fun test_loop() {
        let i = 0;
        while (i < 3) {
            i = loop_body(i);
        };
    }
}
"#,
    );
    let mode = RunCommand::Test {
        filter: String::new(),
        package_path: pkg.path.clone(),
        skip_fetch_latest_git_deps: true,
    };
    let mut t = DapTestServer::start(mode);
    t.initialize_and_launch_test(&pkg);

    t.get_stack_frames();

    t.continue_until_breakpoint();
    t.get_stack_frames();

    t.continue_until_breakpoint();
    t.get_stack_frames();
}

#[test]
#[ignore] // flaky
fn test_step_over_line() {
    // language=Move
    let pkg = build_test_package(
        r#"
module 0x42::test {
    fun helper(a: u64, b: u64): u64 {
        let sum = a + b;
        //^
        let doubled = sum * 2;
        let result = doubled + 1;
        result
    }

    #[test]
    fun test_it() {
        let _ = helper(10, 20);
    }
}
"#,
    );
    let mode = RunCommand::Test {
        filter: String::new(),
        package_path: pkg.path.clone(),
        skip_fetch_latest_git_deps: true,
    };
    let mut t = DapTestServer::start(mode);
    t.initialize_and_launch_test(&pkg);

    let frames = t.get_stack_frames();
    let line1 = frames[0]["line"].as_i64().unwrap();

    t.step_over();
    let frames = t.get_stack_frames();
    let line2 = frames[0]["line"].as_i64().unwrap();
    assert!(
        line2 > line1,
        "step over should advance to next line: was {line1}, now {line2}"
    );

    t.step_over();
    let frames = t.get_stack_frames();
    let line3 = frames[0]["line"].as_i64().unwrap();
    assert!(
        line3 > line2,
        "second step over should advance again: was {line2}, now {line3}"
    );
}

#[test]
fn test_signer_display() {
    // language=Move
    let pkg = build_test_package(
        r#"
module 0x42::test {
    fun use_signer(_s: &signer): u64 {
        42
        //^
    }

    #[test(acc = @0x1)]
    fun test_it(acc: signer) {
        let _ = use_signer(&acc);
    }
}
"#,
    );
    let mode = RunCommand::Test {
        filter: String::new(),
        package_path: pkg.path.clone(),
        skip_fetch_latest_git_deps: true,
    };
    let mut t = DapTestServer::start(mode);
    t.initialize_and_launch_test(&pkg);

    let variables = t.get_frame_variables(0);
    assert_eq!(
        variables["_s"]["value"].as_str().unwrap(),
        "(&) signer(0x1)"
    );
}

#[test]
fn test_signer_display_after_move() {
    // language=Move
    let pkg = build_test_package(
        r#"
module 0x42::test {
    fun consume_signer(_s: signer) {}

    fun helper(s: signer): u64 {
        consume_signer(s);
        42
        //^
    }

    #[test(acc = @0x1)]
    fun test_it(acc: signer) {
        let _ = helper(acc);
    }
}
"#,
    );
    let mode = RunCommand::Test {
        filter: String::new(),
        package_path: pkg.path.clone(),
        skip_fetch_latest_git_deps: true,
    };
    let mut t = DapTestServer::start(mode);
    t.initialize_and_launch_test(&pkg);

    let variables = t.get_frame_variables(0);
    assert_eq!(variables["s"]["value"].as_str().unwrap(), "signer(0x1)");
}

#[test]
#[ignore] // requires network access to mainnet
fn test_replay_basic() {
    let mode = RunCommand::Replay {
        txn_id: 4969730041,
        network: "mainnet".to_string(),
        local_packages: vec![],
        prebuilt_packages: vec![],
        named_addresses: std::collections::BTreeMap::new(),
        skip_fetch_latest_git_deps: true,
    };
    let mut t = DapTestServer::start(mode);
    t.initialize_and_launch_replay(&[], Duration::from_secs(120));

    let long_timeout = Duration::from_secs(120);
    let mut stop_count = 0;
    loop {
        let frames = t.get_stack_frames();
        assert!(!frames.is_empty(), "expected at least one stack frame");
        stop_count += 1;

        if !t.continue_execution_timeout(long_timeout) {
            break;
        }
    }
    assert!(stop_count >= 1, "should have stopped at least once");
}

#[test]
#[ignore] // requires network access to testnet
fn test_replay_with_prebuilt_local_package() {
    let stdlib_build = "/home/mkurnikov/.move/https___github_com_aptos-labs_aptos-framework_git_mainnet/move-stdlib/build/MoveStdlib";
    let framework_build = "/home/mkurnikov/.move/https___github_com_aptos-labs_aptos-framework_git_mainnet/aptos-framework/build/AptosFramework";
    let accounts_build = "/home/mkurnikov/code/etna/move/accounts/build/decibel_accounts";
    let perp_build = "/home/mkurnikov/code/etna/move/perp/build/decibel_perp_dex";
    let market_build = "/home/mkurnikov/code/etna/move/aptos_market/build/aptos_market";

    let bp = std::path::Path::new(perp_build)
        .join("sources/perp_engine.move")
        .canonicalize()
        .expect("perp_engine.move not found");
    let breakpoints = [format!("{}:1440", bp.to_string_lossy())];
    let bp_refs: Vec<&str> = breakpoints.iter().map(|s| s.as_str()).collect();

    let mode = RunCommand::Replay {
        txn_id: 8571567156,
        network: "testnet".to_string(),
        local_packages: vec![],
        prebuilt_packages: vec![
            stdlib_build.into(),
            framework_build.into(),
            accounts_build.into(),
            perp_build.into(),
            market_build.into(),
        ],
        named_addresses: std::collections::BTreeMap::new(),
        skip_fetch_latest_git_deps: true,
    };
    let mut t = DapTestServer::start(mode);
    t.initialize_and_launch_replay(&bp_refs, Duration::from_secs(10));

    let frames = t.get_stack_frames();
    assert!(!frames.is_empty(), "expected at least one stack frame");

    // Fetch Transaction Info
    let scopes = t.get_frame_scopes(0);
    let txn_ref = scopes["Transaction Info"]["variablesReference"]
        .as_i64()
        .unwrap();
    let _txn_vars = t.get_variables_by_reference(txn_ref);

    let _variables = t.get_frame_variables(0);
}

#[test]
fn test_warns_unreachable_breakpoint() {
    // language=Move
    let pkg = build_test_package(
        r#"
module 0x42::test {
    #[test]
    fun test_it() {
        let _ = 1 + 2;
        //^
    }
}
"#,
    );
    let mode = RunCommand::Test {
        filter: String::new(),
        package_path: pkg.path.clone(),
        skip_fetch_latest_git_deps: true,
    };
    let mut t = DapTestServer::start(mode);

    t.initialize();
    t.launch();

    // Set valid breakpoints from the package + one on a nonexistent file
    let mut bps: Vec<&str> = pkg.breakpoints.iter().map(|s| s.as_str()).collect();
    bps.push("/nonexistent/fake_module.move:10");
    t.set_breakpoints(&bps);

    t.send("configurationDone", None);
    // The warning is emitted before execution starts; collect outputs until
    // either a breakpoint stop or termination.
    let mut outputs = Vec::new();
    for _ in 0..30 {
        let m = t.collect_until_event_any(&["output", "stopped", "terminated"], RECV_TIMEOUT);
        if m["event"] == "output" {
            if let Some(text) = m["body"]["output"].as_str() {
                outputs.push(text.to_string());
            }
        }
        if m["event"] == "stopped" || m["event"] == "terminated" {
            break;
        }
    }

    let warning = outputs.iter().find(|o| o.contains("unreachable"));
    assert!(
        warning.is_some(),
        "expected a warning about unresolvable breakpoint, got outputs: {outputs:?}",
    );
    let warning = warning.unwrap();
    assert!(
        warning.contains("/nonexistent/fake_module.move"),
        "warning should mention the unresolvable file, got: {warning}",
    );
}
