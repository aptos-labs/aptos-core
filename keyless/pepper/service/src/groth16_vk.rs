// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::watcher::ExternalResource;
use anyhow::{anyhow, ensure, Result};
use aptos_infallible::RwLock;
use ark_bn254::{Bn254, G1Affine, G2Affine};
use ark_groth16::{PreparedVerifyingKey, VerifyingKey};
use ark_serialize::CanonicalDeserialize;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct VKeyData {
    pub alpha_g1: String,
    pub beta_g2: String,
    pub delta_g2: String,
    pub gamma_abc_g1: Vec<String>,
    pub gamma_g2: String,
}

/// On-chain representation of a VK.
///
/// https://fullnode.testnet.aptoslabs.com/v1/accounts/0x1/resource/0x1::keyless_account::Groth16VerificationKey
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct OnChainGroth16VerificationKey {
    /// Some type info returned by node API.
    pub r#type: String,
    pub data: VKeyData,
}

impl OnChainGroth16VerificationKey {
    pub fn to_ark_pvk(&self) -> Result<PreparedVerifyingKey<Bn254>> {
        let mut gamma_abc_g1 = Vec::with_capacity(self.data.gamma_abc_g1.len());
        for (idx, onchain_ele) in self.data.gamma_abc_g1.iter().enumerate() {
            let ark_ele = g1_from_onchain(onchain_ele).map_err(|e| {
                anyhow!("to_ark_pvk() failed with gamma_abc_g1[{idx}] convert err: {e}")
            })?;
            gamma_abc_g1.push(ark_ele);
        }
        let ark_vk = VerifyingKey {
            alpha_g1: g1_from_onchain(&self.data.alpha_g1)
                .map_err(|e| anyhow!("to_ark_pvk() failed with alpha_g1 convert err: {e}"))?,
            beta_g2: g2_from_onchain(&self.data.beta_g2)
                .map_err(|e| anyhow!("to_ark_pvk() failed with beta_g2 convert err: {e}"))?,
            gamma_g2: g2_from_onchain(&self.data.gamma_g2)
                .map_err(|e| anyhow!("to_ark_pvk() failed with gamma_g2 convert err: {e}"))?,
            delta_g2: g2_from_onchain(&self.data.delta_g2)
                .map_err(|e| anyhow!("to_ark_pvk() failed with delta_g2 convert err: {e}"))?,
            gamma_abc_g1,
        };
        Ok(PreparedVerifyingKey::from(ark_vk))
    }
}

/// This variable holds the cached on-chain VK. A refresh loop exists to update it periodically.
pub static ONCHAIN_GROTH16_VK: Lazy<Arc<RwLock<Option<OnChainGroth16VerificationKey>>>> =
    Lazy::new(|| Arc::new(RwLock::new(None)));

fn g1_from_onchain(onchain: &str) -> Result<G1Affine> {
    let lower = onchain.to_lowercase();
    ensure!(lower.len() >= 2);
    ensure!(&lower[0..2] == "0x");
    let bytes = hex::decode(&lower[2..])
        .map_err(|e| anyhow!("g2_from_onchain() failed with hex decode err: {e}"))?;
    let ret = G1Affine::deserialize_compressed(bytes.as_slice())
        .map_err(|e| anyhow!("g1_from_onchain() failed with g1 deser err: {e}"))?;
    Ok(ret)
}

fn g2_from_onchain(onchain: &str) -> Result<G2Affine> {
    let lower = onchain.to_lowercase();
    ensure!(lower.len() >= 2);
    ensure!(&lower[0..2] == "0x");
    let bytes = hex::decode(&lower[2..])
        .map_err(|e| anyhow!("g2_from_onchain() failed with hex decode err: {e}"))?;
    let ret = G2Affine::deserialize_compressed(bytes.as_slice())
        .map_err(|e| anyhow!("g2_from_onchain() failed with g2 deser err: {e}"))?;
    Ok(ret)
}

impl ExternalResource for OnChainGroth16VerificationKey {
    fn resource_name() -> String {
        "OnChainGroth16VerificationKey".to_string()
    }
}
