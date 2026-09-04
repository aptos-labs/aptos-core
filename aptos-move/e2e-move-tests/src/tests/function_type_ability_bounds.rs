// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! End-to-end tests for function-type ability validation.
//!
//! Function-type ability sets may contain only `copy`, `drop`, and `store`. With
//! `FeatureFlag::CHECK_FUNCTION_TYPE_ABILITIES` enabled, `TypeBuilder::create_ty` rejects
//! `TypeTag`s containing `key` or undefined bits; while disabled, the raw bits are retained for
//! deterministic replay.

use crate::{assert_success, assert_vm_status, tests::common, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_package_builder::PackageBuilder;
use aptos_transaction_simulation::Account;
use aptos_types::{
    account_address::AccountAddress,
    on_chain_config::FeatureFlag,
    transaction::{EntryFunction, TransactionPayload, TransactionStatus},
};
use move_core_types::{
    ability::AbilitySet,
    identifier::Identifier,
    language_storage::{FunctionTag, ModuleId, StructTag, TypeTag},
    vm_status::StatusCode,
};
use once_cell::sync::Lazy;
use serde::Deserialize;
use tempfile::TempDir;

/// `copy + drop + store`, i.e. `AbilitySet::PUBLIC_FUNCTIONS`.
const LEGAL_ABILITY_BYTE: u8 = 0x07;

/// Representative subsets of `AbilitySet::PUBLIC_FUNCTIONS`.
const LEGAL_ABILITY_BYTES: [u8; 6] = [0x00, 0x01, 0x02, 0x03, 0x04, 0x07];

/// `copy + drop + store` plus bit `0x10`, which is not assigned to any ability. Rejected by
/// `AbilitySet::from_u8`, so it is unreachable through module bytecode but not through a `TypeTag`.
const UNKNOWN_BIT_ABILITY_BYTE: u8 = 0x17;

/// `copy + drop + store + key`. Accepted by `AbilitySet::from_u8`, so validating the ability
/// *encoding* would not reject it, yet no function value can ever carry `key`.
const FAKE_KEY_ABILITY_BYTE: u8 = 0x0F;

const MODULE_ADDRESS: &str = "0xcafe";
const MODULE_NAME: &str = "fn_ability_bounds";

const MODULE_SOURCE: &str = r#"
module 0xcafe::fn_ability_bounds {
    use std::signer;
    use std::string::String;
    use std::vector;
    use aptos_std::type_info;
    use aptos_framework::object;

    /// Canonical type names recorded in call order.
    struct Observed has key {
        type_arg_names: vector<String>,
    }

    /// Stores one resource per `T` without requiring a value of `T`.
    struct Holder<phantom T> has key {
        value: u64,
    }

    /// Constrains `T` to `key`, which no function type can legitimately satisfy.
    struct KeyBound<phantom T: key> has key {
        value: u64,
    }

    fun init_module(publisher: &signer) {
        move_to(publisher, Observed { type_arg_names: vector::empty() });
    }

    /// Instantiable at any `T`, so it isolates the type-argument construction step.
    public entry fun observe<T>(caller: &signer, value: u64) acquires Observed {
        let observed = borrow_global_mut<Observed>(@0xcafe);
        vector::push_back(&mut observed.type_arg_names, type_info::type_name<T>());
        move_to(caller, Holder<T> { value });
    }

    /// Stores a resource parameterized by a `T: key` type argument.
    public entry fun make_object<T: key>(caller: &signer, value: u64) {
        let constructor_ref = object::create_object(signer::address_of(caller));
        let object_signer = object::generate_signer(&constructor_ref);
        move_to(&object_signer, KeyBound<T> { value });
    }
}
"#;

#[derive(Deserialize)]
struct Observed {
    type_arg_names: Vec<String>,
}

/// Reuses one package path because `MoveHarness` caches compiled packages by path.
static PACKAGE: Lazy<TempDir> = Lazy::new(|| {
    let mut builder = PackageBuilder::new("FnAbilityBounds");
    builder.add_source("fn_ability_bounds.move", MODULE_SOURCE);
    for (name, path) in [
        ("MoveStdlib", "move-stdlib"),
        ("AptosStdlib", "aptos-stdlib"),
        ("AptosFramework", "aptos-framework"),
    ] {
        builder.add_local_dep(name, &common::framework_dir_path(path).to_string_lossy());
    }
    builder.write_to_temp().unwrap()
});

fn module_address() -> AccountAddress {
    AccountAddress::from_hex_literal(MODULE_ADDRESS).unwrap()
}

fn publish() -> (MoveHarness, Account) {
    let mut harness = MoveHarness::new();
    let account = harness.new_account_at(module_address());
    let status = harness.publish_package_with_options(
        &account,
        PACKAGE.path(),
        BuildOptions::move_2().set_latest_language(),
    );
    assert_success!(status);
    (harness, account)
}

/// Builds a nullary function `TypeTag` with the exact raw ability byte. BCS decoding is used because
/// `AbilitySet::from_u8` rejects bytes containing undefined bits.
fn function_type_tag(ability_byte: u8) -> TypeTag {
    let abilities: AbilitySet =
        bcs::from_bytes(&[ability_byte]).expect("AbilitySet BCS decoding accepts any byte");
    TypeTag::Function(Box::new(FunctionTag {
        args: vec![],
        results: vec![],
        abilities,
    }))
}

fn holder_struct_tag(ability_byte: u8) -> StructTag {
    StructTag {
        address: module_address(),
        module: Identifier::new(MODULE_NAME).unwrap(),
        name: Identifier::new("Holder").unwrap(),
        type_args: vec![function_type_tag(ability_byte)],
    }
}

fn observed_struct_tag() -> StructTag {
    StructTag {
        address: module_address(),
        module: Identifier::new(MODULE_NAME).unwrap(),
        name: Identifier::new("Observed").unwrap(),
        type_args: vec![],
    }
}

fn entry_function_payload(name: &str, ty_args: Vec<TypeTag>, value: u64) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(module_address(), Identifier::new(MODULE_NAME).unwrap()),
        Identifier::new(name).unwrap(),
        ty_args,
        vec![bcs::to_bytes(&value).unwrap()],
    ))
}

fn run_with_ability_byte(
    harness: &mut MoveHarness,
    account: &Account,
    entry_function_name: &str,
    ability_byte: u8,
    value: u64,
) -> TransactionStatus {
    let payload = entry_function_payload(
        entry_function_name,
        vec![function_type_tag(ability_byte)],
        value,
    );
    harness.run_transaction_payload(account, payload)
}

/// Rejects excess function abilities during type-argument construction, before entry-function
/// execution.
#[test]
fn excess_function_type_abilities_are_rejected() {
    assert!(
        AbilitySet::from_u8(UNKNOWN_BIT_ABILITY_BYTE).is_none(),
        "{UNKNOWN_BIT_ABILITY_BYTE:#04x} must be rejected by the only validating constructor"
    );
    assert!(
        AbilitySet::from_u8(FAKE_KEY_ABILITY_BYTE).is_some(),
        "{FAKE_KEY_ABILITY_BYTE:#04x} is a well-formed ability set, so encoding checks miss it"
    );

    let (mut harness, account) = publish();

    for ability_byte in [UNKNOWN_BIT_ABILITY_BYTE, FAKE_KEY_ABILITY_BYTE, 0xFF] {
        let status = run_with_ability_byte(&mut harness, &account, "observe", ability_byte, 7);
        assert_vm_status!(status, StatusCode::CONSTRAINT_NOT_SATISFIED);
    }

    // Rejected type arguments leave the module state unchanged.
    let observed = harness
        .read_resource::<Observed>(account.address(), observed_struct_tag())
        .expect("Observed resource must exist");
    assert!(observed.type_arg_names.is_empty());
    for ability_byte in [UNKNOWN_BIT_ABILITY_BYTE, FAKE_KEY_ABILITY_BYTE] {
        assert_eq!(
            harness.read_resource::<u64>(account.address(), holder_struct_tag(ability_byte)),
            None
        );
    }
}

/// Accepts representative subsets of `AbilitySet::PUBLIC_FUNCTIONS`.
#[test]
fn legal_function_type_abilities_are_still_accepted() {
    let (mut harness, account) = publish();

    for (index, ability_byte) in LEGAL_ABILITY_BYTES.into_iter().enumerate() {
        let status = run_with_ability_byte(
            &mut harness,
            &account,
            "observe",
            ability_byte,
            index as u64,
        );
        assert_success!(
            status,
            "ability byte {ability_byte:#04x} is legal and must still be accepted"
        );
        assert_eq!(
            harness.read_resource::<u64>(account.address(), holder_struct_tag(ability_byte)),
            Some(index as u64)
        );
    }

    let observed = harness
        .read_resource::<Observed>(account.address(), observed_struct_tag())
        .expect("Observed resource must exist");
    assert_eq!(observed.type_arg_names.len(), LEGAL_ABILITY_BYTES.len());
}

/// Rejects a well-formed `0x0f` ability set before its `key` bit can satisfy `T: key`.
#[test]
fn fake_key_no_longer_satisfies_a_key_constraint() {
    let (mut harness, account) = publish();

    let status = run_with_ability_byte(
        &mut harness,
        &account,
        "make_object",
        FAKE_KEY_ABILITY_BYTE,
        1,
    );
    assert_vm_status!(status, StatusCode::CONSTRAINT_NOT_SATISFIED);

    // `AbilitySet::PUBLIC_FUNCTIONS` does not satisfy `T: key`.
    let status =
        run_with_ability_byte(&mut harness, &account, "make_object", LEGAL_ABILITY_BYTE, 1);
    assert_vm_status!(status, StatusCode::CONSTRAINT_NOT_SATISFIED);
}

/// With the check disabled, canonical-name collisions remain and function types that claim `key`
/// satisfy `T: key`.
#[test]
fn behavior_with_the_check_disabled_is_unchanged() {
    let (mut harness, account) = publish();

    harness.enable_features(vec![], vec![FeatureFlag::CHECK_FUNCTION_TYPE_ABILITIES]);

    for (ability_byte, value) in [(LEGAL_ABILITY_BYTE, 7u64), (UNKNOWN_BIT_ABILITY_BYTE, 9u64)] {
        let status = run_with_ability_byte(&mut harness, &account, "observe", ability_byte, value);
        assert_success!(status);
    }

    let observed = harness
        .read_resource::<Observed>(account.address(), observed_struct_tag())
        .expect("Observed resource must exist");
    assert_eq!(observed.type_arg_names, vec![
        "||() has copy + drop + store".to_string(),
        "||() has copy + drop + store".to_string(),
    ]);

    // Canonical-name collisions do not collapse BCS-derived state keys.
    assert_ne!(
        holder_struct_tag(LEGAL_ABILITY_BYTE).access_vector(),
        holder_struct_tag(UNKNOWN_BIT_ABILITY_BYTE).access_vector()
    );
    assert_eq!(
        harness.read_resource::<u64>(account.address(), holder_struct_tag(LEGAL_ABILITY_BYTE)),
        Some(7)
    );
    assert_eq!(
        harness.read_resource::<u64>(
            account.address(),
            holder_struct_tag(UNKNOWN_BIT_ABILITY_BYTE)
        ),
        Some(9)
    );

    let status = run_with_ability_byte(
        &mut harness,
        &account,
        "make_object",
        FAKE_KEY_ABILITY_BYTE,
        1,
    );
    assert_success!(
        status,
        "with the check disabled a function type claiming `key` still satisfies a `T: key` bound"
    );
}
