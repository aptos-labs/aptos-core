// Copyright © Aptos Foundation

use crate::{
    jwks::{rsa::RSA_JWK, unsupported::UnsupportedJWK},
    move_any::{Any as MoveAny, AsMoveAny},
};
use aptos_crypto::bls12381;
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

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

/// Reflection of Move type `0x1::jwks::JWK`.
/// When you load an on-chain config that contains some JWK(s), the JWK will be of this type.
/// When you call a Move function from rust that takes some JWKs as input, pass in JWKs of this type.
/// Otherwise, it is recommended to convert this to the rust enum `JWK` for better rust experience.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub struct JWKMoveStruct {
    variant: MoveAny,
}

pub enum JWK {
    RSA(RSA_JWK),
    Unsupported(UnsupportedJWK),
}

impl From<JWK> for JWKMoveStruct {
    fn from(jwk: JWK) -> Self {
        let variant = match jwk {
            JWK::RSA(variant) => variant.as_move_any(),
            JWK::Unsupported(variant) => variant.as_move_any(),
        };
        JWKMoveStruct { variant }
    }
}

impl From<JWKMoveStruct> for JWK {
    fn from(value: JWKMoveStruct) -> Self {
        match value.variant.type_name.as_str() {
            RSA_JWK::MOVE_TYPE_NAME => {
                let rsa_jwk = MoveAny::unpack(RSA_JWK::MOVE_TYPE_NAME, value.variant).unwrap();
                Self::RSA(rsa_jwk)
            },
            UnsupportedJWK::MOVE_TYPE_NAME => {
                let unsupported_jwk =
                    MoveAny::unpack(UnsupportedJWK::MOVE_TYPE_NAME, value.variant).unwrap();
                Self::Unsupported(unsupported_jwk)
            },
            _ => unreachable!(),
        }
    }
}

/// Move type `0x1::jwks::ProviderJWKs` in rust.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
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
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct AllProvidersJWKs {
    pub entries: Vec<ProviderJWKs>,
}

/// Move type `0x1::jwks::ObservedJWKs` in rust.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct ObservedJWKs {
    pub jwks: AllProvidersJWKs,
}

/// Reflection of Move type `0x1::jwks::ObservedJWKs`.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct PatchedJWKs {
    pub jwks: AllProvidersJWKs,
}

pub mod rsa;
pub mod unsupported;
