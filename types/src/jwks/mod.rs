// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use self::{
    jwk::JWK,
    rsa::{INSECURE_TEST_RSA_JWK, RSA_JWK, SECURE_TEST_RSA_JWK},
};
use crate::{
    aggregate_signature::AggregateSignature, move_utils::as_move_value::AsMoveValue,
    on_chain_config::OnChainConfig,
};
use anyhow::{bail, Context};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use jwk::JWKMoveStruct;
use move_core_types::{
    ident_str,
    identifier::IdentStr,
    language_storage::TypeTag,
    move_resource::MoveStructType,
    value::{MoveStruct, MoveValue},
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt::{Debug, Formatter},
};

pub mod jwk;
pub mod patch;
pub mod rsa;
pub mod unsupported;

pub type Issuer = Vec<u8>;

pub fn secure_test_rsa_jwk() -> RSA_JWK {
    SECURE_TEST_RSA_JWK.clone()
}

pub fn insecure_test_rsa_jwk() -> RSA_JWK {
    INSECURE_TEST_RSA_JWK.clone()
}

pub fn issuer_from_str(s: &str) -> Issuer {
    s.as_bytes().to_vec()
}

#[cfg(any(test, feature = "fuzzing"))]
pub fn dummy_issuer() -> Issuer {
    issuer_from_str("https:://dummy.issuer")
}

/// Move type `0x1::jwks::OIDCProvider` in rust.
/// See its doc in Move for more details.
#[derive(Default, Serialize, Deserialize)]
pub struct OIDCProvider {
    pub name: Issuer,
    pub config_url: Vec<u8>,
}

impl OIDCProvider {
    pub fn new(name: String, config_url: String) -> Self {
        Self {
            name: name.as_bytes().to_vec(),
            config_url: config_url.as_bytes().to_vec(),
        }
    }
}

impl From<crate::on_chain_config::OIDCProvider> for OIDCProvider {
    fn from(value: crate::on_chain_config::OIDCProvider) -> Self {
        OIDCProvider {
            name: value.name.as_bytes().to_vec(),
            config_url: value.config_url.as_bytes().to_vec(),
        }
    }
}

impl TryFrom<OIDCProvider> for crate::on_chain_config::OIDCProvider {
    type Error = anyhow::Error;

    fn try_from(value: OIDCProvider) -> Result<Self, Self::Error> {
        let OIDCProvider { name, config_url } = value;
        let name = String::from_utf8(name)?;
        let config_url = String::from_utf8(config_url)?;
        Ok(crate::on_chain_config::OIDCProvider { name, config_url })
    }
}

impl Debug for OIDCProvider {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OIDCProvider")
            .field("name", &String::from_utf8(self.name.clone()))
            .field("config_url", &String::from_utf8(self.config_url.clone()))
            .finish()
    }
}
/// Move type `0x1::jwks::SupportedOIDCProviders` in rust.
/// See its doc in Move for more details.
#[derive(Debug, Default, Serialize, Deserialize)]
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
    #[serde(with = "serde_bytes")]
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

impl AllProvidersJWKs {
    pub fn get_provider_jwks(&self, iss: &str) -> Option<&ProviderJWKs> {
        self.entries
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

/// Reflection of Move type `0x1::jwks::PatchedJWKs`.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct PatchedJWKs {
    pub jwks: AllProvidersJWKs,
}

impl OnChainConfig for PatchedJWKs {
    const MODULE_IDENTIFIER: &'static str = "jwks";
    const TYPE_IDENTIFIER: &'static str = "PatchedJWKs";
}

/// Reflection of Move type `0x1::jwks::FederatedJWKs`.
#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct FederatedJWKs {
    pub jwks: AllProvidersJWKs,
}

impl MoveStructType for FederatedJWKs {
    const MODULE_NAME: &'static IdentStr = ident_str!("jwks");
    const STRUCT_NAME: &'static IdentStr = ident_str!("FederatedJWKs");
}

/// A JWK update in format of `ProviderJWKs` and a multi-signature of it as a quorum certificate.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub struct QuorumCertifiedUpdate {
    pub update: ProviderJWKs,
    pub multi_sig: AggregateSignature,
}

impl QuorumCertifiedUpdate {
    #[cfg(any(test, feature = "fuzzing"))]
    pub fn dummy() -> Self {
        Self {
            update: ProviderJWKs::new(dummy_issuer()),
            multi_sig: AggregateSignature::empty(),
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

pub static OBSERVED_JWK_UPDATED_MOVE_TYPE_TAG: Lazy<TypeTag> =
    Lazy::new(|| TypeTag::Struct(Box::new(ObservedJWKsUpdated::struct_tag())));
