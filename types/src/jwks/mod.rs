// Copyright © Aptos Foundation

use self::jwk::JWK;
use anyhow::{bail, Context, Ok, Result};
use jwk::JWKMoveStruct;
use move_core_types::{ident_str, identifier::IdentStr, move_resource::MoveStructType};
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

    pub fn get_jwk(&self, id: &str) -> Result<&JWKMoveStruct> {
        for jwk_move in self.jwks() {
            let jwk = JWK::try_from(jwk_move)?;
            match jwk {
                JWK::RSA(rsa_jwk) => {
                    if rsa_jwk.kid.eq(id) {
                        return Ok(jwk_move);
                    }
                },
                JWK::Unsupported(unsupported_jwk) => {
                    if unsupported_jwk.id.eq(id.as_bytes()) {
                        return Ok(jwk_move);
                    }
                },
            }
        }
        bail!("JWK with id {} not found", id);
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

impl PatchedJWKs {
    pub fn get_provider_jwks(&self, iss: &str) -> Option<&ProviderJWKs> {
        self.jwks
            .entries
            .iter()
            .find(|&provider_jwk_set| provider_jwk_set.issuer.eq(&issuer_from_str(iss)))
    }

    pub fn get_jwk(&self, iss: &str, kid: &str) -> Result<&JWKMoveStruct> {
        let provider_jwk_set = self
            .get_provider_jwks(iss)
            .context("JWK not found for issuer")?;
        let jwk = provider_jwk_set.get_jwk(kid)?;
        Ok(jwk)
    }
}

impl MoveStructType for PatchedJWKs {
    const MODULE_NAME: &'static IdentStr = ident_str!("jwks");
    const STRUCT_NAME: &'static IdentStr = ident_str!("PatchedJWKs");
}
