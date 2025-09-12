// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{components::get_signer_arg, utils::*};
use anyhow::Result;
use aptos_crypto::HashValue;
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
    ConcurrentTokenV2,
    LimitMaxIdentifierLength,
    OperatorBeneficiaryChange,
    VMBinaryFormatV7,
    ResourceGroupsSplitInVmChangeSet,
    CommissionChangeDelegationPool,
    Bn254Structures,
    WebAuthnSignature,
    ReconfigureWithDkg,
    KeylessAccounts,
    KeylessButZklessAccounts,
    RemoveDetailedError,
    JwkConsensus,
    ConcurrentFungibleAssets,
    RefundableBytes,
    ObjectCodeDeployment,
    MaxObjectNestingCheck,
    KeylessAccountsWithPasskeys,
    MultisigV2Enhancement,
    DelegationPoolAllowlisting,
    ModuleEventMigration,
    RejectUnstableBytecode,
    TransactionContextExtension,
    CoinToFungibleAssetMigration,
    PrimaryAPTFungibleStoreAtUserAddress,
    ObjectNativeDerivedAddress,
    DispatchableFungibleAsset,
    NewAccountsDefaultToFaAptStore,
    OperationsDefaultToFaAptStore,
    AggregatorV2IsAtLeastApi,
    ConcurrentFungibleBalance,
    DefaultToConcurrentFungibleBalance,
    LimitVMTypeSize,
    AbortIfMultisigPayloadMismatch,
    DisallowUserNative,
    AllowSerializedScriptArgs,
    UseCompatibilityCheckerV2,
    EnableEnumTypes,
    EnableResourceAccessControl,
    RejectUnstableBytecodeForScript,
    FederatedKeyless,
    TransactionSimulationEnhancement,
    CollectionOwner,
    NativeMemoryOperations,
    EnableLoaderV2,
    DisallowInitModuleToPublishModules,
    EnableCallTreeAndInstructionVMCache,
    PermissionedSigner,
    AccountAbstraction,
    VMBinaryFormatV8,
    BulletproofsBatchNatives,
    DerivableAccountAbstraction,
    EnableFunctionValues,
    NewAccountsDefaultToFaStore,
    DefaultAccountResource,
    JwkConsensusPerKeyMode,
    TransactionPayloadV2,
    OrderlessTransactions,
    EnableLazyLoading,
    CalculateTransactionFeeForDistribution,
    DistributeTransactionFee,
    MonotonicallyIncreasingCounter,
    EnableCaptureOption,
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
    next_execution_hash: Option<HashValue>,
    is_multi_step: bool,
) -> Result<Vec<(String, String)>> {
    let signer_arg = get_signer_arg(is_testnet, &next_execution_hash);
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
        next_execution_hash,
        is_multi_step,
        &["std::features"],
        |writer| {
            emit!(writer, "let enabled_blob: vector<u64> = ");
            generate_features_blob(writer, &enabled);
            emitln!(writer, ";\n");

            emit!(writer, "let disabled_blob: vector<u64> = ");
            generate_features_blob(writer, &disabled);
            emitln!(writer, ";\n");

            emitln!(
                writer,
                "features::change_feature_flags_for_next_epoch({}, enabled_blob, disabled_blob);",
                signer_arg
            );
            emitln!(writer, "aptos_governance::reconfigure({});", signer_arg);
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
                AptosFeatureFlag::_DEPRECATED_COLLECT_AND_DISTRIBUTE_GAS_FEES
            },
            FeatureFlag::TreatFriendAsPrivate => AptosFeatureFlag::TREAT_FRIEND_AS_PRIVATE,
            FeatureFlag::Sha512AndRipeMd160Natives => {
                AptosFeatureFlag::SHA_512_AND_RIPEMD_160_NATIVES
            },
            FeatureFlag::AptosStdChainIdNatives => AptosFeatureFlag::APTOS_STD_CHAIN_ID_NATIVES,
            FeatureFlag::VMBinaryFormatV6 => AptosFeatureFlag::VM_BINARY_FORMAT_V6,
            FeatureFlag::VMBinaryFormatV7 => AptosFeatureFlag::VM_BINARY_FORMAT_V7,
            FeatureFlag::VMBinaryFormatV8 => AptosFeatureFlag::VM_BINARY_FORMAT_V8,
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
            FeatureFlag::ConcurrentTokenV2 => AptosFeatureFlag::CONCURRENT_TOKEN_V2,
            FeatureFlag::LimitMaxIdentifierLength => AptosFeatureFlag::LIMIT_MAX_IDENTIFIER_LENGTH,
            FeatureFlag::OperatorBeneficiaryChange => AptosFeatureFlag::OPERATOR_BENEFICIARY_CHANGE,
            FeatureFlag::ResourceGroupsSplitInVmChangeSet => {
                AptosFeatureFlag::RESOURCE_GROUPS_SPLIT_IN_VM_CHANGE_SET
            },
            FeatureFlag::CommissionChangeDelegationPool => {
                AptosFeatureFlag::COMMISSION_CHANGE_DELEGATION_POOL
            },
            FeatureFlag::Bn254Structures => AptosFeatureFlag::BN254_STRUCTURES,
            FeatureFlag::WebAuthnSignature => AptosFeatureFlag::WEBAUTHN_SIGNATURE,
            FeatureFlag::ReconfigureWithDkg => AptosFeatureFlag::_DEPRECATED_RECONFIGURE_WITH_DKG,
            FeatureFlag::KeylessAccounts => AptosFeatureFlag::KEYLESS_ACCOUNTS,
            FeatureFlag::KeylessButZklessAccounts => AptosFeatureFlag::KEYLESS_BUT_ZKLESS_ACCOUNTS,
            FeatureFlag::RemoveDetailedError => {
                AptosFeatureFlag::_DEPRECATED_REMOVE_DETAILED_ERROR_FROM_HASH
            },
            FeatureFlag::JwkConsensus => AptosFeatureFlag::JWK_CONSENSUS,
            FeatureFlag::ConcurrentFungibleAssets => AptosFeatureFlag::CONCURRENT_FUNGIBLE_ASSETS,
            FeatureFlag::RefundableBytes => AptosFeatureFlag::REFUNDABLE_BYTES,
            FeatureFlag::ObjectCodeDeployment => AptosFeatureFlag::OBJECT_CODE_DEPLOYMENT,
            FeatureFlag::MaxObjectNestingCheck => AptosFeatureFlag::MAX_OBJECT_NESTING_CHECK,
            FeatureFlag::KeylessAccountsWithPasskeys => {
                AptosFeatureFlag::KEYLESS_ACCOUNTS_WITH_PASSKEYS
            },
            FeatureFlag::MultisigV2Enhancement => AptosFeatureFlag::MULTISIG_V2_ENHANCEMENT,
            FeatureFlag::DelegationPoolAllowlisting => {
                AptosFeatureFlag::DELEGATION_POOL_ALLOWLISTING
            },
            FeatureFlag::ModuleEventMigration => AptosFeatureFlag::MODULE_EVENT_MIGRATION,
            FeatureFlag::RejectUnstableBytecode => AptosFeatureFlag::_REJECT_UNSTABLE_BYTECODE,
            FeatureFlag::TransactionContextExtension => {
                AptosFeatureFlag::TRANSACTION_CONTEXT_EXTENSION
            },
            FeatureFlag::CoinToFungibleAssetMigration => {
                AptosFeatureFlag::COIN_TO_FUNGIBLE_ASSET_MIGRATION
            },
            FeatureFlag::PrimaryAPTFungibleStoreAtUserAddress => {
                AptosFeatureFlag::PRIMARY_APT_FUNGIBLE_STORE_AT_USER_ADDRESS
            },
            FeatureFlag::ObjectNativeDerivedAddress => {
                AptosFeatureFlag::OBJECT_NATIVE_DERIVED_ADDRESS
            },
            FeatureFlag::DispatchableFungibleAsset => AptosFeatureFlag::DISPATCHABLE_FUNGIBLE_ASSET,
            FeatureFlag::NewAccountsDefaultToFaAptStore => {
                AptosFeatureFlag::NEW_ACCOUNTS_DEFAULT_TO_FA_APT_STORE
            },
            FeatureFlag::OperationsDefaultToFaAptStore => {
                AptosFeatureFlag::OPERATIONS_DEFAULT_TO_FA_APT_STORE
            },
            FeatureFlag::AggregatorV2IsAtLeastApi => {
                AptosFeatureFlag::AGGREGATOR_V2_IS_AT_LEAST_API
            },
            FeatureFlag::ConcurrentFungibleBalance => AptosFeatureFlag::CONCURRENT_FUNGIBLE_BALANCE,
            FeatureFlag::DefaultToConcurrentFungibleBalance => {
                AptosFeatureFlag::DEFAULT_TO_CONCURRENT_FUNGIBLE_BALANCE
            },
            FeatureFlag::LimitVMTypeSize => AptosFeatureFlag::_LIMIT_VM_TYPE_SIZE,
            FeatureFlag::AbortIfMultisigPayloadMismatch => {
                AptosFeatureFlag::ABORT_IF_MULTISIG_PAYLOAD_MISMATCH
            },
            FeatureFlag::DisallowUserNative => AptosFeatureFlag::_DISALLOW_USER_NATIVES,
            FeatureFlag::AllowSerializedScriptArgs => {
                AptosFeatureFlag::ALLOW_SERIALIZED_SCRIPT_ARGS
            },
            FeatureFlag::UseCompatibilityCheckerV2 => {
                AptosFeatureFlag::_USE_COMPATIBILITY_CHECKER_V2
            },
            FeatureFlag::EnableEnumTypes => AptosFeatureFlag::ENABLE_ENUM_TYPES,
            FeatureFlag::EnableResourceAccessControl => {
                AptosFeatureFlag::ENABLE_RESOURCE_ACCESS_CONTROL
            },
            FeatureFlag::RejectUnstableBytecodeForScript => {
                AptosFeatureFlag::_REJECT_UNSTABLE_BYTECODE_FOR_SCRIPT
            },
            FeatureFlag::FederatedKeyless => AptosFeatureFlag::FEDERATED_KEYLESS,
            FeatureFlag::TransactionSimulationEnhancement => {
                AptosFeatureFlag::TRANSACTION_SIMULATION_ENHANCEMENT
            },
            FeatureFlag::CollectionOwner => AptosFeatureFlag::COLLECTION_OWNER,
            FeatureFlag::NativeMemoryOperations => AptosFeatureFlag::NATIVE_MEMORY_OPERATIONS,
            FeatureFlag::EnableLoaderV2 => AptosFeatureFlag::_ENABLE_LOADER_V2,
            FeatureFlag::DisallowInitModuleToPublishModules => {
                AptosFeatureFlag::_DISALLOW_INIT_MODULE_TO_PUBLISH_MODULES
            },
            FeatureFlag::EnableCallTreeAndInstructionVMCache => {
                AptosFeatureFlag::ENABLE_CALL_TREE_AND_INSTRUCTION_VM_CACHE
            },
            FeatureFlag::PermissionedSigner => AptosFeatureFlag::PERMISSIONED_SIGNER,
            FeatureFlag::AccountAbstraction => AptosFeatureFlag::ACCOUNT_ABSTRACTION,
            FeatureFlag::BulletproofsBatchNatives => AptosFeatureFlag::BULLETPROOFS_BATCH_NATIVES,
            FeatureFlag::DerivableAccountAbstraction => {
                AptosFeatureFlag::DERIVABLE_ACCOUNT_ABSTRACTION
            },
            FeatureFlag::EnableFunctionValues => AptosFeatureFlag::ENABLE_FUNCTION_VALUES,
            FeatureFlag::NewAccountsDefaultToFaStore => {
                AptosFeatureFlag::NEW_ACCOUNTS_DEFAULT_TO_FA_STORE
            },
            FeatureFlag::DefaultAccountResource => AptosFeatureFlag::DEFAULT_ACCOUNT_RESOURCE,
            FeatureFlag::JwkConsensusPerKeyMode => AptosFeatureFlag::JWK_CONSENSUS_PER_KEY_MODE,
            FeatureFlag::TransactionPayloadV2 => AptosFeatureFlag::TRANSACTION_PAYLOAD_V2,
            FeatureFlag::OrderlessTransactions => AptosFeatureFlag::ORDERLESS_TRANSACTIONS,
            FeatureFlag::EnableLazyLoading => AptosFeatureFlag::ENABLE_LAZY_LOADING,
            FeatureFlag::CalculateTransactionFeeForDistribution => {
                AptosFeatureFlag::CALCULATE_TRANSACTION_FEE_FOR_DISTRIBUTION
            },
            FeatureFlag::DistributeTransactionFee => AptosFeatureFlag::DISTRIBUTE_TRANSACTION_FEE,
            FeatureFlag::MonotonicallyIncreasingCounter => {
                AptosFeatureFlag::MONOTONICALLY_INCREASING_COUNTER
            },
            FeatureFlag::EnableCaptureOption => AptosFeatureFlag::ENABLE_CAPTURE_OPTION,
        }
    }
}

// We don't need this implementation. Just to make sure we have an exhaustive 1-1 mapping between the two structs.
impl From<AptosFeatureFlag> for FeatureFlag {
    fn from(f: AptosFeatureFlag) -> Self {
        match f {
            AptosFeatureFlag::CODE_DEPENDENCY_CHECK => FeatureFlag::CodeDependencyCheck,
            AptosFeatureFlag::_DEPRECATED_COLLECT_AND_DISTRIBUTE_GAS_FEES => {
                FeatureFlag::CollectAndDistributeGasFees
            },
            AptosFeatureFlag::TREAT_FRIEND_AS_PRIVATE => FeatureFlag::TreatFriendAsPrivate,
            AptosFeatureFlag::SHA_512_AND_RIPEMD_160_NATIVES => {
                FeatureFlag::Sha512AndRipeMd160Natives
            },
            AptosFeatureFlag::APTOS_STD_CHAIN_ID_NATIVES => FeatureFlag::AptosStdChainIdNatives,
            AptosFeatureFlag::VM_BINARY_FORMAT_V6 => FeatureFlag::VMBinaryFormatV6,
            AptosFeatureFlag::VM_BINARY_FORMAT_V7 => FeatureFlag::VMBinaryFormatV7,
            AptosFeatureFlag::VM_BINARY_FORMAT_V8 => FeatureFlag::VMBinaryFormatV8,
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
            AptosFeatureFlag::CONCURRENT_TOKEN_V2 => FeatureFlag::ConcurrentTokenV2,
            AptosFeatureFlag::LIMIT_MAX_IDENTIFIER_LENGTH => FeatureFlag::LimitMaxIdentifierLength,
            AptosFeatureFlag::OPERATOR_BENEFICIARY_CHANGE => FeatureFlag::OperatorBeneficiaryChange,
            AptosFeatureFlag::RESOURCE_GROUPS_SPLIT_IN_VM_CHANGE_SET => {
                FeatureFlag::ResourceGroupsSplitInVmChangeSet
            },
            AptosFeatureFlag::COMMISSION_CHANGE_DELEGATION_POOL => {
                FeatureFlag::CommissionChangeDelegationPool
            },
            AptosFeatureFlag::BN254_STRUCTURES => FeatureFlag::Bn254Structures,
            AptosFeatureFlag::WEBAUTHN_SIGNATURE => FeatureFlag::WebAuthnSignature,
            AptosFeatureFlag::_DEPRECATED_RECONFIGURE_WITH_DKG => FeatureFlag::ReconfigureWithDkg,
            AptosFeatureFlag::KEYLESS_ACCOUNTS => FeatureFlag::KeylessAccounts,
            AptosFeatureFlag::KEYLESS_BUT_ZKLESS_ACCOUNTS => FeatureFlag::KeylessButZklessAccounts,
            AptosFeatureFlag::_DEPRECATED_REMOVE_DETAILED_ERROR_FROM_HASH => {
                FeatureFlag::RemoveDetailedError
            },
            AptosFeatureFlag::JWK_CONSENSUS => FeatureFlag::JwkConsensus,
            AptosFeatureFlag::CONCURRENT_FUNGIBLE_ASSETS => FeatureFlag::ConcurrentFungibleAssets,
            AptosFeatureFlag::REFUNDABLE_BYTES => FeatureFlag::RefundableBytes,
            AptosFeatureFlag::OBJECT_CODE_DEPLOYMENT => FeatureFlag::ObjectCodeDeployment,
            AptosFeatureFlag::MAX_OBJECT_NESTING_CHECK => FeatureFlag::MaxObjectNestingCheck,
            AptosFeatureFlag::KEYLESS_ACCOUNTS_WITH_PASSKEYS => {
                FeatureFlag::KeylessAccountsWithPasskeys
            },
            AptosFeatureFlag::MULTISIG_V2_ENHANCEMENT => FeatureFlag::MultisigV2Enhancement,
            AptosFeatureFlag::DELEGATION_POOL_ALLOWLISTING => {
                FeatureFlag::DelegationPoolAllowlisting
            },
            AptosFeatureFlag::MODULE_EVENT_MIGRATION => FeatureFlag::ModuleEventMigration,
            AptosFeatureFlag::_REJECT_UNSTABLE_BYTECODE => FeatureFlag::RejectUnstableBytecode,
            AptosFeatureFlag::TRANSACTION_CONTEXT_EXTENSION => {
                FeatureFlag::TransactionContextExtension
            },
            AptosFeatureFlag::COIN_TO_FUNGIBLE_ASSET_MIGRATION => {
                FeatureFlag::CoinToFungibleAssetMigration
            },
            AptosFeatureFlag::PRIMARY_APT_FUNGIBLE_STORE_AT_USER_ADDRESS => {
                FeatureFlag::PrimaryAPTFungibleStoreAtUserAddress
            },
            AptosFeatureFlag::OBJECT_NATIVE_DERIVED_ADDRESS => {
                FeatureFlag::ObjectNativeDerivedAddress
            },
            AptosFeatureFlag::DISPATCHABLE_FUNGIBLE_ASSET => FeatureFlag::DispatchableFungibleAsset,
            AptosFeatureFlag::NEW_ACCOUNTS_DEFAULT_TO_FA_APT_STORE => {
                FeatureFlag::NewAccountsDefaultToFaAptStore
            },
            AptosFeatureFlag::OPERATIONS_DEFAULT_TO_FA_APT_STORE => {
                FeatureFlag::OperationsDefaultToFaAptStore
            },
            AptosFeatureFlag::AGGREGATOR_V2_IS_AT_LEAST_API => {
                FeatureFlag::AggregatorV2IsAtLeastApi
            },
            AptosFeatureFlag::CONCURRENT_FUNGIBLE_BALANCE => FeatureFlag::ConcurrentFungibleBalance,
            AptosFeatureFlag::DEFAULT_TO_CONCURRENT_FUNGIBLE_BALANCE => {
                FeatureFlag::DefaultToConcurrentFungibleBalance
            },
            AptosFeatureFlag::_LIMIT_VM_TYPE_SIZE => FeatureFlag::LimitVMTypeSize,
            AptosFeatureFlag::ABORT_IF_MULTISIG_PAYLOAD_MISMATCH => {
                FeatureFlag::AbortIfMultisigPayloadMismatch
            },
            AptosFeatureFlag::_DISALLOW_USER_NATIVES => FeatureFlag::DisallowUserNative,
            AptosFeatureFlag::ALLOW_SERIALIZED_SCRIPT_ARGS => {
                FeatureFlag::AllowSerializedScriptArgs
            },
            AptosFeatureFlag::_USE_COMPATIBILITY_CHECKER_V2 => {
                FeatureFlag::UseCompatibilityCheckerV2
            },
            AptosFeatureFlag::ENABLE_ENUM_TYPES => FeatureFlag::EnableEnumTypes,
            AptosFeatureFlag::ENABLE_RESOURCE_ACCESS_CONTROL => {
                FeatureFlag::EnableResourceAccessControl
            },
            AptosFeatureFlag::_REJECT_UNSTABLE_BYTECODE_FOR_SCRIPT => {
                FeatureFlag::RejectUnstableBytecodeForScript
            },
            AptosFeatureFlag::FEDERATED_KEYLESS => FeatureFlag::FederatedKeyless,
            AptosFeatureFlag::TRANSACTION_SIMULATION_ENHANCEMENT => {
                FeatureFlag::TransactionSimulationEnhancement
            },
            AptosFeatureFlag::COLLECTION_OWNER => FeatureFlag::CollectionOwner,
            AptosFeatureFlag::NATIVE_MEMORY_OPERATIONS => FeatureFlag::NativeMemoryOperations,
            AptosFeatureFlag::_ENABLE_LOADER_V2 => FeatureFlag::EnableLoaderV2,
            AptosFeatureFlag::_DISALLOW_INIT_MODULE_TO_PUBLISH_MODULES => {
                FeatureFlag::DisallowInitModuleToPublishModules
            },
            AptosFeatureFlag::ENABLE_CALL_TREE_AND_INSTRUCTION_VM_CACHE => {
                FeatureFlag::EnableCallTreeAndInstructionVMCache
            },
            AptosFeatureFlag::PERMISSIONED_SIGNER => FeatureFlag::PermissionedSigner,
            AptosFeatureFlag::ACCOUNT_ABSTRACTION => FeatureFlag::AccountAbstraction,
            AptosFeatureFlag::BULLETPROOFS_BATCH_NATIVES => FeatureFlag::BulletproofsBatchNatives,
            AptosFeatureFlag::DERIVABLE_ACCOUNT_ABSTRACTION => {
                FeatureFlag::DerivableAccountAbstraction
            },
            AptosFeatureFlag::ENABLE_FUNCTION_VALUES => FeatureFlag::EnableFunctionValues,
            AptosFeatureFlag::NEW_ACCOUNTS_DEFAULT_TO_FA_STORE => {
                FeatureFlag::NewAccountsDefaultToFaStore
            },
            AptosFeatureFlag::DEFAULT_ACCOUNT_RESOURCE => FeatureFlag::DefaultAccountResource,
            AptosFeatureFlag::JWK_CONSENSUS_PER_KEY_MODE => FeatureFlag::JwkConsensusPerKeyMode,
            AptosFeatureFlag::TRANSACTION_PAYLOAD_V2 => FeatureFlag::TransactionPayloadV2,
            AptosFeatureFlag::ORDERLESS_TRANSACTIONS => FeatureFlag::OrderlessTransactions,
            AptosFeatureFlag::ENABLE_LAZY_LOADING => FeatureFlag::EnableLazyLoading,
            AptosFeatureFlag::CALCULATE_TRANSACTION_FEE_FOR_DISTRIBUTION => {
                FeatureFlag::CalculateTransactionFeeForDistribution
            },
            AptosFeatureFlag::DISTRIBUTE_TRANSACTION_FEE => FeatureFlag::DistributeTransactionFee,
            AptosFeatureFlag::MONOTONICALLY_INCREASING_COUNTER => {
                FeatureFlag::MonotonicallyIncreasingCounter
            },
            AptosFeatureFlag::ENABLE_CAPTURE_OPTION => FeatureFlag::EnableCaptureOption,
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
