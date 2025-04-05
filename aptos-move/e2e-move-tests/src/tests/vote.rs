// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Note[Orderless]: Done
use crate::{
    aptos_governance::*, assert_abort, assert_success, increase_lockup, setup_staking,
    tests::common, MoveHarness,
};
use aptos_language_e2e_tests::feature_flags_for_orderless;
use aptos_types::account_address::AccountAddress;
use move_core_types::transaction_argument::TransactionArgument;
use once_cell::sync::Lazy;
use rstest::rstest;
use std::collections::BTreeMap;

pub static PROPOSAL_SCRIPTS: Lazy<BTreeMap<String, Vec<u8>>> = Lazy::new(build_scripts);

fn build_scripts() -> BTreeMap<String, Vec<u8>> {
    let package_folder = "vote.data";
    let package_names = vec!["enable_partial_governance_voting"];
    common::build_scripts(package_folder, package_names)
}

#[rstest(
    stateless_account1,
    stateless_account2,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, true, false, false),
    case(true, true, true, false),
    case(true, true, true, true),
    case(true, false, false, false),
    case(true, false, true, false),
    case(true, false, true, true),
    case(false, true, false, false),
    case(false, true, true, false),
    case(false, true, true, true),
    case(false, false, false, false),
    case(false, false, true, false),
    case(false, false, true, true)
)]
fn test_vote(
    stateless_account1: bool,
    stateless_account2: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    // Genesis starts with one validator with index 0
    let mut harness = MoveHarness::new();
    harness.enable_features(
        feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
        vec![],
    );
    let validator_1 =
        harness.new_account_with_key_pair(if stateless_account1 { None } else { Some(0) });
    let validator_2 =
        harness.new_account_with_key_pair(if stateless_account2 { None } else { Some(0) });
    let validator_1_address = *validator_1.address();
    let validator_2_address = *validator_2.address();

    let stake_amount_1 = 25_000_000;
    assert_success!(setup_staking(&mut harness, &validator_1, stake_amount_1));
    assert_success!(increase_lockup(&mut harness, &validator_1));
    let stake_amount_2 = 25_000_000;
    assert_success!(setup_staking(&mut harness, &validator_2, stake_amount_2));
    assert_success!(increase_lockup(&mut harness, &validator_2));

    // Disable partial governance voting.
    let core_resources = harness.new_account_at(
        AccountAddress::from_hex_literal("0xA550C18").unwrap(),
        Some(0),
    );
    let script_code = PROPOSAL_SCRIPTS
        .get("enable_partial_governance_voting")
        .expect("proposal script should be built");
    let txn = harness.create_script(&core_resources, script_code.clone(), vec![], vec![
        TransactionArgument::Bool(false),
    ]);
    assert_success!(harness.run(txn));

    let mut proposal_id: u64 = 0;
    assert_success!(create_proposal_v2(
        &mut harness,
        &validator_2,
        validator_2_address,
        vec![1],
        vec![],
        vec![],
        true
    ));
    // Voters can vote on a partial voting proposal but argument voting_power will be ignored.
    assert_success!(partial_vote(
        &mut harness,
        &validator_1,
        validator_1_address,
        proposal_id,
        100,
        true
    ));
    // No remaining voting power.
    assert_eq!(
        get_remaining_voting_power(&mut harness, validator_1_address, proposal_id),
        0
    );

    // Enable partial governance voting. In production it requires governance.
    let txn = harness.create_script(&core_resources, script_code.clone(), vec![], vec![
        TransactionArgument::Bool(true),
    ]);
    assert_success!(harness.run(txn));

    // If a voter has already voted on a proposal before partial voting is enabled, the voter cannot vote on the proposal again.
    assert_abort!(
        partial_vote(
            &mut harness,
            &validator_1,
            validator_1_address,
            proposal_id,
            100,
            true
        ),
        0x10005
    );

    assert_success!(create_proposal_v2(
        &mut harness,
        &validator_1,
        validator_1_address,
        vec![1],
        vec![],
        vec![],
        true
    ));

    // Cannot vote on a non-exist proposal.
    let wrong_proposal_id: u64 = 2;
    assert_abort!(
        partial_vote(
            &mut harness,
            &validator_1,
            validator_1_address,
            wrong_proposal_id,
            100,
            true
        ),
        25863
    );

    proposal_id = 1;
    assert_eq!(
        get_remaining_voting_power(&mut harness, validator_1_address, proposal_id),
        stake_amount_1
    );
    assert_eq!(
        get_remaining_voting_power(&mut harness, validator_2_address, proposal_id),
        stake_amount_1
    );

    // A voter can vote on a proposal multiple times with both Yes/No.
    assert_success!(partial_vote(
        &mut harness,
        &validator_1,
        validator_1_address,
        proposal_id,
        100,
        true
    ));
    assert_eq!(
        get_remaining_voting_power(&mut harness, validator_1_address, proposal_id),
        stake_amount_1 - 100
    );
    assert_success!(partial_vote(
        &mut harness,
        &validator_1,
        validator_1_address,
        proposal_id,
        1000,
        false
    ));
    assert_eq!(
        get_remaining_voting_power(&mut harness, validator_1_address, proposal_id),
        stake_amount_1 - 1100
    );
    // A voter cannot use voting power more than it has.
    assert_success!(partial_vote(
        &mut harness,
        &validator_1,
        validator_1_address,
        proposal_id,
        stake_amount_1,
        true
    ));
    assert_eq!(
        get_remaining_voting_power(&mut harness, validator_1_address, proposal_id),
        0
    );
}
