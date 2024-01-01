// Copyright © Aptos Foundation

use crate::move_any::{Any, AsMoveAny};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use aptos_crypto::bls12381;
use move_core_types::account_address::AccountAddress;
use crate::jwks::rsa::RSA_JWK;
use crate::jwks::unsupported::UnsupportedJWK;

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

/// Move type `0x1::jwks::JWK` in rust.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub struct JWK {
    variant: Any,
}

impl JWK {
    pub fn new_rsa(rsa: RSA_JWK) -> Self {
        Self {
            variant: rsa.as_move_any()
        }
    }

    pub fn new_unsupported(unsupported: UnsupportedJWK) -> Self {
        Self {
            variant: unsupported.as_move_any()
        }
    }
}

/// Move type `0x1::jwks::ProviderJWKs` in rust.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub struct ProviderJWKs {
    pub issuer: Issuer,
    pub version: u64,
    pub jwks: Vec<JWK>,
}

impl ProviderJWKs {
    pub fn jwks(&self) -> &Vec<JWK> {
        &self.jwks
    }
}

/// Move type `0x1::jwks::JWKs` in rust.
pub struct JWKs {
    pub entries: Vec<ProviderJWKs>,
}

/// Move type `0x1::jwks::ObservedJWKs` in rust.
pub struct ObservedJWKs {
    pub jwks: JWKs,
}

pub mod rsa;
pub mod unsupported;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub struct QuorumCertifiedUpdate {
    pub authors: BTreeSet<AccountAddress>,
    pub observed: ProviderJWKs,
    pub multi_sig: bls12381::Signature,
}
