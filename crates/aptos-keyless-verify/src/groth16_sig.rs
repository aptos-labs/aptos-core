// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
//
// Vendored from aptos-core/types/src/keyless/groth16_sig.rs @ rev 8ec3fb76.

use crate::{
    bn254_circom::{G1Bytes, G2Bytes},
    ephemeral::EphemeralSignature,
    zkp_sig::ZkProof,
};
use serde::{Deserialize, Serialize};

/// A Groth16 proof over BN254 in Circom-compatible encoding.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Groth16Proof {
    pub a: G1Bytes,
    pub b: G2Bytes,
    pub c: G1Bytes,
}

impl Groth16Proof {
    pub fn new(a: G1Bytes, b: G2Bytes, c: G1Bytes) -> Self {
        Self { a, b, c }
    }
}

/// The ZK-proof-mode `EphemeralCertificate` payload.
///
/// Carries the Groth16 proof itself, plus three optional commitments that
/// influence the public-input hash and verification:
///   * `extra_field` — an extra `"key":"value"` pair from the JWT that the
///     proof reveals to the chain (e.g. `"family_name":"Doe"`).
///   * `override_aud_val` — used by account recovery flows.
///   * `training_wheels_signature` — an additional Ed25519 signature over the
///     proof bytes by a network-operator key, gated by [`Configuration`].
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct ZeroKnowledgeSig {
    pub proof: ZkProof,
    pub exp_horizon_secs: u64,
    pub extra_field: Option<String>,
    pub override_aud_val: Option<String>,
    pub training_wheels_signature: Option<EphemeralSignature>,
}
