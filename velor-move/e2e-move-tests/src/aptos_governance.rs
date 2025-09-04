// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::harness::MoveHarness;
use velor_cached_packages::velor_stdlib;
use velor_language_e2e_tests::account::Account;
use velor_types::{
    account_address::AccountAddress, move_utils::MemberId, state_store::table::TableHandle,
    transaction::TransactionStatus,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Deserialize, Serialize)]
struct PartialVotingProposals {
    pub proposals: TableHandle,
}

#[derive(Deserialize, Serialize)]
struct RecordKey {
    pub stake_pool: AccountAddress,
    pub proposal_id: u64,
}

#[derive(Deserialize, Serialize)]
struct VotingRecordsV2 {
    pub votes: TableHandle,
}

pub fn create_proposal_v2(
    harness: &mut MoveHarness,
    account: &Account,
    stake_pool: AccountAddress,
    execution_hash: Vec<u8>,
    metadata_location: Vec<u8>,
    metadata_hash: Vec<u8>,
    is_multi_step_proposal: bool,
) -> TransactionStatus {
    harness.run_transaction_payload(
        account,
        velor_stdlib::velor_governance_create_proposal_v2(
            stake_pool,
            execution_hash,
            metadata_location,
            metadata_hash,
            is_multi_step_proposal,
        ),
    )
}

pub fn partial_vote(
    harness: &mut MoveHarness,
    account: &Account,
    stake_pool: AccountAddress,
    proposal_id: u64,
    voting_power: u64,
    should_pass: bool,
) -> TransactionStatus {
    harness.run_transaction_payload(
        account,
        velor_stdlib::velor_governance_partial_vote(
            stake_pool,
            proposal_id,
            voting_power,
            should_pass,
        ),
    )
}

pub fn get_remaining_voting_power(
    harness: &mut MoveHarness,
    stake_pool: AccountAddress,
    proposal_id: u64,
) -> u64 {
    let fun = MemberId::from_str("0x1::velor_governance::get_remaining_voting_power").unwrap();
    let res = harness
        .execute_view_function(fun, vec![], vec![
            bcs::to_bytes(&stake_pool).unwrap(),
            bcs::to_bytes(&proposal_id).unwrap(),
        ])
        .values
        .unwrap();
    bcs::from_bytes::<u64>(&res[0]).unwrap()
}
