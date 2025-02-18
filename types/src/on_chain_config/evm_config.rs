// Copyright (c) Supra Foundation
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};
use super::OnChainConfig;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum OnChainEvmConfig {
    V1(EvmConfigV1),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct EvmConfigV1 {
    chain_id: u64,
}

impl OnChainEvmConfig {
    /// TODO: remove, and allow config from genesis parameter.
    pub fn default_for_test() -> Self {
        Self::V1(EvmConfigV1 { chain_id: 0x12_3456_7890 })
    }
    /// TODO: remove, and allow config from genesis parameter.
    pub fn default_for_mainnet() -> Self {
        Self::V1(EvmConfigV1 { chain_id: 0xffff_aaaa_eeee })
    }
}


/// This onchain config does not exist from genesis, until it is added by the governance proposal.
/// If the config is not found, Evm should not be enabled.
impl OnChainConfig for OnChainEvmConfig {
    const MODULE_IDENTIFIER: &'static str = "evm_config";
    const TYPE_IDENTIFIER: &'static str = "EvmConfig";

    /// The Move resource is
    /// ```ignore
    /// struct EvmConfig has copy, drop, store {
    ///    config: vector<u8>,
    /// }
    /// ```
    /// so we need two rounds of bcs deserilization to turn it back to OnChainEvmConfig
    fn deserialize_into_config(bytes: &[u8]) -> Result<Self> {
        let raw_bytes: Vec<u8> = bcs::from_bytes(bytes)?;
        bcs::from_bytes(&raw_bytes)
            .map_err(|e| anyhow!("[on-chain config] Failed to deserialize into config: {}", e))
    }
}
