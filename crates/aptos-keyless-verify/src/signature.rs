// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
//
// Vendored from aptos-core/types/src/keyless/mod.rs @ rev 8ec3fb76.

use crate::{
    ephemeral::{EphemeralPublicKey, EphemeralSignature},
    errors::VerifyError,
    groth16_sig::ZeroKnowledgeSig,
    openid_sig::OpenIdSig,
};
use serde::{Deserialize, Serialize};

/// The "certificate" tying the ephemeral public key to a JWT identity — either
/// a ZK-proof-of-knowledge of a JWT signature (production) or the raw JWT
/// signature itself (used historically, before ZKP support).
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum EphemeralCertificate {
    ZeroKnowledgeSig(ZeroKnowledgeSig),
    OpenIdSig(OpenIdSig),
}

/// A keyless signature.
///
/// Wire layout: BCS-compatible with `aptos_types::keyless::KeylessSignature`.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct KeylessSignature {
    /// `EphemeralCertificate::ZeroKnowledgeSig(...)` for ZKP mode (the
    /// production path), or `EphemeralCertificate::OpenIdSig(...)` for the
    /// legacy OpenID path.
    pub cert: EphemeralCertificate,

    /// Plaintext JWT header JSON. Read for the `kid` (key id) and `alg`.
    pub jwt_header_json: String,

    /// UNIX seconds; signature is invalid after this time.
    pub exp_date_secs: u64,

    /// Public key under which `ephemeral_signature` verifies.
    pub ephemeral_pubkey: EphemeralPublicKey,

    /// Signature by the ephemeral key over the user's signing message.
    pub ephemeral_signature: EphemeralSignature,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JwtHeader {
    pub kid: String,
    pub alg: String,
}

impl KeylessSignature {
    /// Parse from BCS bytes.
    pub fn from_bcs_bytes(bytes: &[u8]) -> Result<Self, VerifyError> {
        bcs::from_bytes::<KeylessSignature>(bytes)
            .map_err(|e| VerifyError::Decode(format!("KeylessSignature: {}", e)))
    }

    /// Parse `jwt_header_json` and return the `kid` claim.
    pub fn jwt_kid(&self) -> Result<String, VerifyError> {
        let header: JwtHeader = serde_json::from_str(&self.jwt_header_json)
            .map_err(|e| VerifyError::Decode(format!("jwt_header_json: {}", e)))?;
        Ok(header.kid)
    }
}
