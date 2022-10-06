// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{account_address::AccountAddress, event::EventHandle};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct VotingForum {
    pub proposals: AccountAddress,
    events: VotingEvents,
    pub next_proposal_id: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Proposal {
    proposer: AccountAddress,
    execution_content: Vec<GovernanceProposal>,
    metadata: Vec<MetadataEntry>,
    creation_time_secs: u64,
    execution_hash: Vec<u8>,
    min_vote_threshold: u128,
    expiration_secs: u64,
    early_resolution_vote_threshold: Vec<u128>,
    yes_votes: u128,
    no_votes: u128,
    is_resolved: bool,
    resolution_time_secs: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MetadataEntry {
    pub key: String,
    pub value: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GovernanceProposal {
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VotingEvents {
    create_proposal_events: EventHandle,
    register_forum_events: EventHandle,
    resolve_proposal_events: EventHandle,
    vote_events: EventHandle,
}
