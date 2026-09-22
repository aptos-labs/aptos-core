// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{assert_success, tests::common, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_language_e2e_tests::{account::Account, executor::FakeExecutor};
use aptos_package_builder::PackageBuilder;
use aptos_types::{
    move_utils::MemberId,
    on_chain_config::TimedFeatureFlag,
    transaction::{ExecutionStatus, TransactionStatus},
};
use claims::assert_ok;
use move_core_types::{
    account_address::AccountAddress,
    ident_str,
    language_storage::ModuleId,
    vm_status::{sub_status::NFE_BCS_SERIALIZATION_FAILURE, AbortLocation, StatusCode},
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
    let mut builder = PackageBuilder::new("P");
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

#[test]
fn test_bcs_to_bytes_value_size_metering() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    publish_deep_wrapper(&mut h, &acc, 120, 100, 30);

    let gas_off = h.evaluate_entry_function_gas(&acc, run_id(), vec![], vec![]);
    h.set_timed_feature(TimedFeatureFlag::MeterBcsByValueSize, true);
    let gas_on = h.evaluate_entry_function_gas(&acc, run_id(), vec![], vec![]);

    println!("bcs::to_bytes gas  off={gas_off}  on={gas_on}");
    assert!(
        gas_on > gas_off * 20,
        "value-size metering should dominate: on={gas_on} off={gas_off}"
    );
}

#[test]
fn test_bcs_to_bytes_execution_limit() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    publish_deep_wrapper(&mut h, &acc, 120, 200, 100);

    assert_success!(h.run_entry_function(&acc, run_id(), vec![], vec![]));

    // Enable timed feature flag.
    h.set_timed_feature(TimedFeatureFlag::MeterBcsByValueSize, true);

    let status = h.run_entry_function(&acc, run_id(), vec![], vec![]);
    assert!(matches!(
        status,
        TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some(
            StatusCode::EXECUTION_LIMIT_REACHED
        )))
    ));
}

// A `vector<D1>` of `n` depth-120 single-field values holds `1 + 121 * n` value
// nodes, so a few hundred input bytes expand into thousands of value nodes.
const SMALL_ELEMS: u64 = 60;
const LARGE_ELEMS: u64 = 100;

/// Generates a module for exercising deserialize-side value-node metering. `D1{f: D2}
/// ... D{depth}{f: u8}` is a single-field wrapper chain (all `copy`+`store`+`drop`),
/// so a `vector<D1>` of `n` elements is only ~`n` input bytes but ~`(depth + 1) * n`
/// value nodes. The entries let a test store a value with the flag off, then load it
/// with `borrow_global`, unpack it with `from_bytes` (via `copyable_any`), or read it
/// back through the table natives (`init_table`/`load_table`) once the flag is on —
/// deserialization is never capped, so it is charged, not rejected.
fn deep_deser_source(depth: usize) -> String {
    let mut src = String::from(
        "module 0xcafe::test {\n    use std::vector;\n    use aptos_std::copyable_any;\n    use aptos_std::table::{Self, Table};\n\n",
    );
    for k in 1..depth {
        src.push_str(&format!(
            "    struct D{} has drop, store, copy {{ f: D{} }}\n",
            k,
            k + 1
        ));
    }
    src.push_str(&format!(
        "    struct D{} has drop, store, copy {{ f: u8 }}\n\n",
        depth
    ));
    src.push_str("    struct Big has key { v: vector<D1> }\n");
    src.push_str("    struct Holder has key { a: copyable_any::Any }\n");
    src.push_str("    struct BigTable has key { t: Table<u64, vector<D1>> }\n\n");

    // Nested single-field constructor: D1{f:D2{f: ... D{depth}{f:0u8} ...}}.
    let mut ctor = String::from("0u8");
    for k in (1..=depth).rev() {
        ctor = format!("D{}{{f:{}}}", k, ctor);
    }

    src.push_str("    fun build(n: u64): vector<D1> {\n");
    src.push_str("        let v = vector::empty<D1>();\n");
    src.push_str("        let i: u64 = 0;\n");
    src.push_str(&format!(
        "        while (i < n) {{ vector::push_back(&mut v, {ctor}); i = i + 1; }};\n"
    ));
    src.push_str("        v\n    }\n\n");

    src.push_str("    public entry fun init_big(s: &signer, n: u64) {\n");
    src.push_str("        move_to(s, Big { v: build(n) });\n    }\n\n");

    src.push_str("    public entry fun load_big(addr: address) acquires Big {\n");
    src.push_str("        let b = borrow_global<Big>(addr);\n");
    src.push_str("        let _len = vector::length(&b.v);\n    }\n\n");

    src.push_str("    public entry fun init_holder(s: &signer, n: u64) {\n");
    src.push_str("        move_to(s, Holder { a: copyable_any::pack(build(n)) });\n    }\n\n");

    src.push_str("    public entry fun unpack_holder(addr: address) acquires Holder {\n");
    src.push_str("        let Holder { a } = move_from<Holder>(addr);\n");
    src.push_str("        let _v = copyable_any::unpack<vector<D1>>(a);\n    }\n\n");

    src.push_str("    public entry fun init_table(s: &signer, n: u64, m: u64) {\n");
    src.push_str("        let t = table::new<u64, vector<D1>>();\n");
    src.push_str("        let k: u64 = 0;\n");
    src.push_str("        while (k < m) { table::add(&mut t, k, build(n)); k = k + 1; };\n");
    src.push_str("        move_to(s, BigTable { t });\n    }\n\n");

    src.push_str("    public entry fun load_table(addr: address, m: u64) acquires BigTable {\n");
    src.push_str("        let b = borrow_global<BigTable>(addr);\n");
    src.push_str("        let k: u64 = 0;\n");
    src.push_str("        while (k < m) { let _len = vector::length(table::borrow(&b.t, k)); k = k + 1; };\n");
    src.push_str("    }\n\n");

    src.push_str("    public entry fun roundtrip(n: u64, iters: u64) {\n");
    src.push_str("        let a = copyable_any::pack(build(n));\n");
    src.push_str("        let i: u64 = 0;\n");
    src.push_str("        while (i < iters) {\n");
    src.push_str("            let _v = copyable_any::unpack<vector<D1>>(copy a);\n");
    src.push_str("            i = i + 1;\n        };\n    }\n}");
    src
}

fn publish_deep_deser(h: &mut MoveHarness, acc: &Account) {
    let mut builder = PackageBuilder::new("P");
    builder.add_source("test", &deep_deser_source(120));
    builder.add_local_dep(
        "AptosStdlib",
        &common::framework_dir_path("aptos-stdlib").to_string_lossy(),
    );
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

fn entry(name: &str) -> MemberId {
    MemberId::from_str(&format!("0xcafe::test::{name}")).unwrap()
}

/// Deserializing a value via `from_bytes` (reached through `copyable_any::unpack`)
/// must cost execution gas proportional to the produced node count once the flag is
/// on. Loops `unpack` so the traversal charge dominates the transaction.
#[test]
fn test_from_bytes_value_node_metering() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    publish_deep_deser(&mut h, &acc);

    let args = vec![
        bcs::to_bytes(&SMALL_ELEMS).unwrap(),
        bcs::to_bytes(&30u64).unwrap(),
    ];
    let gas_off = h.evaluate_entry_function_gas(&acc, entry("roundtrip"), vec![], args.clone());
    // Enabling the flag activates the per-node deserialize charge. The baseline is
    // measured at genesis (flag off); a single `set_timed_feature` flips it on.
    h.set_timed_feature(TimedFeatureFlag::MeterValueNodesOnDeserialize, true);
    let gas_on = h.evaluate_entry_function_gas(&acc, entry("roundtrip"), vec![], args);

    println!("from_bytes gas  off={gas_off}  on={gas_on}");
    assert!(
        gas_on > gas_off * 20,
        "deserialize node metering should dominate: on={gas_on} off={gas_off}"
    );
}

/// Deserialization has no node-count limit: a large value unpacks via
/// `copyable_any::unpack` (`from_bytes`) once the flag is on and is only charged for
/// the traversal, never rejected. Gas, not a hard cap, bounds deserialization.
#[test]
fn test_from_bytes_large_value_unpacks() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    publish_deep_deser(&mut h, &acc);

    let n = vec![bcs::to_bytes(&LARGE_ELEMS).unwrap()];
    assert_success!(h.run_entry_function(&acc, entry("init_holder"), vec![], n));

    h.set_timed_feature(TimedFeatureFlag::MeterValueNodesOnDeserialize, true);
    let addr = vec![bcs::to_bytes(acc.address()).unwrap()];
    assert_success!(h.run_entry_function(&acc, entry("unpack_holder"), vec![], addr));
}

/// Loading a present resource with `borrow_global` must cost execution gas
/// proportional to its node count once the flag is on. `MeterBcsByValueSize` does
/// not touch the load path, so the genesis-vs-on delta isolates this charge.
#[test]
fn test_load_resource_value_node_metering() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    publish_deep_deser(&mut h, &acc);

    let n = vec![bcs::to_bytes(&SMALL_ELEMS).unwrap()];
    assert_success!(h.run_entry_function(&acc, entry("init_big"), vec![], n));

    let addr = vec![bcs::to_bytes(acc.address()).unwrap()];
    let gas_off = h.evaluate_entry_function_gas(&acc, entry("load_big"), vec![], addr.clone());
    h.set_timed_feature(TimedFeatureFlag::MeterValueNodesOnDeserialize, true);
    let gas_on = h.evaluate_entry_function_gas(&acc, entry("load_big"), vec![], addr);

    println!("load_resource gas  off={gas_off}  on={gas_on}");
    assert!(
        gas_on > gas_off * 3,
        "resource-load node metering should raise gas: on={gas_on} off={gas_off}"
    );
}

/// The same for the bytecode load path: a large stored resource loads with
/// `borrow_global` once the flag is on, bounded by the per-node gas charge rather than
/// any node-count cap.
#[test]
fn test_load_resource_large_value_loads() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    publish_deep_deser(&mut h, &acc);

    let n = vec![bcs::to_bytes(&LARGE_ELEMS).unwrap()];
    assert_success!(h.run_entry_function(&acc, entry("init_big"), vec![], n));

    h.set_timed_feature(TimedFeatureFlag::MeterValueNodesOnDeserialize, true);
    let addr = vec![bcs::to_bytes(acc.address()).unwrap()];
    assert_success!(h.run_entry_function(&acc, entry("load_big"), vec![], addr));
}

/// The table natives deserialize a value from storage on `table::borrow`, so loading it
/// must cost execution gas proportional to its node count once the flag is on. Each
/// distinct key is deserialized once per transaction, so borrowing `M` keys forces `M`
/// independent deserializations; the genesis-vs-on delta isolates the per-node charge
/// added to the table natives.
#[test]
fn test_table_load_value_node_metering() {
    const M: u64 = 10;
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    publish_deep_deser(&mut h, &acc);

    let init_args = vec![
        bcs::to_bytes(&SMALL_ELEMS).unwrap(),
        bcs::to_bytes(&M).unwrap(),
    ];
    assert_success!(h.run_entry_function(&acc, entry("init_table"), vec![], init_args));

    let load_args = vec![
        bcs::to_bytes(acc.address()).unwrap(),
        bcs::to_bytes(&M).unwrap(),
    ];
    let gas_off =
        h.evaluate_entry_function_gas(&acc, entry("load_table"), vec![], load_args.clone());
    h.set_timed_feature(TimedFeatureFlag::MeterValueNodesOnDeserialize, true);
    let gas_on = h.evaluate_entry_function_gas(&acc, entry("load_table"), vec![], load_args);

    println!("table load gas  off={gas_off}  on={gas_on}");
    assert!(
        gas_on > gas_off * 3,
        "table deserialize node metering should raise gas: on={gas_on} off={gas_off}"
    );
}

/// Table loads have no node-count limit either: a large value deserializes through
/// `table::borrow` once the flag is on and is only charged for the traversal, never
/// rejected.
#[test]
fn test_table_load_large_value_loads() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    publish_deep_deser(&mut h, &acc);

    let init_args = vec![
        bcs::to_bytes(&LARGE_ELEMS).unwrap(),
        bcs::to_bytes(&1u64).unwrap(),
    ];
    assert_success!(h.run_entry_function(&acc, entry("init_table"), vec![], init_args));

    h.set_timed_feature(TimedFeatureFlag::MeterValueNodesOnDeserialize, true);
    let load_args = vec![
        bcs::to_bytes(acc.address()).unwrap(),
        bcs::to_bytes(&1u64).unwrap(),
    ];
    assert_success!(h.run_entry_function(&acc, entry("load_table"), vec![], load_args));
}
