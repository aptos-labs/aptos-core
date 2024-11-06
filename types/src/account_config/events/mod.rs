// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

pub mod coin;
pub mod fungible_asset;
pub mod new_block;
pub mod new_epoch;

pub use coin::*;
pub use fungible_asset::*;
pub use new_block::*;
pub use new_epoch::*;

pub fn is_aptos_governance_create_proposal_event(event_type: &str) -> bool {
    event_type == "0x1::aptos_governance::CreateProposal"
        || event_type == "0x1::aptos_governance::CreateProposalEvent"
}
