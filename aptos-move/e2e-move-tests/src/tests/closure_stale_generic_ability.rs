// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{assert_abort, assert_vm_status, tests::common, MoveHarness};
use aptos_cached_packages::aptos_stdlib;
use aptos_framework::{BuildOptions, BuiltPackage};
use aptos_language_e2e_tests::account::Account;
use aptos_package_builder::PackageBuilder;
use aptos_types::{
    account_address::AccountAddress,
    move_utils::MemberId,
    on_chain_config::TimedFeatureFlag,
    transaction::{EntryFunction, ExecutionStatus, TransactionPayload, TransactionStatus},
};
use aptos_vm_environment::prod_configs::{set_async_runtime_checks, set_paranoid_type_checks};
use move_core_types::vm_status::StatusCode;

const ADDRESS: &str = "0xca14";
const FEE_PAYER_ADDRESS: &str = "0xca15";
const SUPPLY_HOLDER_ADDRESS: &str = "0xca16";
const OCTAS_PER_APT: u64 = 100_000_000;
const INITIAL_BALANCE_APT: u64 = 1_000_000;
const INITIAL_BALANCE: u64 = INITIAL_BALANCE_APT * OCTAS_PER_APT;
const FEE_PAYER_BALANCE: u64 = 100 * OCTAS_PER_APT;

const V1: &str = r#"
module 0xca14::provider {
    use aptos_std::copyable_any::Any;

    package fun pack<X: store>(value: X): Any {
        let _ = value;
        abort 1401
    }
}

module 0xca14::caller {
    use std::signer;
    use aptos_std::copyable_any;
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::coin::{Self, Coin};
    use 0xca14::provider;

    const AMOUNT: u64 = 100000000000000;

    struct Single has key {
        coin: Coin<AptosCoin>,
    }

    struct Doubled has key {
        left: Coin<AptosCoin>,
        right: Coin<AptosCoin>,
    }

    struct SupplyReserve has key {
        coin: Coin<AptosCoin>,
    }

    public entry fun create_supply_reserve(owner: &signer) {
        move_to(owner, SupplyReserve { coin: coin::withdraw<AptosCoin>(owner, AMOUNT) })
    }

    public entry fun create(owner: &signer) {
        assert!(coin::balance<AptosCoin>(signer::address_of(owner)) == AMOUNT, 1401);
        let coin = coin::withdraw<AptosCoin>(owner, AMOUNT);
        assert!(coin::value(&coin) == AMOUNT, 1402);
        move_to(owner, Single { coin })
    }

    #[persistent]
    public fun closure_target(owner: &signer, input: u64): u64 {
        if (input == 0) {
            return 10
        };

        let Single { coin } = move_from<Single>(signer::address_of(owner));
        assert!(coin::value(&coin) == AMOUNT, 1403);

        let packed = provider::pack<Coin<AptosCoin>>(coin);

        let packed_copy = copy packed;
        let left = copyable_any::unpack<Coin<AptosCoin>>(packed);
        let right = copyable_any::unpack<Coin<AptosCoin>>(packed_copy);

        assert!(coin::value(&left) == AMOUNT, 1404);
        assert!(coin::value(&right) == AMOUNT, 1405);
        move_to(owner, Doubled { left, right });
        99
    }

    public fun assert_doubled(addr: address) acquires Doubled {
        let doubled = borrow_global<Doubled>(addr);
        assert!(coin::value(&doubled.left) == AMOUNT, 1406);
        assert!(coin::value(&doubled.right) == AMOUNT, 1407)
    }

    public entry fun redeem(owner: &signer) acquires Doubled {
        let addr = signer::address_of(owner);
        let balance_before = coin::balance<AptosCoin>(addr);
        assert!(balance_before == 0, 1408);
        let Doubled { left, right } = move_from<Doubled>(addr);
        assert!(coin::value(&left) == AMOUNT, 1409);
        assert!(coin::value(&right) == AMOUNT, 1410);
        coin::deposit<AptosCoin>(addr, left);
        coin::deposit<AptosCoin>(addr, right);
        assert!(coin::balance<AptosCoin>(addr) == 2 * AMOUNT, 1411)
    }
}

module 0xca14::keeper {
    use 0xca14::caller;

    struct Holder has key {
        f: |&signer, u64|u64 has copy + drop + store,
    }

    public entry fun create(owner: &signer) {
        move_to(owner, Holder { f: caller::closure_target })
    }

    public fun invoke(owner: &signer, input: u64): u64 {
        let f = borrow_global<Holder>(std::signer::address_of(owner)).f;
        f(owner, input)
    }
}

module 0xca14::upgrade_driver {
    use aptos_framework::code;
    use 0xca14::keeper;

    public entry fun upgrade(
        owner: &signer,
        metadata: vector<u8>,
        code: vector<vector<u8>>,
    ) {
        assert!(keeper::invoke(owner, 0) == 10, 1411);
        code::publish_package_txn(owner, metadata, code)
    }
}
"#;

const V2: &str = r#"
module 0xca14::provider {
    use aptos_std::copyable_any::{Self, Any};

    public fun pack<X: copy + drop + store>(value: X): Any {
        copyable_any::pack<X>(value)
    }
}

module 0xca14::caller {
    use std::signer;
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::coin::{Self, Coin};

    const AMOUNT: u64 = 100000000000000;

    struct Single has key {
        coin: Coin<AptosCoin>,
    }

    struct Doubled has key {
        left: Coin<AptosCoin>,
        right: Coin<AptosCoin>,
    }

    struct SupplyReserve has key {
        coin: Coin<AptosCoin>,
    }

    public entry fun create_supply_reserve(owner: &signer) {
        move_to(owner, SupplyReserve { coin: coin::withdraw<AptosCoin>(owner, AMOUNT) })
    }

    public entry fun create(owner: &signer) {
        assert!(coin::balance<AptosCoin>(signer::address_of(owner)) == AMOUNT, 1401);
        let coin = coin::withdraw<AptosCoin>(owner, AMOUNT);
        assert!(coin::value(&coin) == AMOUNT, 1402);
        move_to(owner, Single { coin })
    }

    #[persistent]
    public fun closure_target(_owner: &signer, _input: u64): u64 { 777 }

    public fun assert_doubled(addr: address) acquires Doubled {
        let doubled = borrow_global<Doubled>(addr);
        assert!(coin::value(&doubled.left) == AMOUNT, 1406);
        assert!(coin::value(&doubled.right) == AMOUNT, 1407)
    }

    public entry fun redeem(owner: &signer) acquires Doubled {
        let addr = signer::address_of(owner);
        let balance_before = coin::balance<AptosCoin>(addr);
        assert!(balance_before == 0, 1408);
        let Doubled { left, right } = move_from<Doubled>(addr);
        assert!(coin::value(&left) == AMOUNT, 1409);
        assert!(coin::value(&right) == AMOUNT, 1410);
        coin::deposit<AptosCoin>(addr, left);
        coin::deposit<AptosCoin>(addr, right);
        assert!(coin::balance<AptosCoin>(addr) == 2 * AMOUNT, 1411)
    }
}

module 0xca14::keeper {
    use 0xca14::caller;

    struct Holder has key {
        f: |&signer, u64|u64 has copy + drop + store,
    }

    public entry fun create(owner: &signer) {
        move_to(owner, Holder { f: caller::closure_target })
    }

    public fun invoke(owner: &signer, input: u64): u64 {
        let f = borrow_global<Holder>(std::signer::address_of(owner)).f;
        f(owner, input)
    }
}

module 0xca14::upgrade_driver {
    use aptos_framework::code;
    use 0xca14::keeper;

    public entry fun upgrade(
        owner: &signer,
        metadata: vector<u8>,
        code: vector<vector<u8>>,
    ) {
        assert!(keeper::invoke(owner, 0) == 777, 1411);
        code::publish_package_txn(owner, metadata, code)
    }
}

module 0xca14::trigger {
    use std::signer;
    use 0xca14::caller;
    use 0xca14::keeper;

    fun init_module(owner: &signer) {
        assert!(keeper::invoke(owner, 1) == 99, 1412);
        caller::assert_doubled(signer::address_of(owner))
    }
}
"#;

fn package_builder(source: &str) -> PackageBuilder {
    let mut builder = PackageBuilder::new("StagedGenericAbilityCopyableAnyAptosCoinClone");
    builder.add_source("coin_clone.move", source);
    for (name, path) in [
        ("MoveStdlib", "move-stdlib"),
        ("AptosStdlib", "aptos-stdlib"),
        ("AptosFramework", "aptos-framework"),
    ] {
        builder.add_local_dep(name, &common::framework_dir_path(path).to_string_lossy());
    }
    builder
}

fn expect_success(label: &str, status: TransactionStatus) {
    println!("{label}: {status:?}");
    assert_eq!(status, TransactionStatus::Keep(ExecutionStatus::Success));
}

fn entry_function_payload(function: &str, args: Vec<Vec<u8>>) -> TransactionPayload {
    let MemberId {
        module_id,
        member_id,
    } = function.parse().expect("valid entry-function member id");
    TransactionPayload::EntryFunction(EntryFunction::new(module_id, member_id, vec![], args))
}

fn publish_payload(package: &BuiltPackage) -> TransactionPayload {
    let metadata = package.extract_metadata().expect("package metadata");
    aptos_stdlib::code_publish_package_txn(
        bcs::to_bytes(&metadata).expect("serialize package metadata"),
        package.extract_code(),
    )
}

fn run_with_fee_payer(
    harness: &mut MoveHarness,
    sender: &Account,
    fee_payer: &Account,
    payload: TransactionPayload,
) -> TransactionStatus {
    let txn = harness
        .create_transaction_without_sign(sender, payload)
        .fee_payer(fee_payer.clone())
        .sign_fee_payer();
    harness.run(txn)
}

/// A stored closure over `caller::closure_target` is resolved before `caller` is upgraded, then
/// re-invoked (via a new module's `init_module`) within the same upgrade transaction. Two
/// independent defenses stop the stale closure from duplicating a coin:
///   - with revalidation enabled, the closure is re-resolved to the upgraded `closure_target`
///     (which returns 777), so `init_module` aborts with 1412;
///   - with revalidation disabled, the stale body runs but its cross-module call into the upgraded
///     `provider::pack` fails the type-argument ability recheck (`CONSTRAINT_NOT_SATISFIED`).
/// Either way the upgrade is discarded and the coin is never duplicated.
fn run_coin_duplication_scenario(revalidation_enabled: bool) {
    set_paranoid_type_checks(true);
    set_async_runtime_checks(true);

    let mut harness = MoveHarness::new_testnet();
    harness.set_timed_feature(
        TimedFeatureFlag::RevalidateResolvedClosures,
        revalidation_enabled,
    );
    let account = harness.new_account_with_balance_at(
        AccountAddress::from_hex_literal(ADDRESS).unwrap(),
        INITIAL_BALANCE,
    );
    let fee_payer = harness.new_account_with_balance_at(
        AccountAddress::from_hex_literal(FEE_PAYER_ADDRESS).unwrap(),
        FEE_PAYER_BALANCE,
    );
    let supply_holder = harness.new_account_with_balance_at(
        AccountAddress::from_hex_literal(SUPPLY_HOLDER_ADDRESS).unwrap(),
        INITIAL_BALANCE,
    );
    assert_eq!(
        harness.read_aptos_balance(account.address()),
        INITIAL_BALANCE
    );

    let v1 = package_builder(V1).write_to_temp().expect("write v1");
    let built_v1 =
        BuiltPackage::build(v1.path().to_path_buf(), BuildOptions::move_2()).expect("build v1");
    expect_success(
        "publish v1",
        run_with_fee_payer(
            &mut harness,
            &account,
            &fee_payer,
            publish_payload(&built_v1),
        ),
    );
    assert_eq!(
        harness.read_aptos_balance(account.address()),
        INITIAL_BALANCE
    );

    expect_success(
        "caller::create_supply_reserve",
        run_with_fee_payer(
            &mut harness,
            &supply_holder,
            &fee_payer,
            entry_function_payload(&format!("{ADDRESS}::caller::create_supply_reserve"), vec![]),
        ),
    );

    for function in ["caller::create", "keeper::create"] {
        expect_success(
            function,
            run_with_fee_payer(
                &mut harness,
                &account,
                &fee_payer,
                entry_function_payload(&format!("{ADDRESS}::{function}"), vec![]),
            ),
        );
    }
    assert_eq!(harness.read_aptos_balance(account.address()), 0);

    let v2 = package_builder(V2).write_to_temp().expect("write v2");
    let built =
        BuiltPackage::build(v2.path().to_path_buf(), BuildOptions::move_2()).expect("build v2");
    let metadata = bcs::to_bytes(&built.extract_metadata().expect("metadata")).unwrap();
    let code = built.extract_code();

    let upgrade_status = run_with_fee_payer(
        &mut harness,
        &account,
        &fee_payer,
        entry_function_payload(&format!("{ADDRESS}::upgrade_driver::upgrade"), vec![
            bcs::to_bytes(&metadata).unwrap(),
            bcs::to_bytes(&code).unwrap(),
        ]),
    );
    println!("upgrade (revalidation_enabled={revalidation_enabled}): {upgrade_status:?}");

    if revalidation_enabled {
        assert_abort!(upgrade_status, 1412);
        assert_eq!(harness.read_aptos_balance(account.address()), 0);
        return;
    }

    // Closure revalidation is disabled, so the stale closure still executes the pre-upgrade
    // `closure_target`. But when that body invokes the upgraded `provider::pack` through a
    // cross-module generic call, the runtime re-checks the type-argument abilities against the
    // resolved callee. The upgraded `pack` requires `copy + drop + store`, which `Coin<AptosCoin>`
    // does not satisfy, so the call is rejected and the upgrade transaction is discarded. The coin
    // is therefore never duplicated.
    assert_vm_status!(upgrade_status, StatusCode::CONSTRAINT_NOT_SATISFIED);
    assert_eq!(harness.read_aptos_balance(account.address()), 0);
}

#[test]
fn coin_duplication_prevented_by_ability_recheck_when_revalidation_disabled() {
    run_coin_duplication_scenario(false);
}

#[test]
fn coin_duplication_prevented_when_revalidation_enabled() {
    run_coin_duplication_scenario(true);
}
