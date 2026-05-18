// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
//
// Vendored from aptos-core/types/src/keyless/groth16_vk.rs @ rev 8ec3fb76.

use serde::{Deserialize, Serialize};

/// Groth16 verifying key as published on-chain by `0x1::keyless_account`.
/// Each field is the raw compressed point bytes (no leading length prefix).
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Groth16VerificationKey {
    #[serde(with = "serde_bytes")]
    pub alpha_g1: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub beta_g2: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub gamma_g2: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub delta_g2: Vec<u8>,
    pub gamma_abc_g1: Vec<Vec<u8>>,
}
