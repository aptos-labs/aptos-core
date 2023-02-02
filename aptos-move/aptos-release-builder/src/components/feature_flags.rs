// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::utils::*;
use anyhow::Result;
use aptos_types::on_chain_config::{FeatureFlag as AptosFeatureFlag, Features as AptosFeatures};
use move_model::{code_writer::CodeWriter, emit, emitln, model::Loc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct Features {
    pub enabled: Vec<FeatureFlag>,
    pub disabled: Vec<FeatureFlag>,
}

#[derive(Clone, Deserialize, PartialEq, Eq, Serialize)]
#[allow(non_camel_case_types)]
#[serde(rename_all = "snake_case")]
pub enum FeatureFlag {
    CodeDependencyCheck,
    CollectAndDistributeGasFees,
    TreatFriendAsPrivate,
    Sha512AndRipeMd160Natives,
    AptosStdChainIdNatives,
    VMBinaryFormatV6,
    MultiEd25519PkValidateV2Natives,
    Blake2b256Native,
    ResourceGroups,
}

fn generate_features_blob(writer: &CodeWriter, data: &[u64]) {
    emitln!(writer, "vector[");
    writer.indent();
    for (i, b) in data.iter().enumerate() {
        if i % 20 == 0 {
            if i > 0 {
                emitln!(writer);
            }
        } else {
            emit!(writer, " ");
        }
        emit!(writer, "{},", b);
    }
    emitln!(writer);
    writer.unindent();
    emit!(writer, "]")
}

pub fn generate_feature_upgrade_proposal(
    features: &Features,
    is_testnet: bool,
    next_execution_hash: Vec<u8>,
) -> Result<Vec<(String, String)>> {
    let mut result = vec![];

    let enabled = features
        .enabled
        .iter()
        .map(|f| AptosFeatureFlag::from(f.clone()) as u64)
        .collect::<Vec<_>>();
    let disabled = features
        .disabled
        .iter()
        .map(|f| AptosFeatureFlag::from(f.clone()) as u64)
        .collect::<Vec<_>>();

    assert!(enabled.len() < u16::MAX as usize);
    assert!(disabled.len() < u16::MAX as usize);

    let writer = CodeWriter::new(Loc::default());

    let proposal = generate_governance_proposal(
        &writer,
        is_testnet,
        next_execution_hash.clone(),
        "std::features",
        |writer| {
            emit!(writer, "let enabled_blob: vector<u64> = ");
            generate_features_blob(writer, &enabled);
            emitln!(writer, ";\n");

            emit!(writer, "let disabled_blob: vector<u64> = ");
            generate_features_blob(writer, &disabled);
            emitln!(writer, ";\n");

            if is_testnet && next_execution_hash.is_empty() {
                emitln!(
                    writer,
                    "features::change_feature_flags(framework_signer, enabled_blob, disabled_blob);"
                );
            } else {
                emitln!(
                    writer,
                    "features::change_feature_flags(&framework_signer, enabled_blob, disabled_blob);"
                );
            }
        },
    );

    result.push(("features".to_string(), proposal));
    Ok(result)
}

impl From<FeatureFlag> for AptosFeatureFlag {
    fn from(f: FeatureFlag) -> Self {
        match f {
            FeatureFlag::CodeDependencyCheck => AptosFeatureFlag::CODE_DEPENDENCY_CHECK,
            FeatureFlag::CollectAndDistributeGasFees => {
                AptosFeatureFlag::COLLECT_AND_DISTRIBUTE_GAS_FEES
            },
            FeatureFlag::TreatFriendAsPrivate => AptosFeatureFlag::TREAT_FRIEND_AS_PRIVATE,
            FeatureFlag::Sha512AndRipeMd160Natives => {
                AptosFeatureFlag::SHA_512_AND_RIPEMD_160_NATIVES
            },
            FeatureFlag::AptosStdChainIdNatives => AptosFeatureFlag::APTOS_STD_CHAIN_ID_NATIVES,
            FeatureFlag::VMBinaryFormatV6 => AptosFeatureFlag::VM_BINARY_FORMAT_V6,
            FeatureFlag::MultiEd25519PkValidateV2Natives => {
                AptosFeatureFlag::MULTI_ED25519_PK_VALIDATE_V2_NATIVES
            },
            FeatureFlag::Blake2b256Native => AptosFeatureFlag::BLAKE2B_256_NATIVE,
            FeatureFlag::ResourceGroups => AptosFeatureFlag::RESOURCE_GROUPS,
        }
    }
}

// We don't need this implementation. Just to make sure we have an exhaustive 1-1 mapping between the two structs.
impl From<AptosFeatureFlag> for FeatureFlag {
    fn from(f: AptosFeatureFlag) -> Self {
        match f {
            AptosFeatureFlag::CODE_DEPENDENCY_CHECK => FeatureFlag::CodeDependencyCheck,
            AptosFeatureFlag::COLLECT_AND_DISTRIBUTE_GAS_FEES => {
                FeatureFlag::CollectAndDistributeGasFees
            },
            AptosFeatureFlag::TREAT_FRIEND_AS_PRIVATE => FeatureFlag::TreatFriendAsPrivate,
            AptosFeatureFlag::SHA_512_AND_RIPEMD_160_NATIVES => {
                FeatureFlag::Sha512AndRipeMd160Natives
            },
            AptosFeatureFlag::APTOS_STD_CHAIN_ID_NATIVES => FeatureFlag::AptosStdChainIdNatives,
            AptosFeatureFlag::VM_BINARY_FORMAT_V6 => FeatureFlag::VMBinaryFormatV6,
            AptosFeatureFlag::MULTI_ED25519_PK_VALIDATE_V2_NATIVES => {
                FeatureFlag::MultiEd25519PkValidateV2Natives
            },
            AptosFeatureFlag::BLAKE2B_256_NATIVE => FeatureFlag::Blake2b256Native,
            AptosFeatureFlag::RESOURCE_GROUPS => FeatureFlag::ResourceGroups,
        }
    }
}

impl Features {
    // Compare if the current feature set is different from features that has been enabled on chain.
    pub(crate) fn has_modified(&self, on_chain_features: &AptosFeatures) -> bool {
        self.enabled
            .iter()
            .any(|f| !on_chain_features.is_enabled(AptosFeatureFlag::from(f.clone())))
            || self
                .disabled
                .iter()
                .any(|f| on_chain_features.is_enabled(AptosFeatureFlag::from(f.clone())))
    }
}
