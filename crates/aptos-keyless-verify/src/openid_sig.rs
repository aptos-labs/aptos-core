// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
//
// Vendored from aptos-core/types/src/keyless/openid_sig.rs @ rev 8ec3fb76.

use serde::{Deserialize, Serialize};

/// The OpenID (non-ZKP) mode of `EphemeralCertificate`. Carries a raw JWT
/// signature + the precise byte spans of the relevant claims so the verifier
/// can recompute the OAuth nonce and check it matches the EPK.
///
/// BCS layout matches `aptos_types::keyless::OpenIdSig`.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct OpenIdSig {
    /// RSA signature over `b64url(jwt_header_json) || "." || jwt_payload_b64`.
    #[serde(with = "serde_bytes")]
    pub jwt_sig: Vec<u8>,

    /// The base64url-decoded JWT payload JSON.
    pub jwt_payload_json: String,

    /// Field of the JWT claims that identifies the user (typically `"sub"`).
    pub uid_key: String,

    /// Byte index in the JWT JSON where the EPK commitment lives.
    pub epk_blinder: Vec<u8>,

    /// Pepper used to derive the IDC.
    pub pepper: crate::public_key::Pepper,

    /// Optional override of the `aud` value (used by account-recovery flows).
    pub idc_aud_val: Option<String>,
}

/// Top-level OIDC claims that we surface from the JWT for verification.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OidcClaims {
    pub iss: String,
    pub aud: String,
    pub sub: String,
    pub nonce: String,
    pub iat: u64,
    pub exp: u64,
}

/// Full deserialized JWT claims. `oidc_claims` carries the required fields;
/// the rest is captured in `additional_claims` for downstream inspection
/// (e.g. `email`, `name`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Claims {
    #[serde(flatten)]
    pub oidc_claims: OidcClaims,

    #[serde(flatten)]
    pub additional_claims: serde_json::Map<String, serde_json::Value>,
}
