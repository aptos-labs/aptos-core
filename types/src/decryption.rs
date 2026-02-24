// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{block_info::Round, on_chain_config::OnChainConfig};
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
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct OnchainPerBlockDecryptionKey {
    pub epoch: u64,
    pub round: u64,
    pub decryption_key: Option<Vec<u8>>,
}

impl OnChainConfig for OnchainPerBlockDecryptionKey {
    const MODULE_IDENTIFIER: &'static str = "decryption";
    const TYPE_IDENTIFIER: &'static str = "PerBlockDecryptionKey";
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
