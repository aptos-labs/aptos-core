// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{assert_success, tests::common, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_language_e2e_tests::{account::Account, executor::FakeExecutor};
use aptos_package_builder::PackageBuilder;
use aptos_types::{
    move_utils::MemberId,
    transaction::{ExecutionStatus, TransactionStatus},
};
use claims::assert_ok;
use move_core_types::{
    account_address::AccountAddress,
    ident_str,
    language_storage::ModuleId,
    vm_status::{
        sub_status::NFE_BCS_SERIALIZATION_FAILURE, AbortLocation,
        StatusCode::EXECUTION_LIMIT_REACHED,
    },
};
use std::str::FromStr;

fn initialize(h: &mut MoveHarness) {
    let build_options = BuildOptions::move_2().set_latest_language();
    let path = common::test_dir_path("bcs.data/function-values");

    let framework_account = h.aptos_framework_account();
    let status = h.publish_package_with_options(&framework_account, path.as_path(), build_options);
    assert_success!(status);
}

#[test]
fn test_function_value_serialization() {
    let mut h = MoveHarness::new_with_executor(FakeExecutor::from_head_genesis());
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x123").unwrap());
    initialize(&mut h);

    let status = h.run_entry_function(
        &acc,
        MemberId::from_str("0x1::bcs_function_values_test::successful_bcs_tests").unwrap(),
        vec![],
        vec![],
    );
    assert_success!(status);

    let expected_failures = [
        "failure_bcs_test_friend_function",
        "failure_bcs_test_friend_function_with_capturing",
        "failure_bcs_test_private_function",
        "failure_bcs_test_private_function_with_capturing",
        "failure_bcs_test_anonymous",
        "failure_bcs_test_anonymous_with_capturing",
    ];

    let bcs_location = AbortLocation::Module(ModuleId::new(
        AccountAddress::ONE,
        ident_str!("bcs").to_owned(),
    ));
    let expected_status = ExecutionStatus::MoveAbort {
        location: bcs_location.clone(),
        code: NFE_BCS_SERIALIZATION_FAILURE,
        info: None,
    };

    for name in expected_failures {
        let status = assert_ok!(h
            .run_entry_function(
                &acc,
                MemberId::from_str(&format!("0x1::bcs_function_values_test::{name}")).unwrap(),
                vec![],
                vec![],
            )
            .as_kept_status());
        assert_eq!(&status, &expected_status);
    }
}

/// Generates the L0-L126 DAG Move source (509 DAG nodes, depth 128).
///
/// L0 has 4 u64 fields. L1 to L126 each reference the previous level four times.
/// Without deduplication, `constant_serialized_size` would visit ~4^128/3 nodes.
/// With the deduplication via caching of same struct nodes, `constant_serialized_size`
/// completes in O(DAG size).
fn constant_size_dag_source() -> String {
    // L0 has 4 u64 fields.
    let mut src = String::from(
        "module 0xcafe::test {\n    use std::bcs;\n\n\
         struct L0 has drop { f0: u64, f1: u64, f2: u64, f3: u64 }\n",
    );
    // L1 to L126 each reference the previous level four times.
    for i in 1..=126 {
        src.push_str(&format!(
            "    struct L{i} has drop {{ f0: L{p}, f1: L{p}, f2: L{p}, f3: L{p} }}\n",
            i = i,
            p = i - 1,
        ));
    }
    src.push_str(
        "    public entry fun run() { let _ = bcs::constant_serialized_size<L126>(); }\n}",
    );
    src
}

#[test]
fn test_constant_serialized_size_dag_no_stall() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    let mut builder = PackageBuilder::new("ConstantSizeDag");
    builder.add_source("test", &constant_size_dag_source());
    builder.add_local_dep(
        "MoveStdlib",
        &common::framework_dir_path("move-stdlib").to_string_lossy(),
    );
    let path = builder.write_to_temp().unwrap();

    assert_success!(h.publish_package_with_options(
        &acc,
        path.path(),
        BuildOptions::move_2().set_latest_language(),
    ));
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::test::run").unwrap(),
        vec![],
        vec![],
    ));
}

/// Generates a module with a `depth`-deep single-field wrapper chain
/// (`D1{f: D2} ... D{depth}{f: u8}`) and an entry `run` that builds a
/// `vector<D1>` of `n_elems` such values and serializes it `iters` times.
///
/// Mirrors the PoC: each wrapper struct is one Value node but serializes to zero
/// BCS bytes, so the node count far exceeds the output byte count. This is the
/// case the per-output-byte cost under-charges.
fn deep_wrapper_source(depth: usize, n_elems: u64, iters: u64) -> String {
    let mut src =
        String::from("module 0xcafe::test {\n    use std::bcs;\n    use std::vector;\n\n");
    for k in 1..depth {
        src.push_str(&format!("    struct D{} has drop {{ f: D{} }}\n", k, k + 1));
    }
    src.push_str(&format!("    struct D{} has drop {{ f: u8 }}\n\n", depth));

    let mut ctor = String::from("0u8");
    for k in (1..=depth).rev() {
        ctor = format!("D{}{{f:{}}}", k, ctor);
    }

    src.push_str("    public entry fun run() {\n");
    src.push_str("        let v = vector::empty<D1>();\n");
    src.push_str("        let i: u64 = 0;\n");
    src.push_str(&format!(
        "        while (i < {n_elems}) {{ vector::push_back(&mut v, {ctor}); i = i + 1; }};\n"
    ));
    src.push_str("        let j: u64 = 0;\n");
    src.push_str(&format!(
        "        while (j < {iters}) {{ let _b = bcs::to_bytes(&v); j = j + 1; }}\n"
    ));
    src.push_str("    }\n}");
    src
}

fn publish_deep_wrapper(
    h: &mut MoveHarness,
    acc: &Account,
    depth: usize,
    n_elems: u64,
    iters: u64,
) {
    let mut builder = PackageBuilder::new("BcsGas");
    builder.add_source("test", &deep_wrapper_source(depth, n_elems, iters));
    builder.add_local_dep(
        "MoveStdlib",
        &common::framework_dir_path("move-stdlib").to_string_lossy(),
    );
    let path = builder.write_to_temp().unwrap();
    assert_success!(h.publish_package_with_options(
        acc,
        path.path(),
        BuildOptions::move_2().set_latest_language(),
    ));
}

fn run_id() -> MemberId {
    MemberId::from_str("0xcafe::test::run").unwrap()
}

/// `bcs::to_bytes` of a node-heavy, byte-light value should cost far more once the
/// value-size metering flag is on. Run with `-- --nocapture` to see the numbers.
///
/// `MoveHarness::new()` starts on the TESTING chain before the flag's activation
/// time, so the first run is metered the old (per-byte) way; `new_epoch` forwards
/// past the activation time to turn the flag on for the second run.
#[test]
fn test_bcs_to_bytes_value_size_metering() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    publish_deep_wrapper(&mut h, &acc, 120, 200, 50);

    let gas_off = h.evaluate_entry_function_gas(&acc, run_id(), vec![], vec![]);

    h.new_epoch();
    let gas_on = h.evaluate_entry_function_gas(&acc, run_id(), vec![], vec![]);

    println!("bcs::to_bytes gas  off={gas_off}  on={gas_on}");
    assert!(
        gas_on > gas_off * 20,
        "value-size metering should dominate: on={gas_on} off={gas_off}"
    );
}

/// With the flag on, repeatedly serializing a node-heavy value exhausts the
/// execution-gas budget that the old per-byte cost sails under -- the liveness
/// attack is now metered. Same value + iteration count, opposite outcomes.
#[test]
fn test_bcs_to_bytes_execution_limit() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    publish_deep_wrapper(&mut h, &acc, 120, 200, 100);

    // Flag off: byte-metered, cheap -> the whole loop succeeds.
    assert_success!(h.run_entry_function(&acc, run_id(), vec![], vec![]));

    // Flag on: node-metered -> the loop blows max_execution_gas.
    h.new_epoch();
    let status = h.run_entry_function(&acc, run_id(), vec![], vec![]);
    assert!(
        matches!(
            status,
            TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some(
                EXECUTION_LIMIT_REACHED
            )))
        ),
        "expected EXECUTION_LIMIT_REACHED, got {status:?}"
    );
}
