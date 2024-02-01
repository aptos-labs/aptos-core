// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::utils::*;
use anyhow::Result;
use aptos_types::on_chain_config::{FeatureFlag as AptosFeatureFlag, Features as AptosFeatures};
use move_model::{code_writer::CodeWriter, emit, emitln, model::Loc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

#[derive(Clone, Deserialize, PartialEq, Eq, Serialize, Debug)]
pub struct Features {
    #[serde(default)]
    pub enabled: Vec<FeatureFlag>,
    #[serde(default)]
    pub disabled: Vec<FeatureFlag>,
}

impl Features {
    pub fn empty() -> Self {
        Self {
            enabled: vec![],
            disabled: vec![],
        }
    }

    pub fn squash(&mut self, rhs: Self) {
        let mut enabled: HashSet<_> = self.enabled.iter().cloned().collect();
        let mut disabled: HashSet<_> = self.disabled.iter().cloned().collect();
        let to_enable: HashSet<_> = rhs.enabled.into_iter().collect();
        let to_disable: HashSet<_> = rhs.disabled.into_iter().collect();

        disabled = disabled.difference(&to_enable).cloned().collect();
        enabled.extend(to_enable);

        enabled = enabled.difference(&to_disable).cloned().collect();
        disabled.extend(to_disable);

        self.enabled = enabled.into_iter().collect();
        self.disabled = disabled.into_iter().collect();
    }

    pub fn is_empty(&self) -> bool {
        self.enabled.is_empty() && self.disabled.is_empty()
    }
}

#[derive(Clone, Debug, Deserialize, EnumIter, PartialEq, Eq, Serialize, Hash)]
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
    MultisigAccounts,
    DelegationPools,
    CryptographyAlgebraNatives,
    Bls12381Structures,
    Ed25519PubkeyValidateReturnFalseWrongLength,
    StructConstructors,
    PeriodicalRewardRateReduction,
    PartialGovernanceVoting,
    SignatureCheckerV2,
    StorageSlotMetadata,
    ChargeInvariantViolation,
    DelegationPoolPartialGovernanceVoting,
    GasPayerEnabled,
    AptosUniqueIdentifiers,
    BulletproofsNatives,
    SignerNativeFormatFix,
    ModuleEvent,
    EmitFeeStatement,
    StorageDeletionRefund,
    AggregatorV2Api,
    SignatureCheckerV2ScriptFix,
    SaferResourceGroups,
    SaferMetadata,
    SingleSenderAuthenticator,
    SponsoredAutomaticAccountCreation,
    FeePayerAccountOptional,
    AggregatorV2DelayedFields,
    ConcurrentAssets,
    LimitMaxIdentifierLength,
    OperatorBeneficiaryChange,
    VMBinaryFormatV7,
    ResourceGroupsChargeAsSizeSum,
    CommissionChangeDelegationPool,
    BN254Structures,
    WebAuthnSignature,
    ReconfigureWithDKG,
    ZkIdSignature,
    ZkIdZkLessSignature,
    RemoveDetailedError,
    JWKConsensus,
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

    emitln!(writer, "// Modifying on-chain feature flags: ");
    emitln!(writer, "// Enabled Features: {:?}", features.enabled);
    emitln!(writer, "// Disabled Features: {:?}", features.disabled);
    emitln!(writer, "//");

    let proposal = generate_governance_proposal(
        &writer,
        is_testnet,
        next_execution_hash.clone(),
        &["std::features"],
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
                emitln!(writer, "aptos_governance::reconfigure(framework_signer);");
            } else {
                emitln!(
                    writer,
                    "features::change_feature_flags(&framework_signer, enabled_blob, disabled_blob);"
                );
                emitln!(writer, "aptos_governance::reconfigure(&framework_signer);");
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
            FeatureFlag::VMBinaryFormatV7 => AptosFeatureFlag::VM_BINARY_FORMAT_V7,
            FeatureFlag::MultiEd25519PkValidateV2Natives => {
                AptosFeatureFlag::MULTI_ED25519_PK_VALIDATE_V2_NATIVES
            },
            FeatureFlag::Blake2b256Native => AptosFeatureFlag::BLAKE2B_256_NATIVE,
            FeatureFlag::ResourceGroups => AptosFeatureFlag::RESOURCE_GROUPS,
            FeatureFlag::MultisigAccounts => AptosFeatureFlag::MULTISIG_ACCOUNTS,
            FeatureFlag::DelegationPools => AptosFeatureFlag::DELEGATION_POOLS,
            FeatureFlag::CryptographyAlgebraNatives => {
                AptosFeatureFlag::CRYPTOGRAPHY_ALGEBRA_NATIVES
            },
            FeatureFlag::Bls12381Structures => AptosFeatureFlag::BLS12_381_STRUCTURES,
            FeatureFlag::Ed25519PubkeyValidateReturnFalseWrongLength => {
                AptosFeatureFlag::ED25519_PUBKEY_VALIDATE_RETURN_FALSE_WRONG_LENGTH
            },
            FeatureFlag::StructConstructors => AptosFeatureFlag::STRUCT_CONSTRUCTORS,
            FeatureFlag::PeriodicalRewardRateReduction => {
                AptosFeatureFlag::PERIODICAL_REWARD_RATE_DECREASE
            },
            FeatureFlag::PartialGovernanceVoting => AptosFeatureFlag::PARTIAL_GOVERNANCE_VOTING,
            FeatureFlag::SignatureCheckerV2 => AptosFeatureFlag::SIGNATURE_CHECKER_V2,
            FeatureFlag::StorageSlotMetadata => AptosFeatureFlag::STORAGE_SLOT_METADATA,
            FeatureFlag::ChargeInvariantViolation => AptosFeatureFlag::CHARGE_INVARIANT_VIOLATION,
            FeatureFlag::DelegationPoolPartialGovernanceVoting => {
                AptosFeatureFlag::DELEGATION_POOL_PARTIAL_GOVERNANCE_VOTING
            },
            FeatureFlag::GasPayerEnabled => AptosFeatureFlag::GAS_PAYER_ENABLED,
            FeatureFlag::AptosUniqueIdentifiers => AptosFeatureFlag::APTOS_UNIQUE_IDENTIFIERS,
            FeatureFlag::BulletproofsNatives => AptosFeatureFlag::BULLETPROOFS_NATIVES,
            FeatureFlag::SignerNativeFormatFix => AptosFeatureFlag::SIGNER_NATIVE_FORMAT_FIX,
            FeatureFlag::ModuleEvent => AptosFeatureFlag::MODULE_EVENT,
            FeatureFlag::EmitFeeStatement => AptosFeatureFlag::EMIT_FEE_STATEMENT,
            FeatureFlag::StorageDeletionRefund => AptosFeatureFlag::STORAGE_DELETION_REFUND,
            FeatureFlag::AggregatorV2Api => AptosFeatureFlag::AGGREGATOR_V2_API,
            FeatureFlag::SignatureCheckerV2ScriptFix => {
                AptosFeatureFlag::SIGNATURE_CHECKER_V2_SCRIPT_FIX
            },
            FeatureFlag::SaferResourceGroups => AptosFeatureFlag::SAFER_RESOURCE_GROUPS,
            FeatureFlag::SaferMetadata => AptosFeatureFlag::SAFER_METADATA,
            FeatureFlag::SingleSenderAuthenticator => AptosFeatureFlag::SINGLE_SENDER_AUTHENTICATOR,
            FeatureFlag::SponsoredAutomaticAccountCreation => {
                AptosFeatureFlag::SPONSORED_AUTOMATIC_ACCOUNT_V1_CREATION
            },
            FeatureFlag::FeePayerAccountOptional => AptosFeatureFlag::FEE_PAYER_ACCOUNT_OPTIONAL,
            FeatureFlag::AggregatorV2DelayedFields => {
                AptosFeatureFlag::AGGREGATOR_V2_DELAYED_FIELDS
            },
            FeatureFlag::ConcurrentAssets => AptosFeatureFlag::CONCURRENT_ASSETS,
            FeatureFlag::LimitMaxIdentifierLength => AptosFeatureFlag::LIMIT_MAX_IDENTIFIER_LENGTH,
            FeatureFlag::OperatorBeneficiaryChange => AptosFeatureFlag::OPERATOR_BENEFICIARY_CHANGE,
            FeatureFlag::ResourceGroupsChargeAsSizeSum => {
                AptosFeatureFlag::RESOURCE_GROUPS_CHARGE_AS_SIZE_SUM
            },
            FeatureFlag::CommissionChangeDelegationPool => {
                AptosFeatureFlag::COMMISSION_CHANGE_DELEGATION_POOL
            },
            FeatureFlag::BN254Structures => AptosFeatureFlag::BN254_STRUCTURES,
            FeatureFlag::WebAuthnSignature => AptosFeatureFlag::WEBAUTHN_SIGNATURE,
            FeatureFlag::ReconfigureWithDKG => AptosFeatureFlag::RECONFIGURE_WITH_DKG,
            FeatureFlag::ZkIdSignature => AptosFeatureFlag::ZK_ID_SIGNATURES,
            FeatureFlag::ZkIdZkLessSignature => AptosFeatureFlag::ZK_ID_ZKLESS_SIGNATURE,
            FeatureFlag::RemoveDetailedError => AptosFeatureFlag::REMOVE_DETAILED_ERROR_FROM_HASH,
            FeatureFlag::JWKConsensus => AptosFeatureFlag::JWK_CONSENSUS,
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
            AptosFeatureFlag::VM_BINARY_FORMAT_V7 => FeatureFlag::VMBinaryFormatV7,
            AptosFeatureFlag::MULTI_ED25519_PK_VALIDATE_V2_NATIVES => {
                FeatureFlag::MultiEd25519PkValidateV2Natives
            },
            AptosFeatureFlag::BLAKE2B_256_NATIVE => FeatureFlag::Blake2b256Native,
            AptosFeatureFlag::RESOURCE_GROUPS => FeatureFlag::ResourceGroups,
            AptosFeatureFlag::MULTISIG_ACCOUNTS => FeatureFlag::MultisigAccounts,
            AptosFeatureFlag::DELEGATION_POOLS => FeatureFlag::DelegationPools,
            AptosFeatureFlag::CRYPTOGRAPHY_ALGEBRA_NATIVES => {
                FeatureFlag::CryptographyAlgebraNatives
            },
            AptosFeatureFlag::BLS12_381_STRUCTURES => FeatureFlag::Bls12381Structures,
            AptosFeatureFlag::ED25519_PUBKEY_VALIDATE_RETURN_FALSE_WRONG_LENGTH => {
                FeatureFlag::Ed25519PubkeyValidateReturnFalseWrongLength
            },
            AptosFeatureFlag::STRUCT_CONSTRUCTORS => FeatureFlag::StructConstructors,
            AptosFeatureFlag::PERIODICAL_REWARD_RATE_DECREASE => {
                FeatureFlag::PeriodicalRewardRateReduction
            },
            AptosFeatureFlag::PARTIAL_GOVERNANCE_VOTING => FeatureFlag::PartialGovernanceVoting,
            AptosFeatureFlag::SIGNATURE_CHECKER_V2 => FeatureFlag::SignatureCheckerV2,
            AptosFeatureFlag::STORAGE_SLOT_METADATA => FeatureFlag::StorageSlotMetadata,
            AptosFeatureFlag::CHARGE_INVARIANT_VIOLATION => FeatureFlag::ChargeInvariantViolation,
            AptosFeatureFlag::DELEGATION_POOL_PARTIAL_GOVERNANCE_VOTING => {
                FeatureFlag::DelegationPoolPartialGovernanceVoting
            },
            AptosFeatureFlag::GAS_PAYER_ENABLED => FeatureFlag::GasPayerEnabled,
            AptosFeatureFlag::APTOS_UNIQUE_IDENTIFIERS => FeatureFlag::AptosUniqueIdentifiers,
            AptosFeatureFlag::BULLETPROOFS_NATIVES => FeatureFlag::BulletproofsNatives,
            AptosFeatureFlag::SIGNER_NATIVE_FORMAT_FIX => FeatureFlag::SignerNativeFormatFix,
            AptosFeatureFlag::MODULE_EVENT => FeatureFlag::ModuleEvent,
            AptosFeatureFlag::EMIT_FEE_STATEMENT => FeatureFlag::EmitFeeStatement,
            AptosFeatureFlag::STORAGE_DELETION_REFUND => FeatureFlag::StorageDeletionRefund,
            AptosFeatureFlag::AGGREGATOR_V2_API => FeatureFlag::AggregatorV2Api,
            AptosFeatureFlag::SIGNATURE_CHECKER_V2_SCRIPT_FIX => {
                FeatureFlag::SignatureCheckerV2ScriptFix
            },
            AptosFeatureFlag::SAFER_RESOURCE_GROUPS => FeatureFlag::SaferResourceGroups,
            AptosFeatureFlag::SAFER_METADATA => FeatureFlag::SaferMetadata,
            AptosFeatureFlag::SINGLE_SENDER_AUTHENTICATOR => FeatureFlag::SingleSenderAuthenticator,
            AptosFeatureFlag::SPONSORED_AUTOMATIC_ACCOUNT_V1_CREATION => {
                FeatureFlag::SponsoredAutomaticAccountCreation
            },
            AptosFeatureFlag::FEE_PAYER_ACCOUNT_OPTIONAL => FeatureFlag::FeePayerAccountOptional,
            AptosFeatureFlag::AGGREGATOR_V2_DELAYED_FIELDS => {
                FeatureFlag::AggregatorV2DelayedFields
            },
            AptosFeatureFlag::CONCURRENT_ASSETS => FeatureFlag::ConcurrentAssets,
            AptosFeatureFlag::LIMIT_MAX_IDENTIFIER_LENGTH => FeatureFlag::LimitMaxIdentifierLength,
            AptosFeatureFlag::OPERATOR_BENEFICIARY_CHANGE => FeatureFlag::OperatorBeneficiaryChange,
            AptosFeatureFlag::RESOURCE_GROUPS_CHARGE_AS_SIZE_SUM => {
                FeatureFlag::ResourceGroupsChargeAsSizeSum
            },
            AptosFeatureFlag::COMMISSION_CHANGE_DELEGATION_POOL => {
                FeatureFlag::CommissionChangeDelegationPool
            },
            AptosFeatureFlag::BN254_STRUCTURES => FeatureFlag::BN254Structures,
            AptosFeatureFlag::WEBAUTHN_SIGNATURE => FeatureFlag::WebAuthnSignature,
            AptosFeatureFlag::RECONFIGURE_WITH_DKG => FeatureFlag::ReconfigureWithDKG,
            AptosFeatureFlag::ZK_ID_SIGNATURES => FeatureFlag::ZkIdSignature,
            AptosFeatureFlag::ZK_ID_ZKLESS_SIGNATURE => FeatureFlag::ZkIdZkLessSignature,
            AptosFeatureFlag::REMOVE_DETAILED_ERROR_FROM_HASH => FeatureFlag::RemoveDetailedError,
            AptosFeatureFlag::JWK_CONSENSUS => FeatureFlag::JWKConsensus,
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

impl From<&AptosFeatures> for Features {
    fn from(features: &AptosFeatures) -> Features {
        let mut enabled = vec![];
        let mut disabled = vec![];
        for feature in FeatureFlag::iter() {
            if features.is_enabled(AptosFeatureFlag::from(feature.clone())) {
                enabled.push(feature);
            } else {
                disabled.push(feature);
            }
        }
        Features { enabled, disabled }
    }
}
