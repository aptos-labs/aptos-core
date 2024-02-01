// Copyright © Aptos Foundation

use self::jwk::JWK;
use crate::{move_utils::as_move_value::AsMoveValue, on_chain_config::OnChainConfig};
use anyhow::{bail, Context};
use aptos_crypto::bls12381;
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use jwk::JWKMoveStruct;
use move_core_types::{
    account_address::AccountAddress,
    ident_str,
    identifier::IdentStr,
    move_resource::MoveStructType,
    value::{MoveStruct, MoveValue},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeSet, HashMap},
    fmt::{Debug, Formatter},
};

pub mod jwk;
pub mod rsa;
pub mod unsupported;

pub type Issuer = Vec<u8>;

pub fn issuer_from_str(s: &str) -> Issuer {
    s.as_bytes().to_vec()
}

/// Move type `0x1::jwks::OIDCProvider` in rust.
/// See its doc in Move for more details.
#[derive(Default, Serialize, Deserialize)]
pub struct OIDCProvider {
    pub name: Issuer,
    pub config_url: Vec<u8>,
}

/// Move type `0x1::jwks::SupportedOIDCProviders` in rust.
/// See its doc in Move for more details.
#[derive(Default, Serialize, Deserialize)]
pub struct SupportedOIDCProviders {
    pub providers: Vec<OIDCProvider>,
}

impl SupportedOIDCProviders {
    pub fn into_provider_vec(self) -> Vec<OIDCProvider> {
        self.providers
    }
}

impl OnChainConfig for SupportedOIDCProviders {
    const MODULE_IDENTIFIER: &'static str = "jwks";
    const TYPE_IDENTIFIER: &'static str = "SupportedOIDCProviders";
}

/// Move type `0x1::jwks::ProviderJWKs` in rust.
/// See its doc in Move for more details.
#[derive(Clone, Default, Eq, PartialEq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub struct ProviderJWKs {
    pub issuer: Issuer,
    pub version: u64,
    pub jwks: Vec<JWKMoveStruct>,
}

impl ProviderJWKs {
    pub fn new(issuer: Issuer) -> Self {
        Self {
            issuer,
            version: 0,
            jwks: vec![],
        }
    }
}

impl Debug for ProviderJWKs {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderJWKs")
            .field("issuer", &String::from_utf8(self.issuer.clone()))
            .field("version", &self.version)
            .field("jwks", &self.jwks)
            .finish()
    }
}

impl ProviderJWKs {
    pub fn jwks(&self) -> &Vec<JWKMoveStruct> {
        &self.jwks
    }

    pub fn get_jwk(&self, id: &str) -> anyhow::Result<&JWKMoveStruct> {
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

impl AsMoveValue for ProviderJWKs {
    fn as_move_value(&self) -> MoveValue {
        MoveValue::Struct(MoveStruct::Runtime(vec![
            self.issuer.as_move_value(),
            self.version.as_move_value(),
            self.jwks.as_move_value(),
        ]))
    }
}
/// Move type `0x1::jwks::JWKs` in rust.
/// See its doc in Move for more details.
#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct AllProvidersJWKs {
    pub entries: Vec<ProviderJWKs>,
}

impl From<AllProvidersJWKs> for HashMap<Issuer, ProviderJWKs> {
    fn from(value: AllProvidersJWKs) -> Self {
        let AllProvidersJWKs { entries } = value;
        entries
            .into_iter()
            .map(|entry| (entry.issuer.clone(), entry))
            .collect()
    }
}

/// Move type `0x1::jwks::ObservedJWKs` in rust.
/// See its doc in Move for more details.
#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct ObservedJWKs {
    pub jwks: AllProvidersJWKs,
}

impl ObservedJWKs {
    pub fn into_providers_jwks(self) -> AllProvidersJWKs {
        let Self { jwks } = self;
        jwks
    }
}

impl OnChainConfig for ObservedJWKs {
    const MODULE_IDENTIFIER: &'static str = "jwks";
    const TYPE_IDENTIFIER: &'static str = "ObservedJWKs";
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

    pub fn get_jwk(&self, iss: &str, kid: &str) -> anyhow::Result<&JWKMoveStruct> {
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

/// A JWK update in format of `ProviderJWKs` and a multi-signature of it as a quorum certificate.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub struct QuorumCertifiedUpdate {
    pub authors: BTreeSet<AccountAddress>,
    pub update: ProviderJWKs,
    pub multi_sig: bls12381::Signature,
}

impl QuorumCertifiedUpdate {
    #[cfg(any(test, feature = "fuzzing"))]
    pub fn dummy() -> Self {
        Self {
            authors: Default::default(),
            update: Default::default(),
            multi_sig: bls12381::Signature::dummy_signature(),
        }
    }
}

/// Move event type `0x1::jwks::ObservedJWKsUpdated` in rust.
/// See its doc in Move for more details.
#[derive(Serialize, Deserialize)]
pub struct ObservedJWKsUpdated {
    pub epoch: u64,
    pub jwks: AllProvidersJWKs,
}

impl MoveStructType for ObservedJWKsUpdated {
    const MODULE_NAME: &'static IdentStr = ident_str!("jwks");
    const STRUCT_NAME: &'static IdentStr = ident_str!("ObservedJWKsUpdated");
}
