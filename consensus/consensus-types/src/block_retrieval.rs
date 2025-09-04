// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::block::Block;
use anyhow::ensure;
use velor_crypto::hash::HashValue;
use velor_short_hex_str::AsShortHexStr;
use velor_types::validator_verifier::ValidatorVerifier;
use serde::{Deserialize, Serialize};
use std::fmt;

pub const NUM_RETRIES: usize = 5;
pub const NUM_PEERS_PER_RETRY: usize = 3;
pub const RETRY_INTERVAL_MSEC: u64 = 500;
pub const RPC_TIMEOUT_MSEC: u64 = 5000;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum BlockRetrievalRequest {
    V1(BlockRetrievalRequestV1),
    V2(BlockRetrievalRequestV2),
}

impl BlockRetrievalRequest {
    pub fn new_with_target_round(block_id: HashValue, num_blocks: u64, target_round: u64) -> Self {
        Self::V2(BlockRetrievalRequestV2 {
            block_id,
            num_blocks,
            target_round,
        })
    }

    pub fn block_id(&self) -> HashValue {
        match self {
            BlockRetrievalRequest::V1(req) => req.block_id,
            BlockRetrievalRequest::V2(req) => req.block_id,
        }
    }

    pub fn num_blocks(&self) -> u64 {
        match self {
            BlockRetrievalRequest::V1(req) => req.num_blocks,
            BlockRetrievalRequest::V2(req) => req.num_blocks,
        }
    }
}

/// RPC to get a chain of block of the given length starting from the given block id.
/// TODO @bchocho @hariria fix comment after all nodes upgrade to release with enum BlockRetrievalRequest (not struct)
///
/// NOTE:
/// 1. The [`BlockRetrievalRequest`](BlockRetrievalRequest) struct was renamed to
///    [`BlockRetrievalRequestV1`](BlockRetrievalRequestV1) and deprecated
/// 2. [`BlockRetrievalRequest`](BlockRetrievalRequest) enum was introduced to replace the old
///    [`BlockRetrievalRequest`](BlockRetrievalRequest) struct
///
/// Please use the [`BlockRetrievalRequest`](BlockRetrievalRequest) enum going forward once this enum
/// is introduced in the next release
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct BlockRetrievalRequestV1 {
    block_id: HashValue,
    num_blocks: u64,
    target_block_id: Option<HashValue>,
}

impl BlockRetrievalRequestV1 {
    pub fn new(block_id: HashValue, num_blocks: u64) -> Self {
        Self {
            block_id,
            num_blocks,
            target_block_id: None,
        }
    }

    pub fn new_with_target_block_id(
        block_id: HashValue,
        num_blocks: u64,
        target_block_id: HashValue,
    ) -> Self {
        Self {
            block_id,
            num_blocks,
            target_block_id: Some(target_block_id),
        }
    }

    pub fn block_id(&self) -> HashValue {
        self.block_id
    }

    pub fn num_blocks(&self) -> u64 {
        self.num_blocks
    }

    pub fn target_block_id(&self) -> Option<HashValue> {
        self.target_block_id
    }

    pub fn match_target_id(&self, hash_value: HashValue) -> bool {
        self.target_block_id == Some(hash_value)
    }
}

impl fmt::Display for BlockRetrievalRequestV1 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "[BlockRetrievalRequestV1 starting from id {} with {} blocks]",
            self.block_id, self.num_blocks
        )
    }
}

/// RPC to get a chain of block of the given length starting from the given block id.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct BlockRetrievalRequestV2 {
    block_id: HashValue,
    num_blocks: u64,
    target_round: u64,
}

impl BlockRetrievalRequestV2 {
    pub fn new(block_id: HashValue, num_blocks: u64, target_round: u64) -> Self {
        BlockRetrievalRequestV2 {
            block_id,
            num_blocks,
            target_round,
        }
    }

    pub fn new_with_target_round(block_id: HashValue, num_blocks: u64, target_round: u64) -> Self {
        BlockRetrievalRequestV2 {
            block_id,
            num_blocks,
            target_round,
        }
    }

    pub fn block_id(&self) -> HashValue {
        self.block_id
    }

    pub fn num_blocks(&self) -> u64 {
        self.num_blocks
    }

    pub fn target_round(&self) -> u64 {
        self.target_round
    }

    /// The window start block is either exactly at the target round, or it is at a higher round
    /// and its parent is at a lower round than the target round.
    pub fn is_window_start_block(&self, block: &Block) -> bool {
        block.round() == self.target_round()
            || (block.round() > self.target_round()
                && block.quorum_cert().certified_block().round() < self.target_round())
    }
}

impl fmt::Display for BlockRetrievalRequestV2 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "[BlockRetrievalRequestV2 starting from id {} with {} blocks]",
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

    pub fn verify_inner(&self, retrieval_request: &BlockRetrievalRequest) -> anyhow::Result<()> {
        match &retrieval_request {
            BlockRetrievalRequest::V1(retrieval_request) => {
                ensure!(
                    self.status != BlockRetrievalStatus::Succeeded
                        || self.blocks.len() as u64 == retrieval_request.num_blocks(),
                    "not enough blocks returned, expect {}, get {}",
                    retrieval_request.num_blocks(),
                    self.blocks.len(),
                );
                ensure!(
                    self.status == BlockRetrievalStatus::SucceededWithTarget
                        || !self
                            .blocks
                            .iter()
                            .any(|block| retrieval_request.match_target_id(block.id())),
                    "target was found, but response is not marked as SucceededWithTarget",
                );
                ensure!(
                    self.status != BlockRetrievalStatus::SucceededWithTarget
                        || self
                            .blocks
                            .last()
                            .is_some_and(|block| retrieval_request.match_target_id(block.id())),
                    "target not found in blocks returned, expect {:?}",
                    retrieval_request.target_block_id(),
                );
            },
            BlockRetrievalRequest::V2(retrieval_request) => {
                ensure!(
                    self.status != BlockRetrievalStatus::Succeeded
                        || self.blocks.len() as u64 == retrieval_request.num_blocks(),
                    "not enough blocks returned, expect {}, get {}",
                    retrieval_request.num_blocks(),
                    self.blocks.len(),
                );
                ensure!(
                    self.status == BlockRetrievalStatus::SucceededWithTarget
                        || !self.blocks.last().is_some_and(|block| {
                            block.round() < retrieval_request.target_round()
                                || retrieval_request.is_window_start_block(block)
                        }),
                    "smaller than target round or window start block was found, but response is not marked as SucceededWithTarget",
                );
                ensure!(
                    self.status != BlockRetrievalStatus::SucceededWithTarget
                        || self
                            .blocks
                            .last()
                            .is_some_and(|block| retrieval_request.is_window_start_block(block)),
                    "target not found in blocks returned, expect {},",
                    retrieval_request.target_round(),
                );
            },
        }

        Ok(())
    }

    pub fn verify(
        &self,
        retrieval_request: BlockRetrievalRequest,
        sig_verifier: &ValidatorVerifier,
    ) -> anyhow::Result<()> {
        self.verify_inner(&retrieval_request)?;

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
