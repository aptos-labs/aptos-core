// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! A storable closure serializes the layout of every captured value in full, while the layout
//! is shared in memory. `TimedFeatureFlag::MeterClosureCapturedLayouts` includes the serialized
//! size of the captured layouts in the closure's abstract size, so building a value that
//! serializes to far more than the memory quota hits the quota instead.

use crate::{assert_success, tests::common, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_gas_schedule::gas_feature_versions::RELEASE_V1_49;
use aptos_package_builder::PackageBuilder;
use aptos_transaction_simulation::Account;
use aptos_types::{
    account_address::AccountAddress,
    move_utils::MemberId,
    on_chain_config::TimedFeatureFlag,
    transaction::{AuxiliaryInfo, ExecutionStatus, TransactionStatus},
};
use move_core_types::{
    value::{MoveStructLayout, MoveTypeLayout, VARIANT_COUNT_MAX},
    vm_status::{AbortLocation, StatusCode},
};

const ADDR: &str = "0xbeef";

/// Generates
/// ```text
/// enum E { V0, .., V126 }
/// struct D0 { l: E, r: E }
/// struct D_i { l: D_{i - 1}, r: D_{i - 1} }  // for 0 < i <= depth
/// ```
/// and a closure capturing an empty `vector<D_depth>`. The layout of `D_depth` is a DAG of
/// `depth + 2` nodes, but serializes to `266 * 2^depth - 3` bytes.
fn module_source(depth: usize) -> String {
    let variants = (0..VARIANT_COUNT_MAX)
        .map(|i| format!("V{}", i))
        .collect::<Vec<_>>()
        .join(", ");
    let mut structs = "struct D0 has copy, drop, store { l: E, r: E }\n".to_string();
    for i in 1..=depth {
        structs.push_str(&format!(
            "    struct D{} has copy, drop, store {{ l: D{}, r: D{} }}\n",
            i,
            i - 1,
            i - 1
        ));
    }
    format!(
        r#"
module {ADDR}::m {{
    enum E has copy, drop, store {{ {variants} }}
    {structs}
    struct Holder has key {{ fs: vector<|| has copy+drop+store> }}

    #[persistent]
    fun consume(_v: vector<D{depth}>) {{}}

    fun make(): || has copy+drop+store {{
        let v: vector<D{depth}> = vector[];
        || consume(v)
    }}

    fun build(n: u64): vector<|| has copy+drop+store> {{
        let fs = vector[];
        for (i in 0..n) {{ fs.push_back(make()) }};
        fs
    }}

    entry fun run(n: u64) {{
        build(n);
    }}

    entry fun store(s: &signer, n: u64) {{
        move_to(s, Holder {{ fs: build(n) }})
    }}
}}
"#
    )
}

/// Serialized size of the layout captured by `make`, computed by BCS on the unfolded layout.
fn captured_layout_size(depth: usize) -> u64 {
    let mut layout = MoveTypeLayout::new_struct(MoveStructLayout::RuntimeVariants(vec![
        vec![];
        VARIANT_COUNT_MAX as usize
    ]));
    for _ in 0..=depth {
        layout = MoveTypeLayout::new_struct(MoveStructLayout::Runtime(vec![layout; 2]));
    }
    let layout = MoveTypeLayout::Vector(Box::new(layout));
    bcs::serialized_size(&layout).expect("layout must be serializable") as u64
}

fn setup(depth: usize, metered: bool) -> (MoveHarness, Account) {
    let mut h = MoveHarness::new();
    h.max_gas_per_txn = 2_000_000;
    h.modify_gas_schedule_raw(|schedule| schedule.feature_version = RELEASE_V1_49);
    h.set_timed_feature(TimedFeatureFlag::MeterClosureCapturedLayouts, metered);
    let acc = h.new_account_at(AccountAddress::from_hex_literal(ADDR).expect("valid address"));

    let mut builder = PackageBuilder::new("Test");
    builder.add_source("m.move", &module_source(depth));
    builder.add_local_dep(
        "MoveStdlib",
        &common::framework_dir_path("move-stdlib").to_string_lossy(),
    );
    let path = builder
        .write_to_temp()
        .expect("package must be written to a temporary directory");
    assert_success!(h.publish_package_with_options(
        &acc,
        path.path(),
        BuildOptions::move_2().set_latest_language(),
    ));
    (h, acc)
}

fn entry_function(fun: &str) -> MemberId {
    str::parse(&format!("{}::m::{}", ADDR, fun)).expect("valid function id")
}

fn call(h: &mut MoveHarness, acc: &Account, fun: &str, n: u64) -> TransactionStatus {
    h.run_entry_function(acc, entry_function(fun), vec![], vec![n_arg(n)])
}

fn n_arg(n: u64) -> Vec<u8> {
    bcs::to_bytes(&n).expect("u64 must serialize")
}

/// Peak abstract memory usage of `run(n)`, which is what `txn.memory_quota` bounds.
fn peak_memory(h: &mut MoveHarness, acc: &Account, n: u64) -> u64 {
    let txn = h.create_entry_function(acc, entry_function("run"), vec![], vec![n_arg(n)]);
    let (status, gas_log, ..) =
        h.evaluate_gas_with_profiler_and_status_signed(txn, &AuxiliaryInfo::default());
    assert_success!(status);
    u64::from(gas_log.peak_memory_usage)
}

/// Abstract memory charged per closure. The slope between two closure counts cancels the
/// transaction's fixed overhead.
fn charge_per_closure(depth: usize, metered: bool) -> u64 {
    const SMALL: u64 = 10;
    const LARGE: u64 = 30;
    let (mut h, acc) = setup(depth, metered);
    let small = peak_memory(&mut h, &acc, SMALL);
    let large = peak_memory(&mut h, &acc, LARGE);
    (large - small) / (LARGE - SMALL)
}

/// With metering, a closure costs exactly the bytes its captured layout serializes to on top.
#[test]
fn captured_layout_size_is_charged() {
    for depth in [0, 4] {
        let unmetered = charge_per_closure(depth, false);
        let metered = charge_per_closure(depth, true);
        assert_eq!(
            metered - unmetered,
            captured_layout_size(depth),
            "depth {}",
            depth
        );
    }
}

/// Each closure capturing `vector<D9>` serializes to about 136 KB, while the layout DAG has
/// 12 nodes. Without metering, 80 of them fit under the memory quota and are serialized to about
/// 11 MB before the write limit rejects them. With metering, building them exceeds the quota.
#[test]
fn closures_with_large_captured_layouts_hit_memory_quota() {
    const DEPTH: usize = 9;
    const N: u64 = 80;
    assert!(N * captured_layout_size(DEPTH) > 10_000_000);

    let (mut h, acc) = setup(DEPTH, false);
    assert_success!(call(&mut h, &acc, "run", N));
    // The write limit fails the transaction with `STORAGE_WRITE_LIMIT_REACHED`, which the
    // transaction status records as an execution failure without the status code.
    assert_eq!(
        call(&mut h, &acc, "store", N),
        TransactionStatus::Keep(ExecutionStatus::ExecutionFailure {
            location: AbortLocation::Script,
            function: 0,
            code_offset: 0,
        })
    );

    let (mut h, acc) = setup(DEPTH, true);
    assert_eq!(
        call(&mut h, &acc, "run", N),
        TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some(
            StatusCode::MEMORY_LIMIT_EXCEEDED
        )))
    );

    // One such closure still fits under the quota and the write limit.
    assert_success!(call(&mut h, &acc, "store", 1));
}
