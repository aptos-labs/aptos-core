// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests for `DISABLE_CLOSURE_BCS_SERIALIZATION`: BCS serialization of function
//! values fails while the flag is on; storage writes and execution are unaffected.

use crate::{assert_abort, assert_success, tests::common, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_language_e2e_tests::account::Account;
use aptos_package_builder::PackageBuilder;
use aptos_types::{
    account_address::AccountAddress,
    on_chain_config::FeatureFlag,
    transaction::{ExecutionStatus, TransactionStatus},
};
use move_core_types::vm_status::{sub_status::NFE_BCS_SERIALIZATION_FAILURE, AbortLocation};

const SOURCE: &str = r#"
module 0x66::m {
    use std::bcs;
    use std::option;
    use std::signer;
    use aptos_std::aptos_hash;
    use aptos_std::table;

    #[persistent]
    fun incr(x: u64, y: u64): u64 { x + y }

    struct FunTable has key { t: table::Table<|u64|u64 has copy+store+drop, u64> }
    struct ValTable has key { t: table::Table<u64, |u64|u64 has copy+store+drop> }
    struct Holder has key { f: |u64|u64 has copy+store+drop }

    fun captured(): |u64|u64 has copy+store+drop {
        |y| incr(10, y)
    }

    public entry fun to_bytes_captured() {
        let f = captured();
        let _ = bcs::to_bytes(&f);
    }

    public entry fun to_bytes_no_capture() {
        let f: |u64, u64|u64 has copy+store+drop = incr;
        let _ = bcs::to_bytes(&f);
    }

    public entry fun to_bytes_no_closure() {
        let o = option::none<|u64|u64 has copy+store+drop>();
        let _ = bcs::to_bytes(&o);
    }

    public entry fun serialized_size_closure() {
        let f = captured();
        let _ = bcs::serialized_size(&f);
    }

    public entry fun sip_hash_closure() {
        let f = captured();
        let _ = aptos_hash::sip_hash_from_value(&f);
    }

    public entry fun table_add_closure_key(account: &signer) {
        let t = table::new<|u64|u64 has copy+store+drop, u64>();
        table::add(&mut t, captured(), 1);
        move_to(account, FunTable { t })
    }

    public entry fun table_contains_closure_key(account: &signer) acquires FunTable {
        let t = &borrow_global<FunTable>(signer::address_of(account)).t;
        assert!(table::contains(t, captured()), 0);
    }

    public entry fun table_remove_closure_key(account: &signer) acquires FunTable {
        let t = &mut borrow_global_mut<FunTable>(signer::address_of(account)).t;
        let _ = table::remove(t, captured());
    }

    public entry fun table_closure_value(account: &signer) {
        let t = table::new<u64, |u64|u64 has copy+store+drop>();
        table::add(&mut t, 1, captured());
        move_to(account, ValTable { t })
    }

    public entry fun store_closure(account: &signer) {
        move_to(account, Holder { f: captured() })
    }

    public entry fun call_stored(account: &signer, x: u64, expected: u64) acquires Holder {
        let f = borrow_global<Holder>(signer::address_of(account)).f;
        assert!(f(x) == expected, 1);
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

#[test]
fn closure_serialization_works_while_flag_off() {
    let mut h = MoveHarness::new();
    // Disable explicitly, so the test does not depend on the default feature set.
    h.enable_features(vec![], vec![FeatureFlag::DISABLE_CLOSURE_BCS_SERIALIZATION]);
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x66").unwrap());
    assert_success!(publish(&mut h, &acc));

    for name in [
        "to_bytes_captured",
        "to_bytes_no_capture",
        "to_bytes_no_closure",
        "serialized_size_closure",
        "sip_hash_closure",
        "table_add_closure_key",
        "table_contains_closure_key",
        "table_remove_closure_key",
        "table_closure_value",
        "store_closure",
    ] {
        assert_success!(run(&mut h, &acc, name, vec![]), "{}", name);
    }
    assert_success!(run(&mut h, &acc, "call_stored", vec![
        bcs::to_bytes(&5u64).unwrap(),
        bcs::to_bytes(&15u64).unwrap(),
    ]));
}

#[test]
fn closure_serialization_fails_while_flag_on() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x66").unwrap());
    let other = h.new_account_at(AccountAddress::from_hex_literal("0x77").unwrap());
    assert_success!(publish(&mut h, &acc));

    // Populate state before the flag activates.
    assert_success!(run(&mut h, &acc, "table_add_closure_key", vec![]));
    assert_success!(run(&mut h, &acc, "store_closure", vec![]));

    h.enable_features(vec![FeatureFlag::DISABLE_CLOSURE_BCS_SERIALIZATION], vec![]);

    // Serialization of function values aborts.
    for name in [
        "to_bytes_captured",
        "to_bytes_no_capture",
        "serialized_size_closure",
        "sip_hash_closure",
    ] {
        let status = run(&mut h, &acc, name, vec![]);
        assert_abort!(status, NFE_BCS_SERIALIZATION_FAILURE);
    }

    // Table operations with closure keys fail.
    for name in [
        "table_contains_closure_key",
        "table_remove_closure_key",
        "table_add_closure_key",
    ] {
        let status = run(&mut h, &acc, name, vec![]);
        assert!(
            matches!(
                &status,
                TransactionStatus::Keep(ExecutionStatus::ExecutionFailure {
                    location: AbortLocation::Module(module),
                    ..
                }) if module.address() == &AccountAddress::ONE
                    && module.name().as_str() == "table"
            ),
            "{}: {:?}",
            name,
            status
        );
    }

    // Values without function values serialize as before.
    assert_success!(run(&mut h, &acc, "to_bytes_no_closure", vec![]));

    // Writes and execution of stored function values are unaffected.
    assert_success!(run(&mut h, &other, "store_closure", vec![]));
    assert_success!(run(&mut h, &other, "table_closure_value", vec![]));
    assert_success!(run(&mut h, &acc, "call_stored", vec![
        bcs::to_bytes(&5u64).unwrap(),
        bcs::to_bytes(&15u64).unwrap(),
    ]));
}
