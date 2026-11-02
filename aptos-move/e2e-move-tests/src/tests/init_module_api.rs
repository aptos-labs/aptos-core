// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests for `aptos_framework::init::internal_maybe_initialize`.

use crate::{assert_abort, assert_success, assert_vm_status, tests::common, MoveHarness};
use aptos_crypto::ed25519::Ed25519Signature;
use aptos_framework::{natives::code::UpgradePolicy, BuildOptions, BuiltPackage};
use aptos_language_e2e_tests::account::Account;
use aptos_package_builder::PackageBuilder;
use aptos_types::{
    account_address::AccountAddress,
    error,
    object_address::create_object_code_deployment_address,
    on_chain_config::FeatureFlag,
    transaction::{SignedTransaction, TransactionStatus},
};
use aptos_vm::AptosSimulationVM;
use move_core_types::{parser::parse_struct_tag, vm_status::StatusCode};
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

const EOWNER_CHANGED_SINCE_DEPLOY: u64 = error::permission_denied(2);

const ELAZY_MODULE_INITIALIZATION_NOT_ENABLED: u64 = error::invalid_state(3);

const EINVALID_INITIALIZE_CALLER: u64 = error::invalid_argument(1);

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
fn init_maybe_initialize_upgrade_while_disabled() {
    for only_once in [false, true] {
        let mut h = new_harness();
        let acc = h.new_account_at(AccountAddress::from_hex_literal(ADDR).unwrap());
        publish(&mut h, &acc, &make_module(only_once, ""));
        run(&mut h, &acc);
        assert_eq!(counter(&h, *acc.address()), 1);

        h.enable_features(vec![], vec![FeatureFlag::LAZY_MODULE_INITIALIZATION]);
        publish(&mut h, &acc, &make_module(only_once, "public fun v2() {}"));
        h.enable_features(vec![FeatureFlag::LAZY_MODULE_INITIALIZATION], vec![]);

        run(&mut h, &acc);
        assert_eq!(counter(&h, *acc.address()), if only_once { 1 } else { 2 });
        run(&mut h, &acc);
        assert_eq!(counter(&h, *acc.address()), if only_once { 1 } else { 2 });
    }
}

#[test]
fn init_maybe_initialize_mixed_bundle_resets_before_legacy_init() {
    for previously_initialized in [false, true] {
        let mut h = new_harness();
        let acc = h.new_account_at(AccountAddress::from_hex_literal(ADDR).unwrap());
        publish(&mut h, &acc, &make_module(false, ""));
        if previously_initialized {
            run(&mut h, &acc);
        }

        let mut builder = PackageBuilder::new("TestPack").with_policy(UpgradePolicy::compat());
        builder.add_local_dep(
            "AptosFramework",
            &common::framework_dir_path("aptos-framework").to_string_lossy(),
        );
        builder.add_source("test.move", &make_module(false, "public fun v2() {}"));
        builder.add_source(
            "sibling.move",
            "module 0xcafe::sibling { fun init_module(s: &signer) { 0xcafe::test::run(s); } }",
        );
        let path = builder.write_to_temp().unwrap();
        let txn = h.create_publish_package(&acc, path.path(), Some(BuildOptions::move_2()), |_| {});
        assert_success!(h.run(txn));

        let expected = if previously_initialized { 2 } else { 1 };
        assert_eq!(counter(&h, *acc.address()), expected);
        run(&mut h, &acc);
        assert_eq!(counter(&h, *acc.address()), expected);
    }
}

#[test]
fn init_module_remains_supported_with_lazy_initialization() {
    let mut h = new_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal(ADDR).unwrap());
    publish(
        &mut h,
        &acc,
        "module 0xcafe::test {
            struct Counter has key { value: u64 }
            fun init_module(s: &signer) { move_to(s, Counter { value: 1 }); }
        }",
    );
    assert_eq!(counter(&h, *acc.address()), 1);
}

#[test_case::test_case(false; "eager_allowed")]
#[test_case::test_case(true; "eager_disabled")]
fn init_module_not_run_at_publish_with_lazy_initialization(disable_eager: bool) {
    let mut h = new_harness();
    if disable_eager {
        h.enable_features(
            vec![FeatureFlag::DISABLE_EAGER_MODULE_INITIALIZATION],
            vec![],
        );
    } else {
        h.enable_features(vec![], vec![
            FeatureFlag::DISABLE_EAGER_MODULE_INITIALIZATION,
        ]);
    }
    let acc = h.new_account_at(AccountAddress::from_hex_literal(ADDR).unwrap());
    publish(
        &mut h,
        &acc,
        "module 0xcafe::test {
            use aptos_framework::init;
            struct Counter has key { value: u64 }
            public entry fun run(_s: &signer) {
                let s = init::internal_maybe_initialize(true);
                if (s.is_some()) { init_module(&s.destroy_some()) }
            }
            fun init_module(s: &signer) {
                if (exists<Counter>(@0xcafe)) {
                    Counter[@0xcafe].value += 1;
                } else {
                    move_to(s, Counter { value: 1 });
                }
            }
        }",
    );
    assert!(h
        .read_resource::<Counter>(
            acc.address(),
            parse_struct_tag("0xcafe::test::Counter").unwrap()
        )
        .is_none());

    run(&mut h, &acc);
    assert_eq!(counter(&h, *acc.address()), 1);
    run(&mut h, &acc);
    assert_eq!(counter(&h, *acc.address()), 1);
}

#[test_case::test_case(false; "lazy_disabled")]
#[test_case::test_case(true; "lazy_enabled")]
fn init_module_eager_initialization_disabled(lazy_enabled: bool) {
    let mut h = new_harness();
    h.enable_features(
        vec![FeatureFlag::DISABLE_EAGER_MODULE_INITIALIZATION],
        vec![],
    );
    if !lazy_enabled {
        h.enable_features(vec![], vec![FeatureFlag::LAZY_MODULE_INITIALIZATION]);
    }
    let acc = h.new_account_at(AccountAddress::from_hex_literal(ADDR).unwrap());
    let mut builder = PackageBuilder::new("TestPack").with_policy(UpgradePolicy::compat());
    builder.add_source(
        "test.move",
        "module 0xcafe::test {
            struct Counter has key { value: u64 }
            fun init_module(s: &signer) { move_to(s, Counter { value: 1 }); }
        }",
    );
    let path = builder.write_to_temp().unwrap();
    let package = BuiltPackage::build(path.path().to_owned(), BuildOptions::move_2()).unwrap();
    let txn = h.create_publish_built_package(&acc, &package, |_| {});
    // Simulation exposes the full VM error message returned to clients.
    let simulation_txn = SignedTransaction::new(
        txn.clone().into_raw_transaction(),
        acc.pubkey.as_ed25519().unwrap(),
        Ed25519Signature::dummy_signature(),
    );
    let (error, _) = AptosSimulationVM::create_vm_and_simulate_signed_transaction(
        &simulation_txn,
        h.executor.get_state_view(),
    );
    assert_eq!(error.status_code(), StatusCode::INVALID_INIT_MODULE);
    assert_eq!(
        error.message().map(String::as_str),
        Some(
            "Eager module initialization is disabled for 0xcafe::test. Use a newer compiler \
             version with lazy initialization support for init_module."
        ),
    );
    assert_vm_status!(h.run(txn), StatusCode::INVALID_INIT_MODULE);
    assert!(h
        .read_resource::<Counter>(
            acc.address(),
            parse_struct_tag("0xcafe::test::Counter").unwrap()
        )
        .is_none());

    // Disabling the flag restores eager initialization. A successful retry also proves
    // that the rejected publish did not leave the module or package behind.
    h.enable_features(vec![], vec![
        FeatureFlag::DISABLE_EAGER_MODULE_INITIALIZATION,
    ]);
    let txn = h.create_publish_built_package(&acc, &package, |_| {});
    assert_success!(h.run(txn));
    assert_eq!(counter(&h, *acc.address()), 1);

    // Upgrading an existing legacy module does not run eager initialization and remains allowed.
    h.enable_features(
        vec![FeatureFlag::DISABLE_EAGER_MODULE_INITIALIZATION],
        vec![],
    );
    let txn = h.create_publish_built_package(&acc, &package, |_| {});
    assert_success!(h.run(txn));
    assert_eq!(counter(&h, *acc.address()), 1);
}

#[test]
fn init_module_eager_disabled_allows_module_without_initializer() {
    let mut h = new_harness();
    h.enable_features(
        vec![FeatureFlag::DISABLE_EAGER_MODULE_INITIALIZATION],
        vec![],
    );
    let acc = h.new_account_at(AccountAddress::from_hex_literal(ADDR).unwrap());
    publish(
        &mut h,
        &acc,
        "module 0xcafe::test { public entry fun run(_s: &signer) {} }",
    );
    run(&mut h, &acc);
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
fn init_maybe_initialize_script_caller_rejected() {
    let mut h = new_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal(ADDR).unwrap());

    // Scripts have no module identity, so any reference to the function is rejected up front.
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
    assert_vm_status!(h.run(txn), StatusCode::INVALID_OPERATION_IN_SCRIPT);
}

#[test]
fn init_maybe_initialize_closure_rejected_at_publish() {
    let mut h = new_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal(ADDR).unwrap());

    // A function value over `internal_maybe_initialize`, invoked by another module, would
    // initialize (and mint the signer of) that module. Packing one is rejected at publish.
    let source = r#"module 0xcafe::test {
        use std::option::Option;
        use aptos_framework::init;
        public fun make(): |bool|Option<signer> has drop {
            |only_once| init::internal_maybe_initialize(only_once)
        }
    }"#;

    let mut builder = PackageBuilder::new("TestPack").with_policy(UpgradePolicy::compat());
    builder.add_local_dep(
        "AptosFramework",
        &common::framework_dir_path("aptos-framework").to_string_lossy(),
    );
    builder.add_source("test.move", source);
    let path = builder.write_to_temp().unwrap();
    let status = h.publish_package_with_options(&acc, path.path(), BuildOptions::move_2());
    assert_vm_status!(status, StatusCode::CLOSURE_OVER_RESTRICTED_FUNCTION);
}

#[test_case::test_case(false, false; "legacy_bytecode")]
#[test_case::test_case(false, true; "legacy_stored_value")]
#[test_case::test_case(true, false; "mono_bytecode")]
fn init_maybe_initialize_preexisting_closure_rejected(mono_vm: bool, persisted: bool) {
    let mut h = MoveHarness::new_with_features(
        if mono_vm {
            vec![FeatureFlag::ENABLE_MONO_MOVE]
        } else {
            vec![]
        },
        if mono_vm {
            vec![FeatureFlag::LAZY_MODULE_INITIALIZATION]
        } else {
            vec![
                FeatureFlag::LAZY_MODULE_INITIALIZATION,
                FeatureFlag::ENABLE_MONO_MOVE,
            ]
        },
    );
    let acc = h.new_account_at(AccountAddress::from_hex_literal(ADDR).unwrap());
    let mut builder = PackageBuilder::new("StoredInitializer");
    builder.add_local_dep(
        "AptosFramework",
        &common::framework_dir_path("aptos-framework").to_string_lossy(),
    );
    builder.add_source(
        "stored.move",
        r#"module 0xcafe::stored {
            use std::option::Option;
            use std::signer;
            use aptos_framework::init;
            struct Saved has key {
                callback: |bool|Option<signer> has copy + drop + store,
            }
            public entry fun save(s: &signer) {
                move_to(s, Saved { callback: |once| init::internal_maybe_initialize(once) });
            }
            public entry fun attack() {
                let Saved { callback } = move_from<Saved>(@0xcafe);
                let forged = 0xbeef::victim::invoke(callback).destroy_some();
                assert!(signer::address_of(&forged) == @0xbeef, 100);
            }
            public entry fun attack_fresh() {
                let forged = 0xbeef::victim::invoke(
                    |once| init::internal_maybe_initialize(once)
                ).destroy_some();
                assert!(signer::address_of(&forged) == @0xbeef, 100);
            }
            public entry fun direct() {
                let own = init::internal_maybe_initialize(false).destroy_some();
                assert!(signer::address_of(&own) == @0xcafe, 101);
            }
        }
        module 0xbeef::victim {
            use std::option::Option;
            public fun invoke(callback: |bool|Option<signer> has drop): Option<signer> {
                callback(false)
            }
        }"#,
    );
    let path = builder.write_to_temp().unwrap();
    let package = BuiltPackage::build(path.path().to_owned(), BuildOptions::move_2()).unwrap();
    // Model bytecode accepted by the old publisher. The current publication validator rejects
    // it, but upgrades must also protect existing code and function values already in storage.
    for code in package.extract_code() {
        let module = move_binary_format::CompiledModule::deserialize(&code).unwrap();
        h.executor.add_module(&module.self_id(), code);
    }
    if persisted {
        assert_success!(h.run_entry_function(
            &acc,
            str::parse("0xcafe::stored::save").unwrap(),
            vec![],
            vec![],
        ));
    }
    h.enable_features(vec![FeatureFlag::LAZY_MODULE_INITIALIZATION], vec![]);
    let entry = if persisted { "attack" } else { "attack_fresh" };
    let status = h.run_entry_function(
        &acc,
        str::parse(&format!("0xcafe::stored::{entry}")).unwrap(),
        vec![],
        vec![],
    );
    assert!(
        matches!(status, TransactionStatus::Keep(
            aptos_types::transaction::ExecutionStatus::MoveAbort { code, .. }
        ) if code == EINVALID_INITIALIZE_CALLER),
        "{status:?}",
    );
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::stored::direct").unwrap(),
        vec![],
        vec![],
    ));
}

#[test]
fn init_maybe_initialize_not_resolvable_by_reflection() {
    let mut h = MoveHarness::new_with_features(
        vec![
            FeatureFlag::LAZY_MODULE_INITIALIZATION,
            FeatureFlag::ENABLE_FUNCTION_REFLECTION,
        ],
        vec![],
    );
    let acc = h.new_account_at(AccountAddress::from_hex_literal(ADDR).unwrap());

    // Reflection must refuse the function for the same reason closures over it are rejected.
    let source = r#"module 0xcafe::test {
        use std::option::Option;
        use std::reflect;
        use std::string::utf8;
        const EFUNCTION_NOT_ACCESSIBLE: u64 = 2;
        public entry fun run() {
            let r = reflect::resolve<|bool|Option<signer>>(
                @0x1, &utf8(b"init"), &utf8(b"internal_maybe_initialize"));
            assert!(r.is_err(), 100);
            assert!(r.unwrap_err().error_code() == EFUNCTION_NOT_ACCESSIBLE, 101);
        }
    }"#;
    publish(&mut h, &acc, source);
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::test::run").unwrap(),
        vec![],
        vec![],
    ));
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
/// `init::internal_maybe_initialize`, extended by `extra`. Its address is the named address
/// `object`, bound to the code object's address at build time (see `deploy_to_object`).
fn object_module_src(extra: &str) -> String {
    format!(
        r#"module object::test {{
            use aptos_framework::init;

            struct Counter has key {{ value: u64 }}

            public entry fun run(_s: &signer) {{
                let s = init::internal_maybe_initialize(false);
                if (s.is_some()) {{
                    move_to(&s.destroy_some(), Counter {{ value: 1 }});
                }} else {{
                    s.destroy_none();
                }}
            }}
            {extra}
        }}"#
    )
}

/// Deploys `object_module_src("")` (module `object::test`) to `obj` via object code deployment.
fn deploy_to_object(h: &mut MoveHarness, acc: &Account, obj: AccountAddress) -> TransactionStatus {
    let mut builder = PackageBuilder::new("ObjectPack").with_policy(UpgradePolicy::compat());
    builder.add_local_dep(
        "AptosFramework",
        &common::framework_dir_path("aptos-framework").to_string_lossy(),
    );
    builder.add_source("test.move", &object_module_src(""));
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

// -------------------------------------------------------------------
// Stale code object signer
//
// The deploy owner is recorded only where the owner's signer is checked (object code deployment).
// A code object signer obtained while owning the object stays valid after a transfer, but
// publishing with it cannot refresh the record, so the transferred module stays blocked.

/// Builds `object_module_src(extra)` for the object at `obj`, returning the serialized package
/// metadata and the module code as `code::publish_package_txn` expects them.
fn object_package_bytes(obj: AccountAddress, extra: &str) -> (Vec<u8>, Vec<Vec<u8>>) {
    let mut builder = PackageBuilder::new("ObjectPack").with_policy(UpgradePolicy::compat());
    builder.add_local_dep(
        "AptosFramework",
        &common::framework_dir_path("aptos-framework").to_string_lossy(),
    );
    builder.add_source("test.move", &object_module_src(extra));
    let path = builder.write_to_temp().unwrap();

    let mut options = BuildOptions::move_2();
    options.named_addresses.insert("object".to_string(), obj);
    let package = BuiltPackage::build(path.path().to_owned(), options).unwrap();
    let metadata = bcs::to_bytes(&package.extract_metadata().unwrap()).unwrap();
    (metadata, package.extract_code())
}

#[test]
fn init_maybe_initialize_stale_object_signer_cannot_refresh_owner() {
    let mut h = new_harness();
    let attacker = h.new_account_at(AccountAddress::from_hex_literal(ADDR).unwrap());
    let victim = h.new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap());

    // Takes the code object signer while owning the object, transfers the object, then publishes
    // with the stale signer.
    let attack = r#"module 0xcafe::attack {
        use aptos_framework::code::{Self, PackageRegistry};
        use aptos_framework::object::{Self, Object};
        use aptos_framework::object_code_deployment;
        public entry fun publish_after_transfer(
            attacker: &signer,
            code_object: Object<PackageRegistry>,
            victim: address,
            metadata: vector<u8>,
            code: vector<vector<u8>>,
        ) {
            let s = object_code_deployment::get_code_object_signer(attacker, code_object);
            object::transfer(attacker, code_object, victim);
            code::publish_package_txn(&s, metadata, code);
        }
    }"#;
    publish(&mut h, &attacker, attack);

    let obj = deploy_object_addr(&h, &attacker);
    assert_success!(deploy_to_object(&mut h, &attacker, obj));

    let (metadata, code) = object_package_bytes(obj, "public fun v2() {}");
    assert_success!(h.run_entry_function(
        &attacker,
        str::parse("0xcafe::attack::publish_after_transfer").unwrap(),
        vec![],
        vec![
            bcs::to_bytes(&obj).unwrap(),
            bcs::to_bytes(victim.address()).unwrap(),
            bcs::to_bytes(&metadata).unwrap(),
            bcs::to_bytes(&code).unwrap(),
        ],
    ));

    // The record still names the attacker, so the module cannot self-init under the victim.
    assert_abort!(
        run_object(&mut h, &attacker, obj),
        EOWNER_CHANGED_SINCE_DEPLOY
    );
}

// -------------------------------------------------------------------
// Upgrade reset timing
//
// `code::publish_package` only requests the publish; the new code goes live after the
// transaction's Move execution. The initialization reset must therefore happen after that too,
// or the old code could consume it in the publishing transaction.

/// A module whose entry point `upgrade_and_run` publishes `TestPack` from within the module and
/// then runs the lazy-init preamble while the old code is still live. The initializer adds `inc`.
fn make_self_upgrading_module(only_once: bool, inc: u64) -> String {
    format!(
        r#"module 0xcafe::test {{
            use aptos_framework::code;
            use aptos_framework::init;
            struct Counter has key {{ value: u64 }}
            public entry fun run(_s: &signer) {{
                maybe_initialize();
            }}
            public entry fun upgrade_and_run(s: &signer, metadata: vector<u8>, code: vector<vector<u8>>) {{
                code::publish_package_txn(s, metadata, code);
                maybe_initialize();
            }}
            fun maybe_initialize() {{
                let s = init::internal_maybe_initialize({only_once});
                if (s.is_some()) {{
                    initialize(&s.destroy_some());
                }}
            }}
            fun initialize(s: &signer) {{
                if (exists<Counter>(@0xcafe)) {{
                    Counter[@0xcafe].value += {inc};
                }} else {{
                    move_to(s, Counter {{ value: {inc} }});
                }}
            }}
        }}"#
    )
}

/// Builds `source` as `TestPack` at `0xcafe`, returning serialized metadata and module code.
fn package_bytes(source: &str) -> (Vec<u8>, Vec<Vec<u8>>) {
    let mut builder = PackageBuilder::new("TestPack").with_policy(UpgradePolicy::compat());
    builder.add_local_dep(
        "AptosFramework",
        &common::framework_dir_path("aptos-framework").to_string_lossy(),
    );
    builder.add_source("test.move", source);
    let path = builder.write_to_temp().unwrap();
    let package = BuiltPackage::build(path.path().to_owned(), BuildOptions::move_2()).unwrap();
    let metadata = bcs::to_bytes(&package.extract_metadata().unwrap()).unwrap();
    (metadata, package.extract_code())
}

#[test]
fn init_maybe_initialize_self_upgrade_cannot_consume_reset() {
    let mut h = new_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal(ADDR).unwrap());

    publish(&mut h, &acc, &make_self_upgrading_module(false, 1));
    run(&mut h, &acc);
    assert_eq!(counter(&h, *acc.address()), 1);

    // v1 upgrades itself to v2 and re-enters the preamble with `only_once = true` before v2 is
    // live. It must find the module still initialized: no re-init, and no `true` flag recorded.
    let (metadata, code) = package_bytes(&make_self_upgrading_module(true, 10));
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::test::upgrade_and_run").unwrap(),
        vec![],
        vec![
            bcs::to_bytes(&metadata).unwrap(),
            bcs::to_bytes(&code).unwrap()
        ],
    ));
    assert_eq!(counter(&h, *acc.address()), 1);

    // The reset took effect once v2 was live, so v2's initializer runs on the next call.
    run(&mut h, &acc);
    assert_eq!(counter(&h, *acc.address()), 11);
}
