// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Regression test for captured-closure-layout DAG blow-up during BCS serialization.
//!
//! Publishes a doubling-struct chain whose layout is a DAG and packs a persistent
//! closure that captures it. Checks that small N is accepted and large N is rejected.

use crate::{assert_success, tests::common, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_package_builder::PackageBuilder;
use aptos_types::{
    account_address::AccountAddress,
    transaction::{ExecutionStatus, TransactionStatus},
};
use move_core_types::vm_status::StatusCode;

fn gen_source(addr: &str, n: usize) -> String {
    assert!(n >= 1);
    let mut s = String::new();
    s.push_str(&format!("module {}::test {{\n", addr));
    s.push_str("    struct S0 has copy, drop, store { v: u8 }\n");
    for i in 0..n {
        s.push_str(&format!(
            "    struct S{} has copy, drop, store {{ l: S{}, r: S{} }}\n",
            i + 1,
            i,
            i
        ));
    }
    s.push_str("    struct Work has copy, drop, store, key { bar: || }\n");
    s.push_str(&format!(
        "    #[persistent] fun consumer(_o: vector<S{}>) {{}}\n",
        n
    ));
    s.push_str(&format!(
        "    entry fun store_closure(s: &signer) {{\n        \
                 let o: vector<S{}> = vector[];\n        \
                 move_to(s, Work {{ bar: || consumer(o) }})\n    \
             }}\n",
        n
    ));
    s.push_str("}\n");
    s
}

fn run_for(n: usize) -> TransactionStatus {
    let addr = "0xcafe";
    let src = gen_source(addr, n);

    let mut builder = PackageBuilder::new("Test");
    builder.add_source("test.move", &src);
    builder.add_local_dep(
        "MoveStdlib",
        &common::framework_dir_path("move-stdlib").to_string_lossy(),
    );
    let path = builder.write_to_temp().unwrap();

    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal(addr).unwrap());
    assert_success!(h.publish_package_with_options(
        &acc,
        path.path(),
        BuildOptions::move_2().set_latest_language(),
    ));

    let txn = h.create_entry_function(
        &acc,
        str::parse(&format!("{}::test::store_closure", addr)).unwrap(),
        vec![],
        vec![],
    );
    h.run(txn)
}

/// Expanded layout has 3*2^10 - 1 = 3071 nodes, under the 4096 cap.
#[test]
fn captured_layout_dag_under_cap_succeeds() {
    let status = run_for(10);
    assert_success!(status);
}

/// Expanded layout has 3*2^11 - 1 = 6143 nodes, over the 4096 cap.
#[test]
fn captured_layout_dag_over_cap_rejected() {
    let status = run_for(11);
    assert!(
        matches!(
            status,
            TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some(
                StatusCode::VALUE_SERIALIZATION_ERROR
            )))
        ),
        "unexpected status: {:?}",
        status
    );
}
