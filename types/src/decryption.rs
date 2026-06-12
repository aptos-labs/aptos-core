// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{block_info::Round, on_chain_config::OnChainConfig, secret_sharing::SecretSharedKey};
use anyhow::Context;
use move_core_types::{ident_str, identifier::IdentStr, move_resource::MoveResource};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug, Default, PartialEq, Eq, Hash)]
pub struct DecKeyMetadata {
    pub epoch: u64,
    pub round: Round,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default, PartialEq, Eq)]
pub struct BlockTxnDecryptionKey {
    metadata: DecKeyMetadata,
    #[serde(with = "serde_bytes")]
    decryption_key: Vec<u8>,
}

impl BlockTxnDecryptionKey {
    pub fn new(metadata: DecKeyMetadata, decryption_key: Vec<u8>) -> Self {
        Self {
            metadata,
            decryption_key,
        }
    }

    pub fn metadata(&self) -> &DecKeyMetadata {
        &self.metadata
    }

    pub fn epoch(&self) -> u64 {
        self.metadata.epoch
    }

    pub fn round(&self) -> Round {
        self.metadata.round
    }

    pub fn decryption_key(&self) -> &[u8] {
        &self.decryption_key
    }

    pub fn decryption_key_cloned(&self) -> Vec<u8> {
        self.decryption_key.clone()
    }

    pub fn from_secret_shared_key(key: &SecretSharedKey) -> anyhow::Result<Self> {
        Ok(Self::new(
            DecKeyMetadata {
                epoch: key.metadata.epoch,
                round: key.metadata.round,
            },
            bcs::to_bytes(&key.key).context("SecretSharedKey serialization")?,
        ))
    }
}

/// Mirror of `aptos_framework::decryption::PerBlockDecryptionKeyV2`.
/// Read once per epoch in `build_root`: its existence marks that dense
/// encryption-round tracking is active (blocks emit `BlockMetadataExt::V3`),
/// and `next_encryption_round` seeds the in-memory round chain. Not used on
/// the steady-state hot path.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct OnchainPerBlockDecryptionKeyV2 {
    pub epoch: u64,
    pub block_round: u64,
    pub decryption_key: Option<Vec<u8>>,
    pub encryption_round: Option<u64>,
    pub next_encryption_round: u64,
}

impl OnChainConfig for OnchainPerBlockDecryptionKeyV2 {
    const MODULE_IDENTIFIER: &'static str = "decryption";
    const TYPE_IDENTIFIER: &'static str = "PerBlockDecryptionKeyV2";
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct PerEpochEncryptionKeyResource {
    pub epoch: u64,
    pub encryption_key: Option<Vec<u8>>,
}

impl move_core_types::move_resource::MoveStructType for PerEpochEncryptionKeyResource {
    const MODULE_NAME: &'static IdentStr = ident_str!("decryption");
    const STRUCT_NAME: &'static IdentStr = ident_str!("PerEpochEncryptionKey");
}

impl MoveResource for PerEpochEncryptionKeyResource {}

/// Decryption payload emitted by the consensus pipeline for a block that
/// produced a key: the per-block decryption key paired with the dense
/// `encryption_round` it consumed. Wrap in `Option` at the call site to
/// indicate "no key for this block".
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DecryptionPayload {
    pub key: BlockTxnDecryptionKey,
    pub encryption_round: u64,
}
