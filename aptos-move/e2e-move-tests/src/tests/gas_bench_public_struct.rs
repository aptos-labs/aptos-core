// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Gas benchmark for the `public struct` feature.
//!
//! Run with:
//!   cargo test -p e2e-move-tests test_gas_bench_public_struct -- --nocapture
//!
//! For each of three operations (direct read, unpack, borrow) and four iteration
//! counts (1, 10, 100, 1000), compares gas between:
//!
//!   same-module  — operation in the DEFINING module (direct bytecode, no accessor call)
//!   cross-module — operation in an EXTERNAL module via `public struct`
//!                  (compiler converts each field op into an auto-generated accessor call)
//!
//! Expected: gas(cross-module) >= gas(same-module) for all operations and counts.
//! The overhead grows linearly with n, quantifying the per-operation cost of
//! cross-module access.

use crate::{assert_success, tests::common, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_types::{
    account_address::AccountAddress,
    transaction::{EntryFunction, TransactionPayload},
};
use move_core_types::{identifier::Identifier, language_storage::ModuleId};

const MOD_ADDR: &str = "0xcafe";

fn setup(harness: &mut MoveHarness) -> AccountAddress {
    let mod_addr = AccountAddress::from_hex_literal(MOD_ADDR).unwrap();
    let module_account = harness.new_account_at(mod_addr);
    assert_success!(harness.publish_package_with_options(
        &module_account,
        &common::test_dir_path("gas_bench_public_struct.data/pack"),
        BuildOptions::move_2().set_latest_language(),
    ));
    mod_addr
}

fn run_bench(
    harness: &mut MoveHarness,
    mod_addr: AccountAddress,
    module_name: &str,
    fn_name: &str,
    n: u64,
) -> u64 {
    let caller = harness.new_account_at(AccountAddress::random());
    let module_id = ModuleId::new(mod_addr, Identifier::new(module_name).unwrap());
    let (_gas_log, gas_used, _fee) = harness.evaluate_gas_with_profiler(
        &caller,
        TransactionPayload::EntryFunction(EntryFunction::new(
            module_id,
            Identifier::new(fn_name).unwrap(),
            vec![],
            vec![bcs::to_bytes(&n).unwrap()],
        )),
    );
    gas_used
}

/// Gas benchmark for `public struct` cross-module field operations.
///
/// Covers five operations × four iteration counts:
///   struct: bench_direct (field read), bench_unpack (destructure), bench_pack (construction)
///   enum:   bench_enum_pack (variant construction), bench_enum_test (variant match)
///   counts: 1, 10, 100, 1000
///
/// For each (op, n) pair, compares same-module (baseline) vs. cross-module (accessor call overhead).
#[test]
fn test_gas_bench_public_struct() {
    let mut harness = MoveHarness::new();
    let mod_addr = setup(&mut harness);

    let ops: &[(&str, &str)] = &[
        ("bench_direct", "struct direct read  (config.a)"),
        (
            "bench_unpack",
            "struct unpack       (let Config { a, b, c, d } = config)",
        ),
        ("bench_pack", "struct pack         (Config { a, b, c, d })"),
        (
            "bench_enum_pack",
            "enum pack           (Shape::Circle { radius: i })",
        ),
        (
            "bench_enum_test",
            "enum test variant   (match &shape { Circle => .. })",
        ),
    ];
    let counts = [1u64, 10, 100, 1000];

    for (fn_name, op_label) in ops {
        println!("\n--- Operation: {} ---", op_label);
        println!(
            "  {:>6}  {:>14}  {:>14}  {:>10}",
            "n", "same-module", "cross-module", "diff"
        );
        println!("  {}", "-".repeat(50));

        for n in counts {
            let gas_same = run_bench(&mut harness, mod_addr, "gas_bench_types", fn_name, n);
            let gas_cross = run_bench(&mut harness, mod_addr, "gas_bench_consumer", fn_name, n);
            let diff = gas_cross as i64 - gas_same as i64;
            println!(
                "  {:>6}  {:>14}  {:>14}  {:>+10}",
                n, gas_same, gas_cross, diff
            );

            assert!(
                gas_cross >= gas_same,
                "op={} n={}: cross-module ({} gas) should be >= same-module ({} gas)",
                fn_name,
                n,
                gas_cross,
                gas_same
            );
        }
    }
}
