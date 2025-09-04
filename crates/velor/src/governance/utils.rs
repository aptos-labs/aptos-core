// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{governance::*, *};
use velor_types::on_chain_config::FeatureFlag;

pub fn vote_to_string(vote: bool) -> &'static str {
    if vote {
        "Yes"
    } else {
        "No"
    }
}

pub fn check_remaining_voting_power(
    remaining_voting_power: u64,
    specified_voting_power: Option<u64>,
) -> u64 {
    let mut voting_power = remaining_voting_power;
    if let Some(specified_voting_power) = specified_voting_power {
        if specified_voting_power > voting_power {
            println!(
                "Stake pool only has {} voting power on proposal.",
                voting_power
            );
        } else {
            voting_power = specified_voting_power;
        };
    };
    voting_power
}

pub async fn is_partial_governance_voting_enabled(client: &Client) -> CliTypedResult<bool> {
    common::utils::get_feature_flag(client, FeatureFlag::PARTIAL_GOVERNANCE_VOTING).await
}

pub async fn is_delegation_pool_partial_governance_voting_enabled(
    client: &Client,
) -> CliTypedResult<bool> {
    common::utils::get_feature_flag(
        client,
        FeatureFlag::DELEGATION_POOL_PARTIAL_GOVERNANCE_VOTING,
    )
    .await
}
