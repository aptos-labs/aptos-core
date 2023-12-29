// Copyright © Aptos Foundation

use crate::move_any::{Any};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use serde::{Deserialize, Serialize};

pub type Issuer = Vec<u8>;

/// Move type `0x1::jwks::OIDCProvider` in rust.
pub struct OIDCProvider {
    pub name: Issuer,
    pub config_url: Vec<u8>,
}

/// Move type `0x1::jwks::SupportedOIDCProviders` in rust.
pub struct SupportedOIDCProviders {
    pub providers: Vec<OIDCProvider>,
}

/// Move type `0x1::jwks::JWK` in rust.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub struct JWK {
    variant: Any,
}

/// Move type `0x1::jwks::ProviderJWKs` in rust.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub struct ProviderJWKs {
    pub issuer: Issuer,
    pub version: u64,
    pub jwks: Vec<JWK>,
}

/// Move type `0x1::jwks::JWKs` in rust.
pub struct JWKs {
    pub entries: Vec<ProviderJWKs>,
}

pub mod rsa;
pub mod unsupported;
