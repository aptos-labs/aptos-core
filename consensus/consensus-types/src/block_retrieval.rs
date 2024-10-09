// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::block::Block;
use anyhow::ensure;
use aptos_crypto::hash::HashValue;
use aptos_short_hex_str::AsShortHexStr;
use aptos_types::validator_verifier::ValidatorVerifier;
use serde::{Deserialize, Serialize};
use std::fmt;

pub const NUM_RETRIES: usize = 5;
pub const NUM_PEERS_PER_RETRY: usize = 3;
pub const RETRY_INTERVAL_MSEC: u64 = 500;
pub const RPC_TIMEOUT_MSEC: u64 = 5000;

/// RPC to get a chain of block of the given length starting from the given block id.
/// TODO: needs to become a v2 for backwards compatibility
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct BlockRetrievalRequest {
    block_id: HashValue,
    num_blocks: u64,
    // TODO: remove the Option, if it's not too painful
    target_epoch_and_round: Option<(u64, u64)>,
}

impl BlockRetrievalRequest {
    pub fn new(block_id: HashValue, num_blocks: u64) -> Self {
        Self {
            block_id,
            num_blocks,
            target_epoch_and_round: None,
        }
    }

    pub fn new_with_target_round(
        block_id: HashValue,
        num_blocks: u64,
        target_epoch: u64,
        target_round: u64,
    ) -> Self {
        Self {
            block_id,
            num_blocks,
            target_epoch_and_round: Some((target_epoch, target_round)),
        }
    }

    pub fn block_id(&self) -> HashValue {
        self.block_id
    }

    pub fn num_blocks(&self) -> u64 {
        self.num_blocks
    }

    pub fn target_epoch_and_round(&self) -> Option<(u64, u64)> {
        self.target_epoch_and_round
    }

    pub fn match_target_round(&self, epoch: u64, round: u64) -> bool {
        self.target_epoch_and_round
            .map_or(false, |target| (epoch, round) <= target)
    }
}

impl fmt::Display for BlockRetrievalRequest {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "[BlockRetrievalRequest starting from id {} with {} blocks]",
            self.block_id, self.num_blocks
        )
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum BlockRetrievalStatus {
    // Successfully fill in the request.
    Succeeded,
    // Can not find the block corresponding to block_id.
    IdNotFound,
    // Can not find enough blocks but find some.
    NotEnoughBlocks,
    // Successfully found the target,
    SucceededWithTarget,
}

/// Carries the returned blocks and the retrieval status.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct BlockRetrievalResponse {
    status: BlockRetrievalStatus,
    blocks: Vec<Block>,
}

impl BlockRetrievalResponse {
    pub fn new(status: BlockRetrievalStatus, blocks: Vec<Block>) -> Self {
        Self { status, blocks }
    }

    pub fn status(&self) -> BlockRetrievalStatus {
        self.status.clone()
    }

    pub fn blocks(&self) -> &Vec<Block> {
        &self.blocks
    }

    pub fn verify(
        &self,
        retrieval_request: BlockRetrievalRequest,
        sig_verifier: &ValidatorVerifier,
    ) -> anyhow::Result<()> {
        ensure!(
            self.status != BlockRetrievalStatus::Succeeded
                || self.blocks.len() as u64 == retrieval_request.num_blocks(),
            "not enough blocks returned, expect {}, get {}",
            retrieval_request.num_blocks(),
            self.blocks.len(),
        );
        ensure!(
            self.status != BlockRetrievalStatus::SucceededWithTarget
                || (!self.blocks.is_empty()
                    && retrieval_request.match_target_round(
                        self.blocks.last().unwrap().epoch(),
                        self.blocks.last().unwrap().round()
                    )),
            "target not found in blocks returned, expect {:?}, get ({}, {})",
            retrieval_request.target_epoch_and_round(),
            self.blocks.last().unwrap().epoch(),
            self.blocks.last().unwrap().round(),
        );
        self.blocks
            .iter()
            .try_fold(retrieval_request.block_id(), |expected_id, block| {
                block.validate_signature(sig_verifier)?;
                block.verify_well_formed()?;
                ensure!(
                    block.id() == expected_id,
                    "blocks doesn't form a chain: expect {}, get {}",
                    expected_id,
                    block.id()
                );
                Ok(block.parent_id())
            })
            .map(|_| ())
    }
}

impl fmt::Display for BlockRetrievalResponse {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.status() {
            BlockRetrievalStatus::Succeeded | BlockRetrievalStatus::SucceededWithTarget => {
                write!(
                    f,
                    "[BlockRetrievalResponse: status: {:?}, num_blocks: {}, block_ids: ",
                    self.status(),
                    self.blocks().len(),
                )?;

                f.debug_list()
                    .entries(self.blocks.iter().map(|b| b.id().short_str()))
                    .finish()?;

                write!(f, "]")
            },
            _ => write!(f, "[BlockRetrievalResponse: status: {:?}]", self.status()),
        }
    }
}
