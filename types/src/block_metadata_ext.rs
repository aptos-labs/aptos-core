// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    block_metadata::BlockMetadata, decryption::BlockTxnDecryptionKey, randomness::Randomness,
};
use aptos_crypto::HashValue;
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
    V2(BlockMetadataWithRandAndDecKey),
    V3(BlockMetadataWithFeatures),
}

/// Flat BCS payload for a single feature's per-block data, as encoded in
/// `BlockMetadataWithFeatures::feature_payloads`.
///
/// This type exists only for encoding (pipeline_builder) and decoding (API layer).
/// It is NOT used as a struct field in any transaction type — the wire format stores
/// `feature_payloads` as raw `Vec<u8>` so the transaction definition is permanently stable.
///
/// Variant indices are ULEB128-encoded by BCS and must match the Move
/// `FeatureBlockPayload` enum in `block.move`:
///   0 → Randomness
///   1 → EncryptedMempool
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PerFeatureBlockPayload {
    Randomness { per_block_seed: Option<Vec<u8>> },
    EncryptedMempool { decryption_key: Option<Vec<u8>> },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockMetadataWithFeatures {
    pub id: HashValue,
    pub epoch: u64,
    pub round: u64,
    pub proposer: AccountAddress,
    #[serde(with = "serde_bytes")]
    pub previous_block_votes_bitvec: Vec<u8>,
    pub failed_proposer_indices: Vec<u32>,
    pub timestamp_usecs: u64,
    /// BCS-encoded `Vec<PerFeatureBlockPayload>`: per-block payloads for enabled features.
    /// Stored as raw bytes so this struct never changes when new features are added.
    #[serde(with = "serde_bytes")]
    pub feature_payloads: Vec<u8>,
    /// Minimal positional DKG flags: `dkg_needed[i] = true` means feature i needs an async DKG
    /// session for the next epoch. Trailing `false` entries are omitted; missing index means false.
    /// Index assignments are defined by `*_DKG_IDX` constants in `reconfiguration_with_dkg`.
    pub dkg_needed: Vec<bool>,
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockMetadataWithRandAndDecKey {
    pub id: HashValue,
    pub epoch: u64,
    pub round: u64,
    pub proposer: AccountAddress,
    #[serde(with = "serde_bytes")]
    pub previous_block_votes_bitvec: Vec<u8>,
    pub failed_proposer_indices: Vec<u32>,
    pub timestamp_usecs: u64,
    pub randomness: Option<Randomness>,
    pub decryption_key: Option<BlockTxnDecryptionKey>,
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

    pub fn new_v2(
        id: HashValue,
        epoch: u64,
        round: u64,
        proposer: AccountAddress,
        previous_block_votes_bitvec: Vec<u8>,
        failed_proposer_indices: Vec<u32>,
        timestamp_usecs: u64,
        randomness: Option<Randomness>,
        decryption_key: Option<BlockTxnDecryptionKey>,
    ) -> Self {
        Self::V2(BlockMetadataWithRandAndDecKey {
            id,
            epoch,
            round,
            proposer,
            previous_block_votes_bitvec,
            failed_proposer_indices,
            timestamp_usecs,
            randomness,
            decryption_key,
        })
    }

    pub fn new_v3(
        id: HashValue,
        epoch: u64,
        round: u64,
        proposer: AccountAddress,
        previous_block_votes_bitvec: Vec<u8>,
        failed_proposer_indices: Vec<u32>,
        timestamp_usecs: u64,
        feature_payloads: Vec<u8>,
        dkg_needed: Vec<bool>,
    ) -> Self {
        Self::V3(BlockMetadataWithFeatures {
            id,
            epoch,
            round,
            proposer,
            previous_block_votes_bitvec,
            failed_proposer_indices,
            timestamp_usecs,
            feature_payloads,
            dkg_needed,
        })
    }

    pub fn id(&self) -> HashValue {
        match self {
            BlockMetadataExt::V0(obj) => obj.id(),
            BlockMetadataExt::V1(obj) => obj.id,
            BlockMetadataExt::V2(obj) => obj.id,
            BlockMetadataExt::V3(obj) => obj.id,
        }
    }

    pub fn timestamp_usecs(&self) -> u64 {
        match self {
            BlockMetadataExt::V0(obj) => obj.timestamp_usecs(),
            BlockMetadataExt::V1(obj) => obj.timestamp_usecs,
            BlockMetadataExt::V2(obj) => obj.timestamp_usecs,
            BlockMetadataExt::V3(obj) => obj.timestamp_usecs,
        }
    }

    pub fn proposer(&self) -> AccountAddress {
        match self {
            BlockMetadataExt::V0(obj) => obj.proposer(),
            BlockMetadataExt::V1(obj) => obj.proposer,
            BlockMetadataExt::V2(obj) => obj.proposer,
            BlockMetadataExt::V3(obj) => obj.proposer,
        }
    }

    pub fn previous_block_votes_bitvec(&self) -> &Vec<u8> {
        match self {
            BlockMetadataExt::V0(obj) => obj.previous_block_votes_bitvec(),
            BlockMetadataExt::V1(obj) => &obj.previous_block_votes_bitvec,
            BlockMetadataExt::V2(obj) => &obj.previous_block_votes_bitvec,
            BlockMetadataExt::V3(obj) => &obj.previous_block_votes_bitvec,
        }
    }

    pub fn failed_proposer_indices(&self) -> &Vec<u32> {
        match self {
            BlockMetadataExt::V0(obj) => obj.failed_proposer_indices(),
            BlockMetadataExt::V1(obj) => &obj.failed_proposer_indices,
            BlockMetadataExt::V2(obj) => &obj.failed_proposer_indices,
            BlockMetadataExt::V3(obj) => &obj.failed_proposer_indices,
        }
    }

    pub fn epoch(&self) -> u64 {
        match self {
            BlockMetadataExt::V0(obj) => obj.epoch(),
            BlockMetadataExt::V1(obj) => obj.epoch,
            BlockMetadataExt::V2(obj) => obj.epoch,
            BlockMetadataExt::V3(obj) => obj.epoch,
        }
    }

    pub fn round(&self) -> u64 {
        match self {
            BlockMetadataExt::V0(obj) => obj.round(),
            BlockMetadataExt::V1(obj) => obj.round,
            BlockMetadataExt::V2(obj) => obj.round,
            BlockMetadataExt::V3(obj) => obj.round,
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            BlockMetadataExt::V0(_) => "block_metadata_ext_transaction__v0",
            BlockMetadataExt::V1(_) => "block_metadata_ext_transaction__v1",
            BlockMetadataExt::V2(_) => "block_metadata_ext_transaction__v2",
            BlockMetadataExt::V3(_) => "block_metadata_ext_transaction__v3",
        }
    }
}

impl From<BlockMetadata> for BlockMetadataExt {
    fn from(v0: BlockMetadata) -> Self {
        BlockMetadataExt::V0(v0)
    }
}
