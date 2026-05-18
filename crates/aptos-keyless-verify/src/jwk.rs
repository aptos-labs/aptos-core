// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
//
// Vendored from aptos-core/types/src/jwks/rsa/mod.rs @ rev 8ec3fb76.

use serde::{Deserialize, Serialize};

/// RSA JWK. Mirrors the JSON shape published on-chain at
/// `0x1::jwks::AllProvidersJWKs` (and for FederatedKeyless, at a user-supplied
/// resource address).
#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct RsaJwk {
    pub kid: String,
    pub kty: String,
    pub alg: String,
    /// Public exponent, base64url-encoded (typically `"AQAB"` for 65537).
    pub e: String,
    /// Public modulus, base64url-encoded.
    pub n: String,
}

impl RsaJwk {
    pub const RSA_MODULUS_BYTES: usize = 256;
}
