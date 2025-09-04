// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use velor_crypto_derive::{BCSCryptoHash, CryptoHasher};
use velor_types::block_info::BlockInfo;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

/// VoteData keeps the information about the block, and its parent.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq, CryptoHasher, BCSCryptoHash)]
pub struct VoteData {
    /// Contains all the block information needed for voting for the proposed round.
    proposed: BlockInfo,
    /// Contains all the block information for the block the proposal is extending.
    parent: BlockInfo,
}

impl Display for VoteData {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "VoteData: [block id: {}, epoch: {}, round: {:02}, timestamp: {},\
             parent_block_id: {}, parent_block_round: {:02}, parent_timestamp: {}]",
            self.proposed().id(),
            self.proposed().epoch(),
            self.proposed().round(),
            self.proposed().timestamp_usecs(),
            self.parent().id(),
            self.parent().round(),
            self.parent.timestamp_usecs()
        )
    }
}

impl VoteData {
    /// Constructs a new VoteData from the block information of a proposed block and the block it extends.
    pub fn new(proposed: BlockInfo, parent: BlockInfo) -> Self {
        Self { proposed, parent }
    }

    pub fn dummy() -> Self {
        Self {
            proposed: BlockInfo::empty(),
            parent: BlockInfo::empty(),
        }
    }

    /// Returns block information associated to the block being extended by the proposal.
    pub fn parent(&self) -> &BlockInfo {
        &self.parent
    }

    /// Returns block information associated to the block being voted on.
    pub fn proposed(&self) -> &BlockInfo {
        &self.proposed
    }

    /// Well-formedness checks that are independent of the current state.
    pub fn verify(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.parent.epoch() == self.proposed.epoch(),
            "Parent and proposed epochs do not match",
        );
        anyhow::ensure!(
            self.parent.round() < self.proposed.round(),
            "Proposed round is less than parent round",
        );
        anyhow::ensure!(
            self.parent.timestamp_usecs() <= self.proposed.timestamp_usecs(),
            "Proposed happened before parent",
        );
        anyhow::ensure!(
            // if decoupled execution is turned on, the versions are dummy values (0),
            // but the genesis block per epoch uses the ground truth version number,
            // so we bypass the version check here.
            self.proposed.version() == 0 || self.parent.version() <= self.proposed.version(),
            "Proposed version is less than parent version",
        );
        Ok(())
    }

    /// Is the vote for a NIL block.
    pub fn is_for_nil(&self) -> bool {
        self.proposed().timestamp_usecs() == self.parent().timestamp_usecs()
    }
}
