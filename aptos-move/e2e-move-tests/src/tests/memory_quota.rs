// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{assert_success, tests::common, MoveHarness};
use aptos_types::{
    account_address::AccountAddress,
    transaction::{ExecutionStatus, TransactionStatus},
};
use move_core_types::{value::MoveValue, vm_status::StatusCode};

// TODO(Gas): This test has been disabled since the particularly attack it uses can no longer
//            be carried out due to the increase in execution costs.
//            Revisit and decide whether we should remove this test or rewrite it in another way.
/*
#[test]
fn push_u128s_onto_vector() {
    let mut h = MoveHarness::new();

    // Load the code
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap());
    assert_success!(h.publish_package(
        &acc,
        &common::test_dir_path("memory_quota.data/vec_push_u128"),
    ));

    let result = h.run_entry_function(
        &acc,
        str::parse("0xbeef::test::just_under_quota").unwrap(),
        vec![],
        vec![],
    );
    assert_success!(result);

    let result = h.run_entry_function(
        &acc,
        str::parse("0xbeef::test::just_above_quota").unwrap(),
        vec![],
        vec![],
    );
    assert!(matches!(
        result,
        TransactionStatus::Keep(ExecutionStatus::ExecutionFailure { .. })
    ));
}
*/

#[test]
fn deeply_nested_structs() {
    let mut h = MoveHarness::new();

    h.modify_gas_schedule(|gas_params| {
        gas_params.vm.txn.memory_quota = 10_000_000.into();
        gas_params.vm.txn.max_execution_gas = 1_000_000_000_000.into();
    });

    // Publish the code
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap());
    assert_success!(h.publish_package(
        &acc,
        &common::test_dir_path("memory_quota.data/nested_struct"),
    ));

    // Initialize
    let result = h.run_entry_function(
        &acc,
        str::parse("0xbeef::very_nested_structure::init").unwrap(),
        vec![],
        vec![],
    );
    assert_success!(result);

    // Create nested structs as table entries
    for _i in 0..5 {
        let result = h.run_entry_function(
            &acc,
            str::parse("0xbeef::very_nested_structure::add").unwrap(),
            vec![],
            vec![MoveValue::U64(2000).simple_serialize().unwrap()],
        );
        assert_success!(result);
    }

    // Try to load the whole table -- this should succeed
    let result = h.run_entry_function(
        &acc,
        str::parse("0xbeef::very_nested_structure::read_all").unwrap(),
        vec![],
        vec![],
    );
    assert_success!(result);

    // Forward 2 hours to activate TimedFeatureFlag::FixMemoryUsageTracking
    // Now attempting to load the whole table shall result in an execution failure (memory limit hit)
    h.new_epoch();
    let result = h.run_entry_function(
        &acc,
        str::parse("0xbeef::very_nested_structure::read_all").unwrap(),
        vec![],
        vec![],
    );
    assert!(matches!(
        result,
        TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some(
            StatusCode::MEMORY_LIMIT_EXCEEDED
        )))
    ));
}

#[test]
fn clone_large_vectors() {
    let mut h = MoveHarness::new();

    // Load the code
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap());
    assert_success!(h.publish_package(&acc, &common::test_dir_path("memory_quota.data/clone_vec"),));

    let result = h.run_entry_function(
        &acc,
        str::parse("0xbeef::test::just_under_quota").unwrap(),
        vec![],
        vec![],
    );
    assert_success!(result);

    let result = h.run_entry_function(
        &acc,
        str::parse("0xbeef::test::just_above_quota").unwrap(),
        vec![],
        vec![],
    );
    assert!(matches!(
        result,
        TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some(
            StatusCode::MEMORY_LIMIT_EXCEEDED
        )))
    ));
}

#[test]
fn add_vec_to_table() {
    let mut h = MoveHarness::new();

    // Load the code
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap());
    assert_success!(h.publish_package(
        &acc,
        &common::test_dir_path("memory_quota.data/table_and_vec"),
    ));

    let result = h.run_entry_function(
        &acc,
        str::parse("0xbeef::test::just_under_quota").unwrap(),
        vec![],
        vec![],
    );
    // Should fail when trying to destroy a non-empty table.
    assert!(matches!(
        result,
        TransactionStatus::Keep(ExecutionStatus::MoveAbort { .. })
    ));

    let result = h.run_entry_function(
        &acc,
        str::parse("0xbeef::test::just_above_quota").unwrap(),
        vec![],
        vec![],
    );
    // Should run out of memory before trying to destroy a non-empty table.
    assert!(matches!(
        result,
        TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some(
            StatusCode::MEMORY_LIMIT_EXCEEDED
        )))
    ));
}

use crate::MoveHarnessSend;
use aptos_package_builder::PackageBuilder;

/// Generates Move source for a module with `count` nested structs.
/// If `prev_module` is provided, S0 wraps the deepest struct from that module.
/// Returns (module_name, source_code).
fn gen_nested_module(
    addr: &str,
    chunk_idx: usize,
    count: usize,
    prev_module: Option<(&str, usize)>,
) -> (String, String) {
    let module_name = format!("chunk_{}", chunk_idx);
    let mut lines = vec![format!("module {}::{} {{", addr, module_name)];

    // S0: either wraps previous module's deepest struct, or has a u8 field
    if let Some((prev_mod, prev_count)) = prev_module {
        lines.push(format!("    use {}::{};", addr, prev_mod));
        lines.push(format!(
            "    struct S0 has drop {{ i: {}::S{} }}",
            prev_mod,
            prev_count - 1
        ));
    } else {
        lines.push("    struct S0 has drop { val: u8 }".to_string());
    }

    // S1..S(count-1): each wraps the previous
    for i in 1..count {
        lines.push(format!("    struct S{} has drop {{ i: S{} }}", i, i - 1));
    }

    // build() function: constructs the full nesting chain
    lines.push(String::new());
    lines.push(format!("    public fun build(): S{} {{", count - 1));
    if let Some(prev) = prev_module {
        lines.push(format!("        let s0 = S0 {{ i: {}::build() }};", prev.0));
    } else {
        lines.push("        let s0 = S0 { val: 0 };".to_string());
    }
    for i in 1..count {
        lines.push(format!("        let s{} = S{} {{ i: s{} }};", i, i, i - 1));
    }
    lines.push(format!("        s{}", count - 1));
    lines.push("    }".to_string());

    lines.push("}".to_string());
    (module_name, lines.join("\n"))
}

// NOTE:
// Must use `--release` to mimick mainnet `Drop` recursion cost.
// If --release is not used, the default debug uses ~500+ bytes
// per Drop which is optimistically unrealistic. With release,
// it uses Drop cost identical to mainnet (~64 bytes)

// RUN: cargo test -p e2e-move-tests test_poc_nested_struct_drop_overflow_thread_diff --release -- --nocapture
#[test]
fn test_poc_nested_struct_drop_overflow_thread_diff() {
    // Run everything on an 8MB thread so compilation succeeds,
    // then spawn a 2MB thread for execution only.
    let handle = std::thread::Builder::new()
        .name("compile-thread".into())
        .stack_size(8 * 1024 * 1024) // 8MB for compilation
        .spawn(|| {
            // === Step 1: Setup default harness ===
            let mut h = MoveHarnessSend::new();
            let addr = "0xbeef";
            let acc = h.new_account_at(AccountAddress::from_hex_literal(addr).unwrap());

            // Original bug-bounty PoC used 40 modules x 1000 structs = 40,000 depth
            // against the validator's 2MB rayon thread (40k x ~64 B ≈ 2.5MB → overflow).
            // Here we downsize BOTH to 1/4 to keep the test fast while preserving the
            // overflow property:
            //   - 10 modules x 1000 structs = 10,000 depth
            //   - 512KB execution thread (see Step 4 below)
            //   - 10k x ~64 B ≈ 640KB → overflows 512KB, same as mainnet shape.
            let num_modules = 10;
            let structs_per_module = 1000;

            let base_dir = std::env::temp_dir()
                .join(format!("nested_drop_thread_diff_{}", std::process::id()));
            std::fs::create_dir_all(&base_dir).unwrap();
            let mut prev: Option<(String, usize, std::path::PathBuf)> = None;

            // === Step 2: Publish each chunk module in a separate tx ===
            // If not in separate txs, this will hit max tx size limit
            for chunk in 0..num_modules {
                let pkg_name = format!("chunk_{}", chunk);
                let mut builder = PackageBuilder::new(&pkg_name);
                builder.add_alias("attacker", addr);

                if let Some((_, _, ref prev_path)) = prev {
                    builder.add_local_dep(
                        &format!("chunk_{}", chunk - 1),
                        prev_path.to_str().unwrap(),
                    );
                }

                let prev_ref = prev
                    .as_ref()
                    .map(|(name, count, _)| (name.as_str(), *count));
                let (mod_name, source) =
                    gen_nested_module("attacker", chunk, structs_per_module, prev_ref);
                builder.add_source(&format!("chunk_{}", chunk), &source);

                let pkg_path = base_dir.join(&pkg_name);
                builder.write_to_disk(&pkg_path).unwrap();

                let result = h.publish_package(&acc, &pkg_path);
                assert_success!(result);
                println!("Published chunk_{} ({} structs)", chunk, structs_per_module);

                prev = Some((mod_name, structs_per_module, pkg_path));
            }

            // === Step 3: Publish entry module ===
            let (ref last_module, _, ref last_path) = *prev.as_ref().unwrap();
            let mut entry_builder = PackageBuilder::new("attack");
            entry_builder.add_alias("attacker", addr);
            entry_builder.add_local_dep(
                &format!("chunk_{}", num_modules - 1),
                last_path.to_str().unwrap(),
            );
            let entry_source = format!(
                "module attacker::attack {{\n    \
                     use attacker::{};\n    \
                     public entry fun run() {{\n        \
                         let _v = {}::build();\n    \
                     }}\n\
                 }}",
                last_module, last_module
            );
            entry_builder.add_source("attack", &entry_source);
            let entry_path = base_dir.join("attack");
            entry_builder.write_to_disk(&entry_path).unwrap();

            let result = h.publish_package(&acc, &entry_path);
            assert_success!(result);

            let total_depth = num_modules * structs_per_module;
            println!(
                "All modules published. Total depth: {} ({} modules x {} structs)",
                total_depth, num_modules, structs_per_module
            );

            // === Step 4: Execute on a 512KB thread (1/4 of validator's 2MB rayon
            // default — downsized together with `num_modules` above to keep the test
            // fast while preserving the overflow shape from mainnet). ===
            const EXEC_STACK_BYTES: usize = 512 * 1024;
            println!("Spawning {}-byte execution thread...", EXEC_STACK_BYTES);
            let exec_handle = std::thread::Builder::new()
                .name("exec-downsized".into())
                .stack_size(EXEC_STACK_BYTES)
                .spawn(move || {
                    println!("Executing attack on downsized thread...");
                    h.run_entry_function(
                        &acc,
                        str::parse(&format!("{}::attack::run", addr)).unwrap(),
                        vec![],
                        vec![],
                    )
                })
                .unwrap();

            // The fix must produce a clean VM error instead of an uncatchable SIGABRT.
            // `Err` on join means the thread panicked (stack overflow on platforms
            // where that surfaces as a thread panic), which would be a regression.
            let status = exec_handle
                .join()
                .expect("execution thread panicked — possible stack overflow regression");
            assert!(
                matches!(
                    status,
                    TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some(
                        StatusCode::VM_MAX_VALUE_DEPTH_REACHED
                    )))
                ),
                "expected VM_MAX_VALUE_DEPTH_REACHED, got: {:?}",
                status
            );
        })
        .unwrap();

    handle.join().unwrap();
}
