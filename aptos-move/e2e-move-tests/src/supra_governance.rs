// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::harness::MoveHarness;
use aptos_cached_packages::aptos_stdlib;
use aptos_language_e2e_tests::account::Account;
use aptos_types::{
    account_address::AccountAddress, state_store::table::TableHandle,
    transaction::TransactionStatus,
};
use serde::{Deserialize, Serialize};

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

pub fn supra_create_proposal_v2(
    harness: &mut MoveHarness,
    account: &Account,
    execution_hash: Vec<u8>,
    metadata_location: Vec<u8>,
    metadata_hash: Vec<u8>,
    is_multi_step_proposal: bool,
) -> TransactionStatus {
    harness.run_transaction_payload(
        account,
        aptos_stdlib::supra_governance_supra_create_proposal_v2(
            execution_hash,
            metadata_location,
            metadata_hash,
            is_multi_step_proposal,
        ),
    )
}

pub fn supra_vote(
    harness: &mut MoveHarness,
    account: &Account,
    proposal_id: u64,
    should_pass: bool,
) -> TransactionStatus {
    harness.run_transaction_payload(
        account,
        aptos_stdlib::supra_governance_supra_vote(
            proposal_id,
            should_pass,
        ),
    )
}
