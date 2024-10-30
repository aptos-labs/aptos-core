// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Note[Orderless]: Done
use crate::{assert_abort, assert_success, build_package, tests::common, MoveHarness};
use aptos_framework::BuiltPackage;
use aptos_language_e2e_tests::account::{Account, TransactionBuilder};
use aptos_types::{
    account_address::AccountAddress,
    on_chain_config::OnChainConfig,
    randomness::PerBlockRandomness,
    transaction::{ExecutionStatus, Script, TransactionStatus},
};
use claims::assert_ok;
use move_core_types::{ident_str, language_storage::ModuleId, vm_status::AbortLocation};
use rstest::rstest;

// Error codes from randomness module.
const E_API_USE_SUSCEPTIBLE_TO_TEST_AND_ABORT: u64 = 1;

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn test_and_abort_defense_is_sound_and_correct(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    let acc = h.new_account_with_key_pair(if stateless_account { None } else { Some(0) });
    // These scripts call a public entry function and a public function. The randomness API will reject both calls.
    for dir in [
        "randomness_unsafe_public_entry.data/pack",
        "randomness_unsafe_public.data/pack",
    ] {
        println!("Testing {dir}");
        // This will redeploy the package, so backwards compatibility must be maintained in these directories.
        let (_, package) =
            deploy_code(AccountAddress::ONE, dir, &mut h).expect("building package must succeed");

        let status = run_script(&mut h, &package, stateless_account);
        println!("status: {:?}", status);
        assert_abort!(status, E_API_USE_SUSCEPTIBLE_TO_TEST_AND_ABORT);
    }

    // The randomness module is initialized, but the randomness seed is not set.
    set_randomness_seed(&mut h);

    h.set_default_gas_unit_price(100);
    h.set_max_gas_per_txn(10000); // Should match the default required gas amount.

    // This is a safe call that the randomness API should allow through.
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x1::some_randapp::safe_private_entry_call").unwrap(),
        vec![],
        vec![]
    ));

    // This is a safe call that the randomness API should allow through.
    // (I suppose that, since TXNs with private entry function payloads are okay, increasing the
    // visibility to public(friend) should not create any problems.)
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x1::some_randapp::safe_friend_entry_call").unwrap(),
        vec![],
        vec![]
    ));
}

#[test]
fn test_only_private_entry_function_can_be_annotated() {
    // Make sure building a package fails.
    let mut h = MoveHarness::new();
    assert!(deploy_code(
        AccountAddress::ONE,
        "randomness.data/invalid_pack_non_entry",
        &mut h,
    )
    .is_err());
    assert!(deploy_code(
        AccountAddress::ONE,
        "randomness.data/invalid_pack_public_entry",
        &mut h,
    )
    .is_err());
}

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn test_unbiasable_annotation(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    let acc = h.new_account_with_key_pair(if stateless_account { None } else { Some(0) });
    deploy_code(AccountAddress::ONE, "randomness.data/pack", &mut h)
        .expect("building package must succeed");
    set_randomness_seed(&mut h);

    h.set_default_gas_unit_price(100);
    h.set_max_gas_per_txn(10000); // Should match the default required gas amount.

    let should_succeed = [
        "0x1::test::ok_if_not_annotated_and_not_using_randomness",
        "0x1::test::ok_if_annotated_and_not_using_randomness",
        "0x1::test::ok_if_annotated_and_using_randomness",
    ];

    for entry_func in should_succeed {
        assert_success!(h.run_entry_function(
            &acc,
            str::parse(entry_func).unwrap(),
            vec![],
            vec![]
        ));
    }

    // Non-annotated functions which use randomness fail at runtime.
    let status = h.run_entry_function(
        &acc,
        str::parse("0x1::test::fail_if_not_annotated_and_using_randomness").unwrap(),
        vec![],
        vec![],
    );
    assert_ok!(status.as_kept_status());
    let status = status.as_kept_status().unwrap();

    if let ExecutionStatus::MoveAbort {
        location,
        code,
        info: _,
    } = status
    {
        assert_eq!(
            location,
            AbortLocation::Module(ModuleId::new(
                AccountAddress::ONE,
                ident_str!("randomness").to_owned()
            ))
        );
        assert_eq!(code, E_API_USE_SUSCEPTIBLE_TO_TEST_AND_ABORT);
    } else {
        unreachable!("Non-annotated entry call function should result in Move abort")
    }
}

fn set_randomness_seed(h: &mut MoveHarness) {
    let fx = h.aptos_framework_account();
    let mut pbr = h
        .read_resource::<PerBlockRandomness>(fx.address(), PerBlockRandomness::struct_tag())
        .unwrap();
    assert!(pbr.seed.is_none());

    pbr.seed = Some((0..32).map(|_| 0u8).collect::<Vec<u8>>());
    assert_eq!(pbr.seed.as_ref().unwrap().len(), 32);
    h.set_resource(*fx.address(), PerBlockRandomness::struct_tag(), &pbr);
}

fn deploy_code(
    addr: AccountAddress,
    code_path: &str,
    harness: &mut MoveHarness,
) -> anyhow::Result<(Account, BuiltPackage)> {
    let account = harness.new_account_at(addr, Some(0));

    let package = build_package(
        common::test_dir_path(code_path),
        aptos_framework::BuildOptions::default(),
    )?;

    let txn = harness.create_publish_built_package(&account, &package, |_| {});

    assert_success!(harness.run(txn));
    Ok((account, package))
}

fn run_script(
    h: &mut MoveHarness,
    package: &BuiltPackage,
    stateless_account: bool,
) -> TransactionStatus {
    let alice = h.new_account_with_key_pair(if stateless_account { None } else { Some(0) });
    let scripts = package.extract_script_code();
    let code = scripts[0].clone();

    let txn = TransactionBuilder::new(alice.clone())
        .script(Script::new(code, vec![], vec![]))
        .sequence_number(0)
        .max_gas_amount(1_000_000)
        .gas_unit_price(1)
        .current_time(h.executor.get_block_time_seconds())
        .upgrade_payload(h.use_txn_payload_v2_format, h.use_orderless_transactions)
        .sign();

    h.run(txn)
}
