// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Rust mirrors of `0x1::trust_observation` and the validator-certified update.
//!
//! An observation is opaque to consensus after a target-specific observer has
//! canonicalized it. The signed payload includes both the target and the
//! configuration version, so signatures cannot be replayed for another target
//! or after the target's URL or canonicalizer has changed.

use crate::{
    aggregate_signature::AggregateSignature, move_utils::as_move_value::AsMoveValue,
    on_chain_config::OnChainConfig,
};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use move_core_types::{
    ident_str,
    identifier::IdentStr,
    move_resource::MoveStructType,
    value::{MoveStruct, MoveValue},
};
use serde::{Deserialize, Serialize};

pub type TargetId = Vec<u8>;

/// Defense-in-depth cap for a single observed value, even if a target is
/// misconfigured with a larger `max_bytes` limit.
pub const MAX_VALUE_BYTES: usize = 60 * 1024;

pub const FORMAT_DER_CERTIFICATE: u8 = 1;
pub const FORMAT_CANONICAL_JSON: u8 = 2;
pub const FORMAT_RAW_BYTES: u8 = 3;
pub const PATCH_REPLACE: u8 = 1;
pub const PATCH_REMOVE: u8 = 2;

/// Configuration for one validator-observed HTTPS source.
/// Must match the field order of `trust_observation::TargetSpec`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TargetSpec {
    pub id: TargetId,
    pub url: String,
    pub format: u8,
    pub max_bytes: u64,
    pub config_version: u64,
}

impl AsMoveValue for TargetSpec {
    fn as_move_value(&self) -> MoveValue {
        MoveValue::Struct(MoveStruct::Runtime(vec![
            self.id.as_move_value(),
            self.url.as_move_value(),
            self.format.as_move_value(),
            self.max_bytes.as_move_value(),
            self.config_version.as_move_value(),
        ]))
    }
}

/// On-chain target registry, at `0x1::trust_observation::Config`.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub targets: Vec<TargetSpec>,
}

impl Config {
    pub fn get(&self, id: &[u8]) -> Option<&TargetSpec> {
        self.targets
            .iter()
            .find(|target| target.id.as_slice() == id)
    }
}

impl OnChainConfig for Config {
    const MODULE_IDENTIFIER: &'static str = "trust_observation";
    const TYPE_IDENTIFIER: &'static str = "Config";
}

impl AsMoveValue for Config {
    fn as_move_value(&self) -> MoveValue {
        MoveValue::Struct(MoveStruct::Runtime(vec![self.targets.as_move_value()]))
    }
}

/// An observed, canonicalized byte value for one configured target.
/// The complete struct is BLS-signed by validators as the consensus proposal.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub struct ObservedValue {
    pub target_id: TargetId,
    pub config_version: u64,
    pub epoch: u64,
    pub version: u64,
    pub value: Vec<u8>,
}

impl AsMoveValue for ObservedValue {
    fn as_move_value(&self) -> MoveValue {
        MoveValue::Struct(MoveStruct::Runtime(vec![
            self.target_id.as_move_value(),
            self.config_version.as_move_value(),
            self.epoch.as_move_value(),
            self.version.as_move_value(),
            self.value.as_move_value(),
        ]))
    }
}

/// Current observations at `0x1::trust_observation::ObservedTrustData`.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ObservedTrustData {
    pub entries: Vec<ObservedValue>,
}

impl ObservedTrustData {
    pub fn get(&self, id: &[u8]) -> Option<&ObservedValue> {
        self.entries
            .iter()
            .find(|entry| entry.target_id.as_slice() == id)
    }
}

impl OnChainConfig for ObservedTrustData {
    const MODULE_IDENTIFIER: &'static str = "trust_observation";
    const TYPE_IDENTIFIER: &'static str = "ObservedTrustData";
}

impl AsMoveValue for ObservedTrustData {
    fn as_move_value(&self) -> MoveValue {
        MoveValue::Struct(MoveStruct::Runtime(vec![self.entries.as_move_value()]))
    }
}

/// Governance override. `kind` is PATCH_REPLACE or PATCH_REMOVE.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Patch {
    pub target_id: TargetId,
    pub config_version: u64,
    pub kind: u8,
    pub value: Vec<u8>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct Patches {
    pub entries: Vec<Patch>,
}

impl OnChainConfig for Patches {
    const MODULE_IDENTIFIER: &'static str = "trust_observation";
    const TYPE_IDENTIFIER: &'static str = "Patches";
}

/// Final values after applying governance patches. This is what attestation
/// verifiers should read rather than `ObservedTrustData`.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct EffectiveTrustData {
    pub entries: Vec<ObservedValue>,
}

impl EffectiveTrustData {
    pub fn get(&self, id: &[u8]) -> Option<&ObservedValue> {
        self.entries
            .iter()
            .find(|entry| entry.target_id.as_slice() == id)
    }
}

impl OnChainConfig for EffectiveTrustData {
    const MODULE_IDENTIFIER: &'static str = "trust_observation";
    const TYPE_IDENTIFIER: &'static str = "EffectiveTrustData";
}

/// Certificate for a single observed value, signed by an epoch's validator quorum.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub struct QuorumCertifiedUpdate {
    pub update: ObservedValue,
    pub multi_sig: AggregateSignature,
}

/// Module event emitted when a target's observed value changes.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ObservedTrustDataUpdated {
    pub observed: ObservedValue,
}

impl MoveStructType for ObservedTrustDataUpdated {
    const MODULE_NAME: &'static IdentStr = ident_str!("trust_observation");
    const STRUCT_NAME: &'static IdentStr = ident_str!("ObservedTrustDataUpdated");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_and_rust_bcs_layouts_agree() {
        let target = TargetSpec {
            id: b"intel-root".to_vec(),
            url: "https://example.com/root.cer".to_owned(),
            format: FORMAT_DER_CERTIFICATE,
            max_bytes: 4096,
            config_version: 3,
        };
        let config = Config {
            targets: vec![target],
        };
        assert_eq!(
            bcs::to_bytes(&config).unwrap(),
            config.as_move_value().simple_serialize().unwrap()
        );

        let observed = ObservedValue {
            target_id: b"intel-root".to_vec(),
            config_version: 3,
            epoch: 42,
            version: 9,
            value: vec![0x30, 0x01, 0x00],
        };
        assert_eq!(
            bcs::to_bytes(&observed).unwrap(),
            observed.as_move_value().simple_serialize().unwrap()
        );
    }

    #[test]
    fn target_lookup_is_exact() {
        let config = Config {
            targets: vec![TargetSpec {
                id: b"intel-root".to_vec(),
                url: "https://example.com/root.cer".to_owned(),
                format: FORMAT_DER_CERTIFICATE,
                max_bytes: 4096,
                config_version: 1,
            }],
        };
        assert!(config.get(b"intel-root").is_some());
        assert!(config.get(b"intel-root-extra").is_none());
    }
}
