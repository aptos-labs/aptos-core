// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Note[Orderless]: Done
use crate::{tests::common, MoveHarness};
use aptos_crypto::HashValue;
use aptos_framework::{BuildOptions, BuiltPackage};
use aptos_gas_algebra::Gas;
use aptos_language_e2e_tests::account::{Account, TransactionBuilder};
use aptos_types::{
    on_chain_config::{ApprovedExecutionHashes, OnChainConfig},
    transaction::{ExecutionStatus, Script, TransactionArgument, TransactionStatus},
    vm_status::StatusCode,
};
use rstest::rstest;

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
fn large_transactions(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    // This test validates that only small txns (less than the maximum txn size) can be kept. It
    // then evaluates the limits of the ApprovedExecutionHashes. Specifically, the hash is the code
    // is the only portion that can exceed the size limits. There's a further restriction on the
    // maximum transaction size of 1 MB even for governance, because the governance transaction can
    // be submitted by any one and that can result in a large amount of large transactions making their
    // way into consensus.
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);

    let alice = h.new_account_with_key_pair(if stateless_account { None } else { Some(0) });
    let root = h.aptos_framework_account();
    let entries = ApprovedExecutionHashes { entries: vec![] };
    h.set_resource(
        *root.address(),
        ApprovedExecutionHashes::struct_tag(),
        &entries,
    );

    let small = vec![0; 1024];
    // Max size is 1024 * 1024
    let large = vec![0; 1000 * 1024];
    let very_large = vec![0; 1024 * 1024];

    let status = run(&mut h, &alice, small.clone(), small.clone());
    assert!(!status.is_discarded(), "status: {:?}", status);

    let status = run(&mut h, &alice, large.clone(), small.clone());
    assert!(status.is_discarded(), "status: {:?}", status);

    let status = run(&mut h, &alice, small.clone(), large.clone());
    assert!(status.is_discarded(), "status: {:?}", status);

    let status = run(&mut h, &alice, large.clone(), large.clone());
    assert!(status.is_discarded(), "status: {:?}", status);

    let status = run(&mut h, &alice, very_large.clone(), small.clone());
    assert!(status.is_discarded(), "status: {:?}", status);

    let entries = ApprovedExecutionHashes {
        entries: vec![
            (0, HashValue::sha3_256_of(&large).to_vec()),
            (1, HashValue::sha3_256_of(&very_large).to_vec()),
        ],
    };
    h.set_resource(
        *root.address(),
        ApprovedExecutionHashes::struct_tag(),
        &entries,
    );

    let status = run(&mut h, &alice, small.clone(), small.clone());
    assert!(!status.is_discarded(), "status: {:?}", status);

    let status = run(&mut h, &alice, large.clone(), small.clone());
    assert!(!status.is_discarded(), "status: {:?}", status);

    let status = run(&mut h, &alice, small.clone(), large.clone());
    assert!(status.is_discarded(), "status: {:?}", status);

    let status = run(&mut h, &alice, large.clone(), large);
    assert!(status.is_discarded(), "status: {:?}", status);

    let status = run(&mut h, &alice, very_large, small);
    assert!(status.is_discarded(), "status: {:?}", status);
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
fn alt_execution_limit_for_gov_proposals(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    // This test validates that approved governance scripts automatically get the
    // alternate (usually increased) execution limit.
    let max_gas_regular = 10;
    let max_gas_gov = 100;

    // Set up the testing environment
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);

    let alice = h.new_account_with_key_pair(if stateless_account { None } else { Some(0) });
    let root = h.aptos_framework_account();

    h.modify_gas_schedule(|gas_params| {
        let txn = &mut gas_params.vm.txn;

        txn.max_execution_gas = Gas::new(max_gas_regular).to_unit_with_params(txn);
        txn.max_execution_gas_gov = Gas::new(max_gas_gov).to_unit_with_params(txn);
    });
    h.set_resource(
        *root.address(),
        ApprovedExecutionHashes::struct_tag(),
        &ApprovedExecutionHashes { entries: vec![] },
    );

    // Compile the test script, which contains nothing but an infinite loop.
    let package = BuiltPackage::build(
        common::test_dir_path("infinite_loop.data/empty_loop_script"),
        BuildOptions::default(),
    )
    .expect("should be able to build package");
    let script = package
        .extract_script_code()
        .pop()
        .expect("should be able to get script");

    // Execute the script. The amount of gas used should fall within the regular limit.
    let txn = h.create_script(&alice, script.clone(), vec![], vec![]);
    let output = h.run_raw(txn);
    assert_eq!(
        output.status(),
        &TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some(
            StatusCode::EXECUTION_LIMIT_REACHED
        ))),
    );
    let exec_gas_used = output
        .try_extract_fee_statement()
        .ok()
        .flatten()
        .expect("should be able to get fee statement")
        .execution_gas_used();
    let overshoot = (max_gas_regular.min(max_gas_gov) / 5).max(1);
    assert!(max_gas_regular <= exec_gas_used && exec_gas_used <= max_gas_regular + overshoot);

    // Add the hash of the script to the list of approved hashes.
    h.set_resource(
        *root.address(),
        ApprovedExecutionHashes::struct_tag(),
        &ApprovedExecutionHashes {
            entries: vec![(0, HashValue::sha3_256_of(&script).to_vec())],
        },
    );

    // Execute the script again. This time the amount of gas consumed should be much higher, but
    // still fall within the alt limit for gov scripts.
    let txn = h.create_script(&alice, script.clone(), vec![], vec![]);
    let output = h.run_raw(txn);
    assert_eq!(
        output.status(),
        &TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some(
            StatusCode::EXECUTION_LIMIT_REACHED
        ))),
    );
    let exec_gas_used = output
        .try_extract_fee_statement()
        .ok()
        .flatten()
        .expect("should be able to get fee statement")
        .execution_gas_used();
    println!(
        "max_gas_gov: {}, exec_gas_used: {}, overshoot: {}",
        max_gas_gov, exec_gas_used, overshoot
    );
    assert!(max_gas_gov <= exec_gas_used && exec_gas_used <= max_gas_gov + overshoot);

    // TODO: Consider adding a successful transaction that costs x amount of gas where
    //       max_gas_regular < x < max_gas_gov.
    //       Currently we do not have it as it is hard to have a transaction that costs
    //       x amount of gas without it being fragile to gas-related changes.
}

fn run(
    h: &mut MoveHarness,
    account: &Account,
    code: Vec<u8>,
    txn_arg: Vec<u8>,
) -> TransactionStatus {
    let script = Script::new(code, vec![], vec![TransactionArgument::U8Vector(txn_arg)]);

    let mut txn_builder = TransactionBuilder::new(account.clone())
        .script(script)
        .max_gas_amount(1_000_000)
        .gas_unit_price(1)
        .sequence_number(0)
        .current_time(h.executor.get_block_time())
        .upgrade_payload(h.use_txn_payload_v2_format, h.use_orderless_transactions);

    let seq_num = if h.use_orderless_transactions {
        u64::MAX
    } else {
        h.sequence_number_opt(account.address()).unwrap_or(0)
    };
    txn_builder = txn_builder.sequence_number(seq_num);

    let txn = txn_builder.sign();

    h.run(txn)
}
