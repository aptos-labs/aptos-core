// Copyright © Aptos Foundation

use jwk::JWKMoveStruct;
use serde::{Deserialize, Serialize};

pub mod jwk;
pub mod rsa;
pub mod unsupported;

pub type Issuer = Vec<u8>;

pub fn issuer_from_str(s: &str) -> Issuer {
    s.as_bytes().to_vec()
}

/// Move type `0x1::jwks::OIDCProvider` in rust.
pub struct OIDCProvider {
    pub name: Issuer,
    pub config_url: Vec<u8>,
}

/// Move type `0x1::jwks::SupportedOIDCProviders` in rust.
pub struct SupportedOIDCProviders {
    pub providers: Vec<OIDCProvider>,
}

/// Move type `0x1::jwks::ProviderJWKs` in rust.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ProviderJWKs {
    pub issuer: Issuer,
    pub version: u64,
    pub jwks: Vec<JWKMoveStruct>,
}

impl ProviderJWKs {
    pub fn jwks(&self) -> &Vec<JWKMoveStruct> {
        &self.jwks
    }
}

/// Move type `0x1::jwks::JWKs` in rust.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct AllProvidersJWKs {
    pub entries: Vec<ProviderJWKs>,
}

/// Move type `0x1::jwks::ObservedJWKs` in rust.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct ObservedJWKs {
    pub jwks: AllProvidersJWKs,
}

/// Reflection of Move type `0x1::jwks::ObservedJWKs`.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct PatchedJWKs {
    pub jwks: AllProvidersJWKs,
}
