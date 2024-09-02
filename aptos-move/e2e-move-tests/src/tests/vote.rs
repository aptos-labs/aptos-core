// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    supra_governance::*, assert_abort, assert_success,
    tests::common, MoveHarness,
};
use aptos_types::account_address::AccountAddress;
use once_cell::sync::Lazy;
use std::collections::BTreeMap;

pub static PROPOSAL_SCRIPTS: Lazy<BTreeMap<String, Vec<u8>>> = Lazy::new(build_scripts);

fn build_scripts() -> BTreeMap<String, Vec<u8>> {
    let package_folder = "vote.data";
    let package_names = vec!["enable_partial_governance_voting"];
    common::build_scripts(package_folder, package_names)
}

#[test]
fn test_supra_vote() {
    // Genesis starts with one validator with index 0
    let mut harness = MoveHarness::new();
    let proposer = harness.new_account_at(AccountAddress::from_hex_literal("0xdd2").unwrap());
    let voter = harness.new_account_at(AccountAddress::from_hex_literal("0xdd1").unwrap());

    let mut proposal_id: u64 = 0;
    assert_success!(supra_create_proposal_v2(
        &mut harness,
        &proposer,
        vec![1],
        vec![],
        vec![],
        true
    ));
    // Voters can vote on a voting proposal.
    assert_success!(supra_vote(
        &mut harness,
        &voter,
        proposal_id,
        true
    ));

    // Enable partial governance voting. In production, it requires governance.
    let core_resources =
        harness.new_account_at(AccountAddress::from_hex_literal("0xA550C18").unwrap());
    let script_code = PROPOSAL_SCRIPTS
        .get("enable_partial_governance_voting")
        .expect("proposal script should be built");
    let txn = harness.create_script(&core_resources, script_code.clone(), vec![], vec![]);
    assert_success!(harness.run(txn));

    // If a voter has already voted on a proposal before partial voting is enabled, the voter cannot vote on the proposal again.
    assert_abort!(
        supra_vote(
            &mut harness,
            &voter,
            proposal_id,
            true
        ),
        0x8000D
    );

    assert_success!(supra_create_proposal_v2(
        &mut harness,
        &voter,
        vec![1],
        vec![],
        vec![],
        true
    ));

    // Cannot vote on a non-exist proposal.
    let wrong_proposal_id: u64 = 2;
    assert_abort!(
        supra_vote(
            &mut harness,
            &voter,
            wrong_proposal_id,
            true
        ),
        25863
    );

    proposal_id = 1;
    // A voter can vote on a proposal multiple times with both Yes/No.
    assert_success!(supra_vote(
        &mut harness,
        &voter,
        proposal_id,
        true
    ));
    assert_success!(supra_vote(
        &mut harness,
        &voter,
        proposal_id,
        false
    ));
}
