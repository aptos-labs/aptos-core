// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::vote::Vote;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

/// DelayedQCMsg is the struct that is sent by the proposer to self when it receives enough votes
/// for a QC but it still delays the creation of the QC to ensure that slow nodes are given enough
/// time to catch up to the chain and cast their votes.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct DelayedQcMsg {
    /// Vote data for the QC that is being delayed.
    pub vote: Vote,
}

impl Display for DelayedQcMsg {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "DelayedQcMsg: vote [{}]", self.vote,)
    }
}

impl DelayedQcMsg {
    pub fn new(vote: Vote) -> Self {
        Self { vote }
    }

    pub fn vote(&self) -> &Vote {
        &self.vote
    }
}
