// Copyright © Aptos Foundation

use crate::{
    aggregate_signature::AggregateSignature, move_utils::as_move_value::AsMoveValue,
    on_chain_config::OnChainConfig, validator_verifier::ValidatorVerifier,
};
use anyhow::{ensure, Result};
use aptos_bitvec::BitVec;
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
#[derive(Default, Serialize, Deserialize)]
pub struct OIDCProvider {
    pub name: Issuer,
    pub config_url: Vec<u8>,
}

/// Move type `0x1::jwks::SupportedOIDCProviders` in rust.
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub struct QuorumCertifiedUpdate {
    pub authors: BTreeSet<AccountAddress>,
    pub observed: ProviderJWKs,
    pub multi_sig: bls12381::Signature,
}

/// Verify a quorum-certified JWK update by verifying its multi-sig and check the version is expected.
/// Return the update payload if verification succeeded.
///
/// Used in VM to execute JWK validator transactions.
pub fn verify_jwk_qc_update(
    verifier: &ValidatorVerifier,
    on_chain: &ProviderJWKs,
    qc_update: QuorumCertifiedUpdate,
) -> Result<ProviderJWKs> {
    let QuorumCertifiedUpdate {
        authors,
        observed,
        multi_sig,
    } = qc_update;
    let signer_bit_vec = BitVec::from(
        verifier
            .get_ordered_account_addresses()
            .into_iter()
            .map(|addr| authors.contains(&addr))
            .collect::<Vec<_>>(),
    );
    ensure!(
        on_chain.version + 1 == observed.version,
        "verify_jwk_qc_update failed with unexpected version"
    );
    verifier.verify_multi_signatures(
        &observed,
        &AggregateSignature::new(signer_bit_vec, Some(multi_sig)),
    )?;
    verifier.check_voting_power(authors.iter(), true)?;
    Ok(observed)
}

#[derive(Serialize, Deserialize)]
pub struct ObservedJWKsUpdated {
    pub epoch: u64,
    pub jwks: AllProvidersJWKs,
}

impl MoveStructType for ObservedJWKsUpdated {
    const MODULE_NAME: &'static IdentStr = ident_str!("jwks");
    const STRUCT_NAME: &'static IdentStr = ident_str!("ObservedJWKsUpdated");
}
