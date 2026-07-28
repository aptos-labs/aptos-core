// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests for `aptos_framework::init::internal_maybe_initialize`.

use crate::{assert_abort, assert_success, tests::common, MoveHarness};
use aptos_framework::{natives::code::UpgradePolicy, BuildOptions, BuiltPackage};
use aptos_language_e2e_tests::account::Account;
use aptos_package_builder::PackageBuilder;
use aptos_types::{
    account_address::AccountAddress, object_address::create_object_code_deployment_address,
    on_chain_config::FeatureFlag, transaction::TransactionStatus,
};
use move_core_types::parser::parse_struct_tag;
use serde::{Deserialize, Serialize};

const ADDR: &str = "0xcafe";

/// A harness with lazy module initialization enabled (the feature is off by default).
fn new_harness() -> MoveHarness {
    MoveHarness::new_with_features(vec![FeatureFlag::LAZY_MODULE_INITIALIZATION], vec![])
}

#[derive(Serialize, Deserialize)]
struct Counter {
    value: u64,
}

// The abort code for error::invalid_argument(EINVALID_INITIALIZE_CALLER) where the
// constant is 0x1: (INVALID_ARGUMENT=1 << 16) | 1 = 0x10001.
const EINVALID_INITIALIZE_CALLER: u64 = 0x10001;

// The abort code for error::permission_denied(EOWNER_CHANGED_SINCE_DEPLOY) where the
// constant is 0x2: (PERMISSION_DENIED=5 << 16) | 2 = 0x50002.
const EOWNER_CHANGED_SINCE_DEPLOY: u64 = 0x50002;

// The abort code for error::invalid_state(ELAZY_MODULE_INITIALIZATION_NOT_ENABLED) where the
// constant is 0x3: (INVALID_STATE=3 << 16) | 3 = 0x30003.
const ELAZY_MODULE_INITIALIZATION_NOT_ENABLED: u64 = 0x30003;

fn make_module(only_once: bool, extra: &str) -> String {
    format!(
        r#"module 0xcafe::test {{
            use aptos_framework::init;
            struct Counter has key {{ value: u64 }}
            public entry fun run(_s: &signer) {{
                let s = init::internal_maybe_initialize({only_once});
                if (s.is_some()) {{
                    initialize(&s.destroy_some());
                }}
            }}
            fun initialize(s: &signer) {{
                if (exists<Counter>(@0xcafe)) {{
                    Counter[@0xcafe].value += 1;
                }} else {{
                    move_to(s, Counter {{ value: 1 }});
                }}
            }}
            {extra}
        }}"#
    )
}

fn publish(h: &mut MoveHarness, acc: &aptos_language_e2e_tests::account::Account, source: &str) {
    let mut builder = PackageBuilder::new("TestPack").with_policy(UpgradePolicy::compat());
    builder.add_local_dep(
        "AptosFramework",
        &common::framework_dir_path("aptos-framework").to_string_lossy(),
    );
    builder.add_source("test.move", source);
    let path = builder.write_to_temp().unwrap();
    let txn = h.create_publish_package(acc, path.path(), Some(BuildOptions::move_2()), |_| {});
    assert_success!(h.run(txn));
}

fn run(h: &mut MoveHarness, acc: &aptos_language_e2e_tests::account::Account) {
    assert_success!(h.run_entry_function(
        acc,
        str::parse("0xcafe::test::run").unwrap(),
        vec![],
        vec![],
    ));
}

fn counter(h: &MoveHarness, addr: AccountAddress) -> u64 {
    h.read_resource::<Counter>(&addr, parse_struct_tag("0xcafe::test::Counter").unwrap())
        .unwrap()
        .value
}

// -------------------------------------------------------------------
// Positive tests

#[test]
fn init_maybe_initialize_runs_on_first_deployment() {
    let mut h = new_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal(ADDR).unwrap());

    publish(&mut h, &acc, &make_module(false, ""));
    run(&mut h, &acc);

    assert_eq!(counter(&h, *acc.address()), 1);
}

#[test]
fn init_maybe_initialize_no_rerun_without_upgrade() {
    let mut h = new_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal(ADDR).unwrap());

    publish(&mut h, &acc, &make_module(false, ""));
    run(&mut h, &acc);
    run(&mut h, &acc); // second call — already initialized

    assert_eq!(counter(&h, *acc.address()), 1);
}

#[test]
fn init_maybe_initialize_reruns_after_upgrade() {
    let mut h = new_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal(ADDR).unwrap());

    publish(&mut h, &acc, &make_module(false, ""));
    run(&mut h, &acc);
    assert_eq!(counter(&h, *acc.address()), 1);

    // Upgrade: adds a new public function (compatible change).
    publish(&mut h, &acc, &make_module(false, "public fun v2() {}"));
    run(&mut h, &acc);

    // Upgrade reset the initialization state, so the initializer ran again.
    assert_eq!(counter(&h, *acc.address()), 2);
}

#[test]
fn init_maybe_initialize_only_once_survives_upgrade() {
    let mut h = new_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal(ADDR).unwrap());

    publish(&mut h, &acc, &make_module(true, ""));
    run(&mut h, &acc);
    assert_eq!(counter(&h, *acc.address()), 1);

    // Upgrade: the only_once flag means reset_initialized skips this module.
    publish(&mut h, &acc, &make_module(true, "public fun v2() {}"));
    run(&mut h, &acc);

    // Counter unchanged — initializer did not re-run.
    assert_eq!(counter(&h, *acc.address()), 1);
}

#[test]
fn init_maybe_initialize_only_once_flag_is_immutable() {
    let mut h = new_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal(ADDR).unwrap());

    publish(&mut h, &acc, &make_module(true, ""));
    run(&mut h, &acc);
    assert_eq!(counter(&h, *acc.address()), 1);

    // Upgrade to a version passing only_once = false. The stored `true` flag persists,
    // so the initializer must still not re-run.
    publish(&mut h, &acc, &make_module(false, "public fun v2() {}"));
    run(&mut h, &acc);

    assert_eq!(counter(&h, *acc.address()), 1);
}

#[test]
fn init_maybe_initialize_flag_upgradable_to_only_once() {
    let mut h = new_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal(ADDR).unwrap());

    publish(&mut h, &acc, &make_module(false, ""));
    run(&mut h, &acc);
    assert_eq!(counter(&h, *acc.address()), 1);

    // Upgrade to a version passing only_once = true. The stored `false` flag means the
    // upgrade resets the entry, so the initializer re-runs and now records `true`.
    publish(&mut h, &acc, &make_module(true, "public fun v2() {}"));
    run(&mut h, &acc);
    assert_eq!(counter(&h, *acc.address()), 2);

    // A further upgrade no longer resets: the recorded flag is now `true`.
    publish(
        &mut h,
        &acc,
        &make_module(true, "public fun v2() {} public fun v3() {}"),
    );
    run(&mut h, &acc);
    assert_eq!(counter(&h, *acc.address()), 2);
}

// -------------------------------------------------------------------
// Negative test

#[test]
fn init_maybe_initialize_script_caller_aborts() {
    let mut h = new_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal(ADDR).unwrap());

    // Scripts have no module identity, so initialization cannot be requested from them.
    let script = r#"script {
        use aptos_framework::init;
        fun main() {
            let _s = init::internal_maybe_initialize(false);
        }
    }"#;

    let mut builder = PackageBuilder::new("TestPack").with_policy(UpgradePolicy::compat());
    builder.add_local_dep(
        "AptosFramework",
        &common::framework_dir_path("aptos-framework").to_string_lossy(),
    );
    builder.add_source("main.move", script);
    let path = builder.write_to_temp().unwrap();
    let package = BuiltPackage::build(path.path().to_owned(), BuildOptions::move_2()).unwrap();
    let code = package.extract_script_code().pop().unwrap();

    let txn = h.create_script(&acc, code, vec![], vec![]);
    assert_abort!(h.run(txn), EINVALID_INITIALIZE_CALLER);
}

#[test]
fn init_maybe_initialize_aborts_when_feature_disabled() {
    // Feature off (note: not `new_harness`) -> the entry point must abort.
    let mut h =
        MoveHarness::new_with_features(vec![], vec![FeatureFlag::LAZY_MODULE_INITIALIZATION]);
    let acc = h.new_account_at(AccountAddress::from_hex_literal(ADDR).unwrap());

    publish(&mut h, &acc, &make_module(false, ""));
    assert_abort!(
        h.run_entry_function(
            &acc,
            str::parse("0xcafe::test::run").unwrap(),
            vec![],
            vec![]
        ),
        ELAZY_MODULE_INITIALIZATION_NOT_ENABLED
    );
}

// -------------------------------------------------------------------
// Object ownership guard
//
// A module deployed to an object has the object's address, and object ownership is
// transferable and separable from the code. Self-init is therefore only permitted
// while the object is still owned by whoever owned it when the code was published;
// otherwise the caller could mint a signer for an object it no longer controls.

/// The object address a code deployment by `acc` will land on next.
fn deploy_object_addr(h: &MoveHarness, acc: &Account) -> AccountAddress {
    let seq = h.sequence_number(acc.address());
    create_object_code_deployment_address(*acc.address(), seq + 1)
}

/// A module deployed to a code object that lazily self-initializes via
/// `init::internal_maybe_initialize`. Its address is the named address `object`, bound to the
/// code object's address at build time (see `deploy_to_object`).
const OBJECT_MODULE_SRC: &str = r#"module object::test {
    use aptos_framework::init;

    struct Counter has key { value: u64 }

    public entry fun run(_s: &signer) {
        let s = init::internal_maybe_initialize(false);
        if (s.is_some()) {
            move_to(&s.destroy_some(), Counter { value: 1 });
        } else {
            s.destroy_none();
        }
    }
}"#;

/// Deploys `OBJECT_MODULE_SRC` (module `object::test`) to `obj` via object code deployment.
fn deploy_to_object(h: &mut MoveHarness, acc: &Account, obj: AccountAddress) -> TransactionStatus {
    let mut builder = PackageBuilder::new("ObjectPack").with_policy(UpgradePolicy::compat());
    builder.add_local_dep(
        "AptosFramework",
        &common::framework_dir_path("aptos-framework").to_string_lossy(),
    );
    builder.add_source("test.move", OBJECT_MODULE_SRC);
    let path = builder.write_to_temp().unwrap();

    let mut options = BuildOptions::move_2();
    options.named_addresses.insert("object".to_string(), obj);
    h.object_code_deployment_package(acc, path.path(), options)
}

fn run_object(h: &mut MoveHarness, caller: &Account, obj: AccountAddress) -> TransactionStatus {
    h.run_entry_function(
        caller,
        str::parse(&format!("{}::test::run", obj)).unwrap(),
        vec![],
        vec![],
    )
}

#[test]
fn init_maybe_initialize_object_owner_unchanged_runs() {
    let mut h = new_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal(ADDR).unwrap());
    let obj = deploy_object_addr(&h, &acc);

    assert_success!(deploy_to_object(&mut h, &acc, obj));

    // Owner unchanged since deploy -> self-init is allowed.
    assert_success!(run_object(&mut h, &acc, obj));

    let value = h
        .read_resource::<Counter>(
            &obj,
            parse_struct_tag(&format!("{}::test::Counter", obj)).unwrap(),
        )
        .unwrap()
        .value;
    assert_eq!(value, 1);
}

#[test]
fn init_maybe_initialize_object_owner_changed_aborts() {
    let mut h = new_harness();
    let attacker = h.new_account_at(AccountAddress::from_hex_literal(ADDR).unwrap());
    let victim = h.new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap());
    let obj = deploy_object_addr(&h, &attacker);

    assert_success!(deploy_to_object(&mut h, &attacker, obj));

    // The attack: publish -> transfer the code object to the victim -> self-init.
    assert_success!(h.run_entry_function(
        &attacker,
        str::parse("0x1::object::transfer_call").unwrap(),
        vec![],
        vec![
            bcs::to_bytes(&obj).unwrap(),
            bcs::to_bytes(victim.address()).unwrap(),
        ],
    ));

    // Ownership changed since deploy -> self-init must abort (no signer minted).
    assert_abort!(
        run_object(&mut h, &attacker, obj),
        EOWNER_CHANGED_SINCE_DEPLOY
    );
}

// -------------------------------------------------------------------
// Account key rotation does not block self-init
//
// A module hosted on an account authorizes its own code by publishing it. Rotating the account's
// authentication key does not change the principal that owns the account, so -- unlike an object
// ownership transfer, which hands the address to a different owner -- it must not block self-init.

/// Rotates `acc`'s authentication key to a fixed dummy 32-byte key.
fn rotate_auth_key(h: &mut MoveHarness, acc: &Account) -> TransactionStatus {
    h.run_entry_function(
        acc,
        str::parse("0x1::account::rotate_authentication_key_call").unwrap(),
        vec![],
        vec![bcs::to_bytes(&vec![7u8; 32]).unwrap()],
    )
}

#[test]
fn init_maybe_initialize_account_self_init_survives_key_rotation() {
    let mut h = new_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal(ADDR).unwrap());
    let caller = h.new_account_at(AccountAddress::from_hex_literal("0xd00d").unwrap());

    // Publish uninitialized, then rotate the account's key before the first init.
    publish(&mut h, &acc, &make_module(false, ""));
    assert_success!(rotate_auth_key(&mut h, &acc));

    // Rotation is not an ownership handoff for an account, so self-init still succeeds. Invoke
    // from a different caller, since `acc` can no longer sign after the rotation.
    assert_success!(h.run_entry_function(
        &caller,
        str::parse("0xcafe::test::run").unwrap(),
        vec![],
        vec![],
    ));
    assert_eq!(counter(&h, *acc.address()), 1);
}
