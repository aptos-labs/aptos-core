// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests for deeply-nested function values. The Move VM must ensure that it is not possible to
//! construct values that are too deep, as this can cause stack overflow.

use crate::{assert_success, assert_vm_status, tests::common, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_gas_schedule::gas_feature_versions::RELEASE_V1_49;
use aptos_package_builder::PackageBuilder;
use aptos_transaction_simulation::Account;
use aptos_types::{
    on_chain_config::TimedFeatureFlag,
    transaction::{AuxiliaryInfo, TransactionStatus},
};
use move_core_types::{account_address::AccountAddress, vm_status::StatusCode};

#[test]
fn test_vm_value_too_deep_with_function_values() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x99").unwrap());

    let status = publish(
        &mut h,
        &acc,
        r#"
        module 0x99::m {
            public fun dummy2(_v: || has drop+copy) {}

            // Creates a very deep value that can be tested for off by 1 around the current maximum
            // depth value.
            public entry fun run2(n: u64) {
                let f: || has copy+drop = || {};
                let i = 0;
                while (i < n) {
                  f = || dummy2(f);
                  i = i + 1;
                };
            }
        }
        "#,
    );
    assert_success!(status);

    let status = h.run_entry_function(&acc, str::parse("0x99::m::run2").unwrap(), vec![], vec![
        bcs::to_bytes(&129_u64).unwrap(),
    ]);
    assert_vm_status!(status, StatusCode::VM_MAX_VALUE_DEPTH_REACHED);

    let status = h.run_entry_function(&acc, str::parse("0x99::m::run2").unwrap(), vec![], vec![
        bcs::to_bytes(&128_u64).unwrap(),
    ]);
    assert_success!(status);
}

/// Packing a closure checks the depth of its captured values by traversing them, so the charge
/// must scale with their size, unless their type bounds their depth. The entry functions pack a
/// closure over the same captured value `n` times, recovering the value by calling the closure.
#[test]
fn test_pack_closure_captured_values_metering() {
    const NARROW: u64 = 1;
    const WIDE: u64 = 1000;

    for (gas_feature_version, timed_feature, metered) in
        [(RELEASE_V1_49, false, false), (RELEASE_V1_49, true, true)]
    {
        let mut h = MoveHarness::new();
        h.modify_gas_schedule_raw(|schedule| schedule.feature_version = gas_feature_version);
        h.set_timed_feature(
            TimedFeatureFlag::MeterPackClosureCapturedValues,
            timed_feature,
        );
        let acc = h.new_account_at(AccountAddress::from_hex_literal("0x99").unwrap());
        let status = publish(
            &mut h,
            &acc,
            r#"
            module 0x99::m {
                struct Holder has drop { v: vector<vector<u8>> }

                fun id(h: Holder): Holder { h }

                fun id_plain(v: vector<vector<u8>>): vector<vector<u8>> { v }

                fun id_generic<T>(v: T): T { v }

                fun make(width: u64): vector<vector<u8>> {
                    let v = vector[];
                    for (i in 0..width) { v.push_back(vector[1u8]) };
                    v
                }

                public entry fun repack(width: u64, n: u64) {
                    let h = Holder { v: make(width) };
                    for (i in 0..n) {
                        let f = || id(h);
                        h = f();
                    };
                }

                public entry fun repack_plain(width: u64, n: u64) {
                    let v = make(width);
                    for (i in 0..n) {
                        let f = || id_plain(v);
                        v = f();
                    };
                }

                public entry fun repack_generic(width: u64, n: u64) {
                    let h = Holder { v: make(width) };
                    for (i in 0..n) {
                        let f = || id_generic(h);
                        h = f();
                    };
                }
            }
            "#,
        );
        assert_success!(status);

        let loop_cost = |h: &mut MoveHarness, fun: &str, width: u64| {
            let mut gas = |n: u64| {
                h.evaluate_entry_function_gas(
                    &acc,
                    str::parse(&format!("0x99::m::{fun}")).unwrap(),
                    vec![],
                    vec![bcs::to_bytes(&width).unwrap(), bcs::to_bytes(&n).unwrap()],
                )
            };
            gas(100) - gas(0)
        };
        let narrow = loop_cost(&mut h, "repack", NARROW);
        let wide = loop_cost(&mut h, "repack", WIDE);
        let plain_narrow = loop_cost(&mut h, "repack_plain", NARROW);
        let plain_wide = loop_cost(&mut h, "repack_plain", WIDE);
        println!(
            "gas version {gas_feature_version}, timed feature {timed_feature}: \
             narrow={narrow} wide={wide} plain_narrow={plain_narrow} plain_wide={plain_wide}"
        );
        if metered {
            assert!(wide > narrow * 10, "narrow={narrow} wide={wide}");
        } else {
            assert!(wide <= narrow + 1, "narrow={narrow} wide={wide}");
        }
        // The type of a vector of vectors bounds its depth, so it is not traversed.
        assert!(
            plain_wide <= plain_narrow + 1,
            "plain_narrow={plain_narrow} plain_wide={plain_wide}"
        );

        // Both closure charges must contribute to the cost of a single opcode hit.
        // Cover generic packing, which can record type creation between the charges,
        // and plain captures, which do not incur the traversal charge.
        const REPACKS: u64 = 3;
        for fun in ["repack", "repack_plain", "repack_generic"] {
            let fun = format!("0x99::m::{fun}");
            let args = vec![
                bcs::to_bytes(&NARROW).unwrap(),
                bcs::to_bytes(&REPACKS).unwrap(),
            ];
            let expected_gas = h.evaluate_entry_function_gas(
                &acc,
                str::parse(&fun).unwrap(),
                vec![],
                args.clone(),
            );
            let txn = h.create_entry_function(&acc, str::parse(&fun).unwrap(), vec![], args);
            let (status, log, gas_used, _) =
                h.evaluate_gas_with_profiler_and_status_signed(txn, &AuxiliaryInfo::default());
            assert_success!(status);
            assert_eq!(gas_used, expected_gas);
            let ops = log.exec_io.aggregate_gas_events().ops;
            let (_, hits, _) = ops
                .iter()
                .find(|(op, _, _)| op == "pack_closure")
                .expect("repacking must execute closure packing");
            assert_eq!(*hits as u64, REPACKS);
        }
    }
}

fn publish(h: &mut MoveHarness, account: &Account, source: &str) -> TransactionStatus {
    let mut builder = PackageBuilder::new("Package");
    builder.add_source("m.move", source);
    builder.add_local_dep(
        "MoveStdlib",
        &common::framework_dir_path("move-stdlib").to_string_lossy(),
    );
    let path = builder.write_to_temp().unwrap();
    h.publish_package_with_options(
        account,
        path.path(),
        BuildOptions::move_2().set_latest_language(),
    )
}
