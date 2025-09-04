// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{block_metadata::BlockMetadata, randomness::Randomness};
use velor_crypto::HashValue;
use move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};

/// The extended block metadata.
///
/// NOTE for `V0`: this is designed to allow a default block metadata to be represented by this type.
/// By doing so, we can use a single type `BlockMetadataExt` across `StateComputer`,
/// and avoid defining an extra `GenericBlockMetadata` enum for many util functions.
///
/// Implementation also ensures correct conversion to enum `Transaction`:
/// `V0` goes to variant `Transaction::BlockMetadata` and the rest goes to variant `Transaction::BlockMetadataExt`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockMetadataExt {
    V0(BlockMetadata),
    V1(BlockMetadataWithRandomness),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockMetadataWithRandomness {
    pub id: HashValue,
    pub epoch: u64,
    pub round: u64,
    pub proposer: AccountAddress,
    #[serde(with = "serde_bytes")]
    pub previous_block_votes_bitvec: Vec<u8>,
    pub failed_proposer_indices: Vec<u32>,
    pub timestamp_usecs: u64,
    pub randomness: Option<Randomness>,
}

impl BlockMetadataExt {
    pub fn new_v1(
        id: HashValue,
        epoch: u64,
        round: u64,
        proposer: AccountAddress,
        previous_block_votes_bitvec: Vec<u8>,
        failed_proposer_indices: Vec<u32>,
        timestamp_usecs: u64,
        randomness: Option<Randomness>,
    ) -> Self {
        Self::V1(BlockMetadataWithRandomness {
            id,
            epoch,
            round,
            proposer,
            previous_block_votes_bitvec,
            failed_proposer_indices,
            timestamp_usecs,
            randomness,
        })
    }

    pub fn id(&self) -> HashValue {
        match self {
            BlockMetadataExt::V0(obj) => obj.id(),
            BlockMetadataExt::V1(obj) => obj.id,
        }
    }

    pub fn timestamp_usecs(&self) -> u64 {
        match self {
            BlockMetadataExt::V0(obj) => obj.timestamp_usecs(),
            BlockMetadataExt::V1(obj) => obj.timestamp_usecs,
        }
    }

    pub fn proposer(&self) -> AccountAddress {
        match self {
            BlockMetadataExt::V0(obj) => obj.proposer(),
            BlockMetadataExt::V1(obj) => obj.proposer,
        }
    }

    pub fn previous_block_votes_bitvec(&self) -> &Vec<u8> {
        match self {
            BlockMetadataExt::V0(obj) => obj.previous_block_votes_bitvec(),
            BlockMetadataExt::V1(obj) => &obj.previous_block_votes_bitvec,
        }
    }

    pub fn failed_proposer_indices(&self) -> &Vec<u32> {
        match self {
            BlockMetadataExt::V0(obj) => obj.failed_proposer_indices(),
            BlockMetadataExt::V1(obj) => &obj.failed_proposer_indices,
        }
    }

    pub fn epoch(&self) -> u64 {
        match self {
            BlockMetadataExt::V0(obj) => obj.epoch(),
            BlockMetadataExt::V1(obj) => obj.epoch,
        }
    }

    pub fn round(&self) -> u64 {
        match self {
            BlockMetadataExt::V0(obj) => obj.round(),
            BlockMetadataExt::V1(obj) => obj.round,
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            BlockMetadataExt::V0(_) => "block_metadata_ext_transaction__v0",
            BlockMetadataExt::V1(_) => "block_metadata_ext_transaction__v1",
        }
    }
}

impl From<BlockMetadata> for BlockMetadataExt {
    fn from(v0: BlockMetadata) -> Self {
        BlockMetadataExt::V0(v0)
    }
}
