// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::order_vote::OrderVote;
use aptos_types::validator_verifier::ValidatorVerifier;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct OrderVoteMsg {
    /// The container for the vote (VoteData, LedgerInfo, Signature)
    vote: OrderVote,
}

impl Display for OrderVoteMsg {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "OrderVoteMsg: [{}]", self.vote)
    }
}

impl OrderVoteMsg {
    pub fn new(vote: OrderVote) -> Self {
        Self { vote }
    }

    /// Container for actual voting material
    pub fn vote(&self) -> &OrderVote {
        &self.vote
    }

    pub fn epoch(&self) -> u64 {
        self.vote.epoch()
    }

    pub fn verify(&self, validator: &ValidatorVerifier) -> anyhow::Result<()> {
        self.vote().verify(validator)
    }
}