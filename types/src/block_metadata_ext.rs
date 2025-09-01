// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block_metadata::BlockMetadata, move_utils::as_move_value::AsMoveValue, randomness::Randomness,
};
use aptos_crypto::HashValue;
use move_core_types::{account_address::AccountAddress, value::MoveValue};
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

    pub fn get_prologue_move_args(self) -> Vec<MoveValue> {
        match self {
            BlockMetadataExt::V0(block_metadata) => block_metadata.get_prologue_move_args(),
            BlockMetadataExt::V1(block_metadata) => {
                vec![
                    MoveValue::Signer(AccountAddress::ZERO), // Run as 0x0
                    MoveValue::Address(
                        AccountAddress::from_bytes(block_metadata.id.to_vec()).unwrap(),
                    ),
                    MoveValue::U64(block_metadata.epoch),
                    MoveValue::U64(block_metadata.round),
                    MoveValue::Address(block_metadata.proposer),
                    block_metadata
                        .failed_proposer_indices
                        .into_iter()
                        .map(|i| i as u64)
                        .collect::<Vec<_>>()
                        .as_move_value(),
                    block_metadata.previous_block_votes_bitvec.as_move_value(),
                    MoveValue::U64(block_metadata.timestamp_usecs),
                    block_metadata
                        .randomness
                        .as_ref()
                        .map(Randomness::randomness_cloned)
                        .as_move_value(),
                ]
            },
        }
    }
}

impl From<BlockMetadata> for BlockMetadataExt {
    fn from(v0: BlockMetadata) -> Self {
        BlockMetadataExt::V0(v0)
    }
}
