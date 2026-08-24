// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests for `ENABLE_FUNCTION_DATA_FORMAT_V2`: function values are written in storage
//! format V2, V1 data stays readable and upgrades on rewrite, and operations on
//! stored function values (call, equality, comparison, formatting) materialize the
//! captured arguments on demand.

use crate::{assert_success, tests::common, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_language_e2e_tests::account::Account;
use aptos_package_builder::PackageBuilder;
use aptos_types::{
    account_address::AccountAddress, on_chain_config::FeatureFlag, transaction::TransactionStatus,
};
use move_core_types::language_storage::StructTag;
use std::str::FromStr;

const SOURCE: &str = r#"
module 0x66::m {
    use std::bcs;
    use std::cmp;
    use std::signer;
    use std::string;
    use std::vector;
    use aptos_std::string_utils;

    #[persistent]
    fun incr(x: u64, y: u64): u64 { x + y }

    struct Holder has key { f: |u64|u64 has copy+store+drop }

    fun captured(x: u64): |u64|u64 has copy+store+drop {
        |y| incr(x, y)
    }

    public entry fun store(account: &signer, x: u64) {
        move_to(account, Holder { f: captured(x) })
    }

    public entry fun rewrite(account: &signer, x: u64) acquires Holder {
        let Holder { f: _ } = move_from<Holder>(signer::address_of(account));
        move_to(account, Holder { f: captured(x) })
    }

    public entry fun call_stored(account: &signer, x: u64, expected: u64) acquires Holder {
        let f = borrow_global<Holder>(signer::address_of(account)).f;
        assert!(f(x) == expected, 1);
    }

    // Equality between a stored and a fresh function value: materializes the stored
    // captured arguments on demand.
    public entry fun eq_stored_vs_fresh(
        account: &signer,
        x: u64,
        expected_eq: bool,
    ) acquires Holder {
        let f = borrow_global<Holder>(signer::address_of(account)).f;
        let g = captured(x);
        assert!((f == g) == expected_eq, 2);
    }

    // Equality between two loads of the same stored function value: equal serialized
    // captured arguments compare equal without materialization.
    public entry fun eq_stored_vs_stored(account: &signer) acquires Holder {
        let f = borrow_global<Holder>(signer::address_of(account)).f;
        let g = borrow_global<Holder>(signer::address_of(account)).f;
        assert!(f == g, 3);
    }

    public entry fun cmp_stored_vs_fresh(
        account: &signer,
        x: u64,
        expected_eq: bool,
    ) acquires Holder {
        let f = borrow_global<Holder>(signer::address_of(account)).f;
        let g = captured(x);
        assert!(cmp::compare(&f, &g).is_eq() == expected_eq, 4);
    }

    // Formatting a stored function value must look exactly like formatting a freshly
    // created one, i.e., captured arguments are decoded and printed.
    public entry fun format_stored_matches_fresh(account: &signer, x: u64) acquires Holder {
        let f = &borrow_global<Holder>(signer::address_of(account)).f;
        let g = captured(x);
        let formatted = string_utils::to_string(f);
        assert!(formatted == string_utils::to_string(&g), 5);
        // The captured argument is decoded and printed, not shown as opaque bytes.
        assert!(string::index_of(&formatted, &string::utf8(b"10")) != string::length(&formatted), 6);
    }

    // Serializes a fresh function value and checks the format version byte.
    public entry fun to_bytes_version(expected_version: u8) {
        let f = captured(10);
        let bz = bcs::to_bytes(&f);
        // Element 0 is the BCS sequence length, elements 1-2 the version (u16, LE).
        assert!(*vector::borrow(&bz, 1) == expected_version, 7);
        assert!(*vector::borrow(&bz, 2) == 0, 8);
    }
}
"#;

fn publish(h: &mut MoveHarness, account: &Account) -> TransactionStatus {
    let mut builder = PackageBuilder::new("Package");
    builder.add_source("m.move", SOURCE);
    builder.add_local_dep(
        "AptosStdlib",
        &common::framework_dir_path("aptos-stdlib").to_string_lossy(),
    );
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

fn run(
    h: &mut MoveHarness,
    account: &Account,
    name: &str,
    args: Vec<Vec<u8>>,
) -> TransactionStatus {
    h.run_entry_function(
        account,
        str::parse(&format!("0x66::m::{}", name)).unwrap(),
        vec![],
        args,
    )
}

/// Returns the stored format version of the closure in `Holder` (the resource is a
/// single closure field, so the version u16 sits right after the sequence length
/// byte).
fn stored_format_version(h: &MoveHarness, addr: &AccountAddress) -> u16 {
    let bytes = h
        .read_resource_raw(addr, StructTag::from_str("0x66::m::Holder").unwrap())
        .expect("Holder must exist");
    u16::from_le_bytes([bytes[1], bytes[2]])
}

fn run_all_stored_closure_ops(h: &mut MoveHarness, acc: &Account) {
    assert_success!(run(h, acc, "call_stored", vec![
        bcs::to_bytes(&5u64).unwrap(),
        bcs::to_bytes(&15u64).unwrap(),
    ]));
    assert_success!(run(h, acc, "eq_stored_vs_fresh", vec![
        bcs::to_bytes(&10u64).unwrap(),
        bcs::to_bytes(&true).unwrap(),
    ]));
    assert_success!(run(h, acc, "eq_stored_vs_fresh", vec![
        bcs::to_bytes(&11u64).unwrap(),
        bcs::to_bytes(&false).unwrap(),
    ]));
    assert_success!(run(h, acc, "eq_stored_vs_stored", vec![]));
    assert_success!(run(h, acc, "cmp_stored_vs_fresh", vec![
        bcs::to_bytes(&10u64).unwrap(),
        bcs::to_bytes(&true).unwrap(),
    ]));
    assert_success!(run(h, acc, "cmp_stored_vs_fresh", vec![
        bcs::to_bytes(&11u64).unwrap(),
        bcs::to_bytes(&false).unwrap(),
    ]));
    assert_success!(run(h, acc, "format_stored_matches_fresh", vec![
        bcs::to_bytes(&10u64).unwrap()
    ]));
}

#[test]
fn function_value_v1_upgrades_to_v2_on_rewrite() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x66").unwrap());
    assert_success!(publish(&mut h, &acc));

    // Flag off: stored in V1.
    assert_success!(run(&mut h, &acc, "store", vec![
        bcs::to_bytes(&10u64).unwrap()
    ]));
    assert_eq!(stored_format_version(&h, acc.address()), 1);
    run_all_stored_closure_ops(&mut h, &acc);

    h.enable_features(vec![FeatureFlag::ENABLE_FUNCTION_DATA_FORMAT_V2], vec![]);

    // V1 data stays fully usable with the flag on.
    run_all_stored_closure_ops(&mut h, &acc);

    // Rewriting upgrades the stored bytes to V2.
    assert_success!(run(&mut h, &acc, "rewrite", vec![
        bcs::to_bytes(&10u64).unwrap()
    ]));
    assert_eq!(stored_format_version(&h, acc.address()), 2);
    run_all_stored_closure_ops(&mut h, &acc);
}

#[test]
fn function_value_v2_stored_and_used() {
    let mut h = MoveHarness::new();
    h.enable_features(vec![FeatureFlag::ENABLE_FUNCTION_DATA_FORMAT_V2], vec![]);
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x66").unwrap());
    assert_success!(publish(&mut h, &acc));

    assert_success!(run(&mut h, &acc, "store", vec![
        bcs::to_bytes(&10u64).unwrap()
    ]));
    assert_eq!(stored_format_version(&h, acc.address()), 2);
    run_all_stored_closure_ops(&mut h, &acc);
}

#[test]
fn function_value_to_bytes_version_follows_flag() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x66").unwrap());
    assert_success!(publish(&mut h, &acc));

    assert_success!(run(&mut h, &acc, "to_bytes_version", vec![bcs::to_bytes(
        &1u8
    )
    .unwrap()]));

    h.enable_features(vec![FeatureFlag::ENABLE_FUNCTION_DATA_FORMAT_V2], vec![]);
    assert_success!(run(&mut h, &acc, "to_bytes_version", vec![bcs::to_bytes(
        &2u8
    )
    .unwrap()]));
}

#[test]
fn function_value_v2_with_serialization_disabled() {
    use crate::assert_abort;
    use move_core_types::vm_status::sub_status::NFE_BCS_SERIALIZATION_FAILURE;

    let mut h = MoveHarness::new();
    h.enable_features(
        vec![
            FeatureFlag::DISABLE_CLOSURE_BCS_SERIALIZATION,
            FeatureFlag::ENABLE_FUNCTION_DATA_FORMAT_V2,
        ],
        vec![],
    );
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x66").unwrap());
    assert_success!(publish(&mut h, &acc));

    // Storage writes and stored-value operations work with both flags on; only
    // Move-observable serialization is disabled.
    assert_success!(run(&mut h, &acc, "store", vec![
        bcs::to_bytes(&10u64).unwrap()
    ]));
    assert_eq!(stored_format_version(&h, acc.address()), 2);
    run_all_stored_closure_ops(&mut h, &acc);
    let status = run(&mut h, &acc, "to_bytes_version", vec![
        bcs::to_bytes(&2u8).unwrap()
    ]);
    assert_abort!(status, NFE_BCS_SERIALIZATION_FAILURE);

    // Lifting the restriction after the format switch: serialization resumes and
    // produces V2 bytes.
    h.enable_features(vec![], vec![FeatureFlag::DISABLE_CLOSURE_BCS_SERIALIZATION]);
    assert_success!(run(&mut h, &acc, "to_bytes_version", vec![bcs::to_bytes(
        &2u8
    )
    .unwrap()]));
}
