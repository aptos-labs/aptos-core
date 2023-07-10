// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::common::{Author, Round};
use aptos_crypto::HashValue;
use aptos_short_hex_str::AsShortHexStr;
use aptos_types::{
    block_info::BlockInfo,
    validator_verifier::ValidatorVerifier,
};
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display, Formatter};

type Epoch = u64;

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct RandShare {
    author: Author, // can remove
    block_info: BlockInfo,
    share: Vec<u8>,    // place holder for the VRF share
}

// this is required by structured log
impl Debug for RandShare {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl Display for RandShare {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "RandShare: [author: {}, block_info {}]",
            self.author.short_str(),
            self.block_info,
        )
    }
}

impl RandShare {
    /// Generates a new RandShare
    pub fn new(
        author: Author,
        block_info: BlockInfo,
        share: Vec<u8>,
    ) -> Self {
        Self {
            author,
            block_info,
            share,
        }
    }

    pub fn author(&self) -> Author {
        self.author
    }

    pub fn block_info(&self) -> &BlockInfo {
        &self.block_info
    }

    pub fn round(&self) -> Round {
        self.block_info.round()
    }

    pub fn epoch(&self) -> Epoch {
        self.block_info.epoch()
    }

    /// Verifies that the consensus data hash of LedgerInfo corresponds to the commit proposal,
    /// and then verifies the signature.
    pub fn verify(&self, _validator: &ValidatorVerifier) -> anyhow::Result<()> {
        // todo: also need to verify the validity of the VRF share
        Ok(())
    }

    pub fn share(&self) -> &Vec<u8> {
        &self.share
    }
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct RandShares {
    item_id: HashValue,   // hash of the ordered_item
    author: Author,
    epoch: u64,
    shares: Vec<Option<RandShare>>,
}

// this is required by structured log
impl Debug for RandShares {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl Display for RandShares {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "RandShares: [item_id: {}, author: {}, epoch: {}, shares {:?}]",
            self.item_id.short_str(),
            self.author,
            self.epoch,
            self.shares,
        )
    }
}

impl RandShares {
    /// Generates a new RandShare
    pub fn new(
        item_id: HashValue,
        author: Author,
        epoch: Epoch,
        shares: Vec<Option<RandShare>>,
    ) -> Self {
        Self {
            item_id,
            author,
            epoch,
            shares,
        }
    }

    pub fn item_id(&self) -> HashValue {
        self.item_id
    }

    pub fn author(&self) -> Author {
        self.author
    }

    pub fn epoch(&self) -> Epoch {
        self.epoch
    }

    /// Verifies that the consensus data hash of LedgerInfo corresponds to the commit proposal,
    /// and then verifies the signature.
    pub fn verify(&self, _validator: &ValidatorVerifier) -> anyhow::Result<()> {
        // todo: also need to verify the validity of the VRF share
        Ok(())
    }

    pub fn shares(&self) -> &Vec<Option<RandShare>> {
        &self.shares
    }

    pub fn rounds(&self) -> Vec<Round> {
        self.shares.iter().filter_map(|s| s.as_ref().map(|share| share.round())).collect()
    }
}
