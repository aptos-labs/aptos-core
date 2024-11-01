// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

pub mod deposit;
pub mod new_block;
pub mod new_epoch;
pub mod withdraw;

pub use deposit::*;
pub use new_block::*;
pub use new_epoch::*;
pub use withdraw::*;

pub fn is_aptos_governance_create_proposal_event(event_type: &str) -> bool {
    event_type == "0x1::aptos_governance::CreateProposal"
        || event_type == "0x1::aptos_governance::CreateProposalEvent"
}
