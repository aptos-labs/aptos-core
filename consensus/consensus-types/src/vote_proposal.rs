// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{block::Block, vote_data::VoteData};
use aptos_crypto::hash::ACCUMULATOR_PLACEHOLDER_HASH;
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

/// This structure contains all the information needed by safety rules to
/// evaluate a proposal / block for correctness / safety and to produce a Vote.
#[derive(Clone, Debug, CryptoHasher, Deserialize, BCSCryptoHash, Serialize)]
pub struct VoteProposal {
    /// The block / proposal to evaluate
    #[serde(bound(deserialize = "Block: Deserialize<'de>"))]
    block: Block,
}

impl VoteProposal {
    pub fn new(block: Block) -> Self {
        Self { block }
    }

    pub fn block(&self) -> &Block {
        &self.block
    }

    pub fn gen_vote_data(&self) -> anyhow::Result<VoteData> {
        Ok(VoteData::new(
            self.block()
                .gen_block_info(*ACCUMULATOR_PLACEHOLDER_HASH, 0, None),
            self.block().quorum_cert().certified_block().clone(),
        ))
    }
}

impl Display for VoteProposal {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "VoteProposal[block: {}]", self.block,)
    }
}
