// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_abort, assert_success, build_package, tests::common, MoveHarness};
use aptos_framework::BuiltPackage;
use aptos_language_e2e_tests::account::{Account, TransactionBuilder};
use aptos_types::{
    account_address::AccountAddress,
    on_chain_config::OnChainConfig,
    randomness::PerBlockRandomness,
    transaction::{Script, TransactionStatus},
};

#[test]
fn test_and_abort_defense_is_sound_and_correct() {
    let mut h = MoveHarness::new();

    // These scripts call a public entry function and a public function. The randomness API will reject both calls.
    for dir in [
        "randomness_unsafe_public_entry.data/pack",
        "randomness_unsafe_public.data/pack",
    ] {
        println!("Testing {dir}");
        // This will redeploy the package, so backwards compatibility must be maintained in these directories.
        let (_, package) = deploy_code(AccountAddress::ONE, dir, &mut h);

        let status = run_script(&mut h, &package);
        assert_abort!(status, 1);
    }

    // The randomness module is initialized, but the randomness seed is not set.
    let fx = h.aptos_framework_account();
    let mut pbr = h
        .read_resource::<PerBlockRandomness>(fx.address(), PerBlockRandomness::struct_tag())
        .unwrap();
    assert!(pbr.seed.is_none());

    pbr.seed = Some((0..32).map(|_| 0u8).collect::<Vec<u8>>());
    assert_eq!(pbr.seed.as_ref().unwrap().len(), 32);
    h.set_resource(*fx.address(), PerBlockRandomness::struct_tag(), &pbr);

    // This is a safe call that the randomness API should allow through.
    let status = run_entry_func(
        &mut h,
        "0xa11ce",
        "0x1::some_randapp::safe_private_entry_call",
    );
    assert_success!(status);

    // This is a safe call that the randomness API should allow through.
    // (I suppose that, since TXNs with private entry function payloads are okay, increasing the
    // visibility to public(friend) should not create any problems.)
    let status = run_entry_func(
        &mut h,
        "0xa11ce",
        "0x1::some_randapp::safe_friend_entry_call",
    );
    assert_success!(status);
}

fn deploy_code(
    addr: AccountAddress,
    code_path: &str,
    harness: &mut MoveHarness,
) -> (Account, BuiltPackage) {
    let account = harness.new_account_at(addr);

    let package = build_package(
        common::test_dir_path(code_path),
        aptos_framework::BuildOptions::default(),
    )
    .expect("building package must succeed");

    let txn = harness.create_publish_built_package(&account, &package, |_| {});

    assert_success!(harness.run(txn));
    (account, package)
}

fn run_script(h: &mut MoveHarness, package: &BuiltPackage) -> TransactionStatus {
    let alice = h.new_account_at(AccountAddress::from_hex_literal("0xa11ce").unwrap());
    let scripts = package.extract_script_code();
    let code = scripts[0].clone();

    let txn = TransactionBuilder::new(alice.clone())
        .script(Script::new(code, vec![], vec![]))
        .sequence_number(10)
        .max_gas_amount(1_000_000)
        .gas_unit_price(1)
        .sign();

    h.run(txn)
}

fn run_entry_func(h: &mut MoveHarness, signer: &str, name: &str) -> TransactionStatus {
    let alice = h.new_account_at(AccountAddress::from_hex_literal(signer).unwrap());

    println!("Running entry function '{name}'");
    h.run_entry_function(&alice, str::parse(name).unwrap(), vec![], vec![])
}
