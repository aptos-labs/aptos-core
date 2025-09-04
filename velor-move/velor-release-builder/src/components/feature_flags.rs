// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{components::get_signer_arg, utils::*};
use anyhow::Result;
use velor_crypto::HashValue;
use velor_types::on_chain_config::{FeatureFlag as VelorFeatureFlag, Features as VelorFeatures};
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
    VelorStdChainIdNatives,
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
    VelorUniqueIdentifiers,
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
        .map(|f| VelorFeatureFlag::from(f.clone()) as u64)
        .collect::<Vec<_>>();
    let disabled = features
        .disabled
        .iter()
        .map(|f| VelorFeatureFlag::from(f.clone()) as u64)
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
            emitln!(writer, "velor_governance::reconfigure({});", signer_arg);
        },
    );

    result.push(("features".to_string(), proposal));
    Ok(result)
}

impl From<FeatureFlag> for VelorFeatureFlag {
    fn from(f: FeatureFlag) -> Self {
        match f {
            FeatureFlag::CodeDependencyCheck => VelorFeatureFlag::CODE_DEPENDENCY_CHECK,
            FeatureFlag::CollectAndDistributeGasFees => {
                VelorFeatureFlag::_DEPRECATED_COLLECT_AND_DISTRIBUTE_GAS_FEES
            },
            FeatureFlag::TreatFriendAsPrivate => VelorFeatureFlag::TREAT_FRIEND_AS_PRIVATE,
            FeatureFlag::Sha512AndRipeMd160Natives => {
                VelorFeatureFlag::SHA_512_AND_RIPEMD_160_NATIVES
            },
            FeatureFlag::VelorStdChainIdNatives => VelorFeatureFlag::VELOR_STD_CHAIN_ID_NATIVES,
            FeatureFlag::VMBinaryFormatV6 => VelorFeatureFlag::VM_BINARY_FORMAT_V6,
            FeatureFlag::VMBinaryFormatV7 => VelorFeatureFlag::VM_BINARY_FORMAT_V7,
            FeatureFlag::VMBinaryFormatV8 => VelorFeatureFlag::VM_BINARY_FORMAT_V8,
            FeatureFlag::MultiEd25519PkValidateV2Natives => {
                VelorFeatureFlag::MULTI_ED25519_PK_VALIDATE_V2_NATIVES
            },
            FeatureFlag::Blake2b256Native => VelorFeatureFlag::BLAKE2B_256_NATIVE,
            FeatureFlag::ResourceGroups => VelorFeatureFlag::RESOURCE_GROUPS,
            FeatureFlag::MultisigAccounts => VelorFeatureFlag::MULTISIG_ACCOUNTS,
            FeatureFlag::DelegationPools => VelorFeatureFlag::DELEGATION_POOLS,
            FeatureFlag::CryptographyAlgebraNatives => {
                VelorFeatureFlag::CRYPTOGRAPHY_ALGEBRA_NATIVES
            },
            FeatureFlag::Bls12381Structures => VelorFeatureFlag::BLS12_381_STRUCTURES,
            FeatureFlag::Ed25519PubkeyValidateReturnFalseWrongLength => {
                VelorFeatureFlag::ED25519_PUBKEY_VALIDATE_RETURN_FALSE_WRONG_LENGTH
            },
            FeatureFlag::StructConstructors => VelorFeatureFlag::STRUCT_CONSTRUCTORS,
            FeatureFlag::PeriodicalRewardRateReduction => {
                VelorFeatureFlag::PERIODICAL_REWARD_RATE_DECREASE
            },
            FeatureFlag::PartialGovernanceVoting => VelorFeatureFlag::PARTIAL_GOVERNANCE_VOTING,
            FeatureFlag::SignatureCheckerV2 => VelorFeatureFlag::SIGNATURE_CHECKER_V2,
            FeatureFlag::StorageSlotMetadata => VelorFeatureFlag::STORAGE_SLOT_METADATA,
            FeatureFlag::ChargeInvariantViolation => VelorFeatureFlag::CHARGE_INVARIANT_VIOLATION,
            FeatureFlag::DelegationPoolPartialGovernanceVoting => {
                VelorFeatureFlag::DELEGATION_POOL_PARTIAL_GOVERNANCE_VOTING
            },
            FeatureFlag::GasPayerEnabled => VelorFeatureFlag::GAS_PAYER_ENABLED,
            FeatureFlag::VelorUniqueIdentifiers => VelorFeatureFlag::VELOR_UNIQUE_IDENTIFIERS,
            FeatureFlag::BulletproofsNatives => VelorFeatureFlag::BULLETPROOFS_NATIVES,
            FeatureFlag::SignerNativeFormatFix => VelorFeatureFlag::SIGNER_NATIVE_FORMAT_FIX,
            FeatureFlag::ModuleEvent => VelorFeatureFlag::MODULE_EVENT,
            FeatureFlag::EmitFeeStatement => VelorFeatureFlag::EMIT_FEE_STATEMENT,
            FeatureFlag::StorageDeletionRefund => VelorFeatureFlag::STORAGE_DELETION_REFUND,
            FeatureFlag::AggregatorV2Api => VelorFeatureFlag::AGGREGATOR_V2_API,
            FeatureFlag::SignatureCheckerV2ScriptFix => {
                VelorFeatureFlag::SIGNATURE_CHECKER_V2_SCRIPT_FIX
            },
            FeatureFlag::SaferResourceGroups => VelorFeatureFlag::SAFER_RESOURCE_GROUPS,
            FeatureFlag::SaferMetadata => VelorFeatureFlag::SAFER_METADATA,
            FeatureFlag::SingleSenderAuthenticator => VelorFeatureFlag::SINGLE_SENDER_AUTHENTICATOR,
            FeatureFlag::SponsoredAutomaticAccountCreation => {
                VelorFeatureFlag::SPONSORED_AUTOMATIC_ACCOUNT_V1_CREATION
            },
            FeatureFlag::FeePayerAccountOptional => VelorFeatureFlag::FEE_PAYER_ACCOUNT_OPTIONAL,
            FeatureFlag::AggregatorV2DelayedFields => {
                VelorFeatureFlag::AGGREGATOR_V2_DELAYED_FIELDS
            },
            FeatureFlag::ConcurrentTokenV2 => VelorFeatureFlag::CONCURRENT_TOKEN_V2,
            FeatureFlag::LimitMaxIdentifierLength => VelorFeatureFlag::LIMIT_MAX_IDENTIFIER_LENGTH,
            FeatureFlag::OperatorBeneficiaryChange => VelorFeatureFlag::OPERATOR_BENEFICIARY_CHANGE,
            FeatureFlag::ResourceGroupsSplitInVmChangeSet => {
                VelorFeatureFlag::RESOURCE_GROUPS_SPLIT_IN_VM_CHANGE_SET
            },
            FeatureFlag::CommissionChangeDelegationPool => {
                VelorFeatureFlag::COMMISSION_CHANGE_DELEGATION_POOL
            },
            FeatureFlag::Bn254Structures => VelorFeatureFlag::BN254_STRUCTURES,
            FeatureFlag::WebAuthnSignature => VelorFeatureFlag::WEBAUTHN_SIGNATURE,
            FeatureFlag::ReconfigureWithDkg => VelorFeatureFlag::_DEPRECATED_RECONFIGURE_WITH_DKG,
            FeatureFlag::KeylessAccounts => VelorFeatureFlag::KEYLESS_ACCOUNTS,
            FeatureFlag::KeylessButZklessAccounts => VelorFeatureFlag::KEYLESS_BUT_ZKLESS_ACCOUNTS,
            FeatureFlag::RemoveDetailedError => {
                VelorFeatureFlag::_DEPRECATED_REMOVE_DETAILED_ERROR_FROM_HASH
            },
            FeatureFlag::JwkConsensus => VelorFeatureFlag::JWK_CONSENSUS,
            FeatureFlag::ConcurrentFungibleAssets => VelorFeatureFlag::CONCURRENT_FUNGIBLE_ASSETS,
            FeatureFlag::RefundableBytes => VelorFeatureFlag::REFUNDABLE_BYTES,
            FeatureFlag::ObjectCodeDeployment => VelorFeatureFlag::OBJECT_CODE_DEPLOYMENT,
            FeatureFlag::MaxObjectNestingCheck => VelorFeatureFlag::MAX_OBJECT_NESTING_CHECK,
            FeatureFlag::KeylessAccountsWithPasskeys => {
                VelorFeatureFlag::KEYLESS_ACCOUNTS_WITH_PASSKEYS
            },
            FeatureFlag::MultisigV2Enhancement => VelorFeatureFlag::MULTISIG_V2_ENHANCEMENT,
            FeatureFlag::DelegationPoolAllowlisting => {
                VelorFeatureFlag::DELEGATION_POOL_ALLOWLISTING
            },
            FeatureFlag::ModuleEventMigration => VelorFeatureFlag::MODULE_EVENT_MIGRATION,
            FeatureFlag::RejectUnstableBytecode => VelorFeatureFlag::_REJECT_UNSTABLE_BYTECODE,
            FeatureFlag::TransactionContextExtension => {
                VelorFeatureFlag::TRANSACTION_CONTEXT_EXTENSION
            },
            FeatureFlag::CoinToFungibleAssetMigration => {
                VelorFeatureFlag::COIN_TO_FUNGIBLE_ASSET_MIGRATION
            },
            FeatureFlag::PrimaryAPTFungibleStoreAtUserAddress => {
                VelorFeatureFlag::PRIMARY_APT_FUNGIBLE_STORE_AT_USER_ADDRESS
            },
            FeatureFlag::ObjectNativeDerivedAddress => {
                VelorFeatureFlag::OBJECT_NATIVE_DERIVED_ADDRESS
            },
            FeatureFlag::DispatchableFungibleAsset => VelorFeatureFlag::DISPATCHABLE_FUNGIBLE_ASSET,
            FeatureFlag::NewAccountsDefaultToFaAptStore => {
                VelorFeatureFlag::NEW_ACCOUNTS_DEFAULT_TO_FA_APT_STORE
            },
            FeatureFlag::OperationsDefaultToFaAptStore => {
                VelorFeatureFlag::OPERATIONS_DEFAULT_TO_FA_APT_STORE
            },
            FeatureFlag::AggregatorV2IsAtLeastApi => {
                VelorFeatureFlag::AGGREGATOR_V2_IS_AT_LEAST_API
            },
            FeatureFlag::ConcurrentFungibleBalance => VelorFeatureFlag::CONCURRENT_FUNGIBLE_BALANCE,
            FeatureFlag::DefaultToConcurrentFungibleBalance => {
                VelorFeatureFlag::DEFAULT_TO_CONCURRENT_FUNGIBLE_BALANCE
            },
            FeatureFlag::LimitVMTypeSize => VelorFeatureFlag::_LIMIT_VM_TYPE_SIZE,
            FeatureFlag::AbortIfMultisigPayloadMismatch => {
                VelorFeatureFlag::ABORT_IF_MULTISIG_PAYLOAD_MISMATCH
            },
            FeatureFlag::DisallowUserNative => VelorFeatureFlag::_DISALLOW_USER_NATIVES,
            FeatureFlag::AllowSerializedScriptArgs => {
                VelorFeatureFlag::ALLOW_SERIALIZED_SCRIPT_ARGS
            },
            FeatureFlag::UseCompatibilityCheckerV2 => {
                VelorFeatureFlag::_USE_COMPATIBILITY_CHECKER_V2
            },
            FeatureFlag::EnableEnumTypes => VelorFeatureFlag::ENABLE_ENUM_TYPES,
            FeatureFlag::EnableResourceAccessControl => {
                VelorFeatureFlag::ENABLE_RESOURCE_ACCESS_CONTROL
            },
            FeatureFlag::RejectUnstableBytecodeForScript => {
                VelorFeatureFlag::_REJECT_UNSTABLE_BYTECODE_FOR_SCRIPT
            },
            FeatureFlag::FederatedKeyless => VelorFeatureFlag::FEDERATED_KEYLESS,
            FeatureFlag::TransactionSimulationEnhancement => {
                VelorFeatureFlag::TRANSACTION_SIMULATION_ENHANCEMENT
            },
            FeatureFlag::CollectionOwner => VelorFeatureFlag::COLLECTION_OWNER,
            FeatureFlag::NativeMemoryOperations => VelorFeatureFlag::NATIVE_MEMORY_OPERATIONS,
            FeatureFlag::EnableLoaderV2 => VelorFeatureFlag::_ENABLE_LOADER_V2,
            FeatureFlag::DisallowInitModuleToPublishModules => {
                VelorFeatureFlag::_DISALLOW_INIT_MODULE_TO_PUBLISH_MODULES
            },
            FeatureFlag::EnableCallTreeAndInstructionVMCache => {
                VelorFeatureFlag::ENABLE_CALL_TREE_AND_INSTRUCTION_VM_CACHE
            },
            FeatureFlag::PermissionedSigner => VelorFeatureFlag::PERMISSIONED_SIGNER,
            FeatureFlag::AccountAbstraction => VelorFeatureFlag::ACCOUNT_ABSTRACTION,
            FeatureFlag::BulletproofsBatchNatives => VelorFeatureFlag::BULLETPROOFS_BATCH_NATIVES,
            FeatureFlag::DerivableAccountAbstraction => {
                VelorFeatureFlag::DERIVABLE_ACCOUNT_ABSTRACTION
            },
            FeatureFlag::EnableFunctionValues => VelorFeatureFlag::ENABLE_FUNCTION_VALUES,
            FeatureFlag::NewAccountsDefaultToFaStore => {
                VelorFeatureFlag::NEW_ACCOUNTS_DEFAULT_TO_FA_STORE
            },
            FeatureFlag::DefaultAccountResource => VelorFeatureFlag::DEFAULT_ACCOUNT_RESOURCE,
            FeatureFlag::JwkConsensusPerKeyMode => VelorFeatureFlag::JWK_CONSENSUS_PER_KEY_MODE,
            FeatureFlag::TransactionPayloadV2 => VelorFeatureFlag::TRANSACTION_PAYLOAD_V2,
            FeatureFlag::OrderlessTransactions => VelorFeatureFlag::ORDERLESS_TRANSACTIONS,
            FeatureFlag::EnableLazyLoading => VelorFeatureFlag::ENABLE_LAZY_LOADING,
            FeatureFlag::CalculateTransactionFeeForDistribution => {
                VelorFeatureFlag::CALCULATE_TRANSACTION_FEE_FOR_DISTRIBUTION
            },
            FeatureFlag::DistributeTransactionFee => VelorFeatureFlag::DISTRIBUTE_TRANSACTION_FEE,
            FeatureFlag::MonotonicallyIncreasingCounter => {
                VelorFeatureFlag::MONOTONICALLY_INCREASING_COUNTER
            },
        }
    }
}

// We don't need this implementation. Just to make sure we have an exhaustive 1-1 mapping between the two structs.
impl From<VelorFeatureFlag> for FeatureFlag {
    fn from(f: VelorFeatureFlag) -> Self {
        match f {
            VelorFeatureFlag::CODE_DEPENDENCY_CHECK => FeatureFlag::CodeDependencyCheck,
            VelorFeatureFlag::_DEPRECATED_COLLECT_AND_DISTRIBUTE_GAS_FEES => {
                FeatureFlag::CollectAndDistributeGasFees
            },
            VelorFeatureFlag::TREAT_FRIEND_AS_PRIVATE => FeatureFlag::TreatFriendAsPrivate,
            VelorFeatureFlag::SHA_512_AND_RIPEMD_160_NATIVES => {
                FeatureFlag::Sha512AndRipeMd160Natives
            },
            VelorFeatureFlag::VELOR_STD_CHAIN_ID_NATIVES => FeatureFlag::VelorStdChainIdNatives,
            VelorFeatureFlag::VM_BINARY_FORMAT_V6 => FeatureFlag::VMBinaryFormatV6,
            VelorFeatureFlag::VM_BINARY_FORMAT_V7 => FeatureFlag::VMBinaryFormatV7,
            VelorFeatureFlag::VM_BINARY_FORMAT_V8 => FeatureFlag::VMBinaryFormatV8,
            VelorFeatureFlag::MULTI_ED25519_PK_VALIDATE_V2_NATIVES => {
                FeatureFlag::MultiEd25519PkValidateV2Natives
            },
            VelorFeatureFlag::BLAKE2B_256_NATIVE => FeatureFlag::Blake2b256Native,
            VelorFeatureFlag::RESOURCE_GROUPS => FeatureFlag::ResourceGroups,
            VelorFeatureFlag::MULTISIG_ACCOUNTS => FeatureFlag::MultisigAccounts,
            VelorFeatureFlag::DELEGATION_POOLS => FeatureFlag::DelegationPools,
            VelorFeatureFlag::CRYPTOGRAPHY_ALGEBRA_NATIVES => {
                FeatureFlag::CryptographyAlgebraNatives
            },
            VelorFeatureFlag::BLS12_381_STRUCTURES => FeatureFlag::Bls12381Structures,
            VelorFeatureFlag::ED25519_PUBKEY_VALIDATE_RETURN_FALSE_WRONG_LENGTH => {
                FeatureFlag::Ed25519PubkeyValidateReturnFalseWrongLength
            },
            VelorFeatureFlag::STRUCT_CONSTRUCTORS => FeatureFlag::StructConstructors,
            VelorFeatureFlag::PERIODICAL_REWARD_RATE_DECREASE => {
                FeatureFlag::PeriodicalRewardRateReduction
            },
            VelorFeatureFlag::PARTIAL_GOVERNANCE_VOTING => FeatureFlag::PartialGovernanceVoting,
            VelorFeatureFlag::SIGNATURE_CHECKER_V2 => FeatureFlag::SignatureCheckerV2,
            VelorFeatureFlag::STORAGE_SLOT_METADATA => FeatureFlag::StorageSlotMetadata,
            VelorFeatureFlag::CHARGE_INVARIANT_VIOLATION => FeatureFlag::ChargeInvariantViolation,
            VelorFeatureFlag::DELEGATION_POOL_PARTIAL_GOVERNANCE_VOTING => {
                FeatureFlag::DelegationPoolPartialGovernanceVoting
            },
            VelorFeatureFlag::GAS_PAYER_ENABLED => FeatureFlag::GasPayerEnabled,
            VelorFeatureFlag::VELOR_UNIQUE_IDENTIFIERS => FeatureFlag::VelorUniqueIdentifiers,
            VelorFeatureFlag::BULLETPROOFS_NATIVES => FeatureFlag::BulletproofsNatives,
            VelorFeatureFlag::SIGNER_NATIVE_FORMAT_FIX => FeatureFlag::SignerNativeFormatFix,
            VelorFeatureFlag::MODULE_EVENT => FeatureFlag::ModuleEvent,
            VelorFeatureFlag::EMIT_FEE_STATEMENT => FeatureFlag::EmitFeeStatement,
            VelorFeatureFlag::STORAGE_DELETION_REFUND => FeatureFlag::StorageDeletionRefund,
            VelorFeatureFlag::AGGREGATOR_V2_API => FeatureFlag::AggregatorV2Api,
            VelorFeatureFlag::SIGNATURE_CHECKER_V2_SCRIPT_FIX => {
                FeatureFlag::SignatureCheckerV2ScriptFix
            },
            VelorFeatureFlag::SAFER_RESOURCE_GROUPS => FeatureFlag::SaferResourceGroups,
            VelorFeatureFlag::SAFER_METADATA => FeatureFlag::SaferMetadata,
            VelorFeatureFlag::SINGLE_SENDER_AUTHENTICATOR => FeatureFlag::SingleSenderAuthenticator,
            VelorFeatureFlag::SPONSORED_AUTOMATIC_ACCOUNT_V1_CREATION => {
                FeatureFlag::SponsoredAutomaticAccountCreation
            },
            VelorFeatureFlag::FEE_PAYER_ACCOUNT_OPTIONAL => FeatureFlag::FeePayerAccountOptional,
            VelorFeatureFlag::AGGREGATOR_V2_DELAYED_FIELDS => {
                FeatureFlag::AggregatorV2DelayedFields
            },
            VelorFeatureFlag::CONCURRENT_TOKEN_V2 => FeatureFlag::ConcurrentTokenV2,
            VelorFeatureFlag::LIMIT_MAX_IDENTIFIER_LENGTH => FeatureFlag::LimitMaxIdentifierLength,
            VelorFeatureFlag::OPERATOR_BENEFICIARY_CHANGE => FeatureFlag::OperatorBeneficiaryChange,
            VelorFeatureFlag::RESOURCE_GROUPS_SPLIT_IN_VM_CHANGE_SET => {
                FeatureFlag::ResourceGroupsSplitInVmChangeSet
            },
            VelorFeatureFlag::COMMISSION_CHANGE_DELEGATION_POOL => {
                FeatureFlag::CommissionChangeDelegationPool
            },
            VelorFeatureFlag::BN254_STRUCTURES => FeatureFlag::Bn254Structures,
            VelorFeatureFlag::WEBAUTHN_SIGNATURE => FeatureFlag::WebAuthnSignature,
            VelorFeatureFlag::_DEPRECATED_RECONFIGURE_WITH_DKG => FeatureFlag::ReconfigureWithDkg,
            VelorFeatureFlag::KEYLESS_ACCOUNTS => FeatureFlag::KeylessAccounts,
            VelorFeatureFlag::KEYLESS_BUT_ZKLESS_ACCOUNTS => FeatureFlag::KeylessButZklessAccounts,
            VelorFeatureFlag::_DEPRECATED_REMOVE_DETAILED_ERROR_FROM_HASH => {
                FeatureFlag::RemoveDetailedError
            },
            VelorFeatureFlag::JWK_CONSENSUS => FeatureFlag::JwkConsensus,
            VelorFeatureFlag::CONCURRENT_FUNGIBLE_ASSETS => FeatureFlag::ConcurrentFungibleAssets,
            VelorFeatureFlag::REFUNDABLE_BYTES => FeatureFlag::RefundableBytes,
            VelorFeatureFlag::OBJECT_CODE_DEPLOYMENT => FeatureFlag::ObjectCodeDeployment,
            VelorFeatureFlag::MAX_OBJECT_NESTING_CHECK => FeatureFlag::MaxObjectNestingCheck,
            VelorFeatureFlag::KEYLESS_ACCOUNTS_WITH_PASSKEYS => {
                FeatureFlag::KeylessAccountsWithPasskeys
            },
            VelorFeatureFlag::MULTISIG_V2_ENHANCEMENT => FeatureFlag::MultisigV2Enhancement,
            VelorFeatureFlag::DELEGATION_POOL_ALLOWLISTING => {
                FeatureFlag::DelegationPoolAllowlisting
            },
            VelorFeatureFlag::MODULE_EVENT_MIGRATION => FeatureFlag::ModuleEventMigration,
            VelorFeatureFlag::_REJECT_UNSTABLE_BYTECODE => FeatureFlag::RejectUnstableBytecode,
            VelorFeatureFlag::TRANSACTION_CONTEXT_EXTENSION => {
                FeatureFlag::TransactionContextExtension
            },
            VelorFeatureFlag::COIN_TO_FUNGIBLE_ASSET_MIGRATION => {
                FeatureFlag::CoinToFungibleAssetMigration
            },
            VelorFeatureFlag::PRIMARY_APT_FUNGIBLE_STORE_AT_USER_ADDRESS => {
                FeatureFlag::PrimaryAPTFungibleStoreAtUserAddress
            },
            VelorFeatureFlag::OBJECT_NATIVE_DERIVED_ADDRESS => {
                FeatureFlag::ObjectNativeDerivedAddress
            },
            VelorFeatureFlag::DISPATCHABLE_FUNGIBLE_ASSET => FeatureFlag::DispatchableFungibleAsset,
            VelorFeatureFlag::NEW_ACCOUNTS_DEFAULT_TO_FA_APT_STORE => {
                FeatureFlag::NewAccountsDefaultToFaAptStore
            },
            VelorFeatureFlag::OPERATIONS_DEFAULT_TO_FA_APT_STORE => {
                FeatureFlag::OperationsDefaultToFaAptStore
            },
            VelorFeatureFlag::AGGREGATOR_V2_IS_AT_LEAST_API => {
                FeatureFlag::AggregatorV2IsAtLeastApi
            },
            VelorFeatureFlag::CONCURRENT_FUNGIBLE_BALANCE => FeatureFlag::ConcurrentFungibleBalance,
            VelorFeatureFlag::DEFAULT_TO_CONCURRENT_FUNGIBLE_BALANCE => {
                FeatureFlag::DefaultToConcurrentFungibleBalance
            },
            VelorFeatureFlag::_LIMIT_VM_TYPE_SIZE => FeatureFlag::LimitVMTypeSize,
            VelorFeatureFlag::ABORT_IF_MULTISIG_PAYLOAD_MISMATCH => {
                FeatureFlag::AbortIfMultisigPayloadMismatch
            },
            VelorFeatureFlag::_DISALLOW_USER_NATIVES => FeatureFlag::DisallowUserNative,
            VelorFeatureFlag::ALLOW_SERIALIZED_SCRIPT_ARGS => {
                FeatureFlag::AllowSerializedScriptArgs
            },
            VelorFeatureFlag::_USE_COMPATIBILITY_CHECKER_V2 => {
                FeatureFlag::UseCompatibilityCheckerV2
            },
            VelorFeatureFlag::ENABLE_ENUM_TYPES => FeatureFlag::EnableEnumTypes,
            VelorFeatureFlag::ENABLE_RESOURCE_ACCESS_CONTROL => {
                FeatureFlag::EnableResourceAccessControl
            },
            VelorFeatureFlag::_REJECT_UNSTABLE_BYTECODE_FOR_SCRIPT => {
                FeatureFlag::RejectUnstableBytecodeForScript
            },
            VelorFeatureFlag::FEDERATED_KEYLESS => FeatureFlag::FederatedKeyless,
            VelorFeatureFlag::TRANSACTION_SIMULATION_ENHANCEMENT => {
                FeatureFlag::TransactionSimulationEnhancement
            },
            VelorFeatureFlag::COLLECTION_OWNER => FeatureFlag::CollectionOwner,
            VelorFeatureFlag::NATIVE_MEMORY_OPERATIONS => FeatureFlag::NativeMemoryOperations,
            VelorFeatureFlag::_ENABLE_LOADER_V2 => FeatureFlag::EnableLoaderV2,
            VelorFeatureFlag::_DISALLOW_INIT_MODULE_TO_PUBLISH_MODULES => {
                FeatureFlag::DisallowInitModuleToPublishModules
            },
            VelorFeatureFlag::ENABLE_CALL_TREE_AND_INSTRUCTION_VM_CACHE => {
                FeatureFlag::EnableCallTreeAndInstructionVMCache
            },
            VelorFeatureFlag::PERMISSIONED_SIGNER => FeatureFlag::PermissionedSigner,
            VelorFeatureFlag::ACCOUNT_ABSTRACTION => FeatureFlag::AccountAbstraction,
            VelorFeatureFlag::BULLETPROOFS_BATCH_NATIVES => FeatureFlag::BulletproofsBatchNatives,
            VelorFeatureFlag::DERIVABLE_ACCOUNT_ABSTRACTION => {
                FeatureFlag::DerivableAccountAbstraction
            },
            VelorFeatureFlag::ENABLE_FUNCTION_VALUES => FeatureFlag::EnableFunctionValues,
            VelorFeatureFlag::NEW_ACCOUNTS_DEFAULT_TO_FA_STORE => {
                FeatureFlag::NewAccountsDefaultToFaStore
            },
            VelorFeatureFlag::DEFAULT_ACCOUNT_RESOURCE => FeatureFlag::DefaultAccountResource,
            VelorFeatureFlag::JWK_CONSENSUS_PER_KEY_MODE => FeatureFlag::JwkConsensusPerKeyMode,
            VelorFeatureFlag::TRANSACTION_PAYLOAD_V2 => FeatureFlag::TransactionPayloadV2,
            VelorFeatureFlag::ORDERLESS_TRANSACTIONS => FeatureFlag::OrderlessTransactions,
            VelorFeatureFlag::ENABLE_LAZY_LOADING => FeatureFlag::EnableLazyLoading,
            VelorFeatureFlag::CALCULATE_TRANSACTION_FEE_FOR_DISTRIBUTION => {
                FeatureFlag::CalculateTransactionFeeForDistribution
            },
            VelorFeatureFlag::DISTRIBUTE_TRANSACTION_FEE => FeatureFlag::DistributeTransactionFee,
            VelorFeatureFlag::MONOTONICALLY_INCREASING_COUNTER => {
                FeatureFlag::MonotonicallyIncreasingCounter
            },
        }
    }
}

impl Features {
    // Compare if the current feature set is different from features that has been enabled on chain.
    pub(crate) fn has_modified(&self, on_chain_features: &VelorFeatures) -> bool {
        self.enabled
            .iter()
            .any(|f| !on_chain_features.is_enabled(VelorFeatureFlag::from(f.clone())))
            || self
                .disabled
                .iter()
                .any(|f| on_chain_features.is_enabled(VelorFeatureFlag::from(f.clone())))
    }
}

impl From<&VelorFeatures> for Features {
    fn from(features: &VelorFeatures) -> Features {
        let mut enabled = vec![];
        let mut disabled = vec![];
        for feature in FeatureFlag::iter() {
            if features.is_enabled(VelorFeatureFlag::from(feature.clone())) {
                enabled.push(feature);
            } else {
                disabled.push(feature);
            }
        }
        Features { enabled, disabled }
    }
}
