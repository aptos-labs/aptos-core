// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::on_chain_config::OnChainConfig;
use move_binary_format::{
    file_format_common,
    file_format_common::{IDENTIFIER_SIZE_MAX, LEGACY_IDENTIFIER_SIZE_MAX},
};
use move_core_types::{
    effects::{ChangeSet, Op},
    language_storage::CORE_CODE_ADDRESS,
};
use serde::{Deserialize, Serialize};
use strum_macros::{EnumString, FromRepr};

/// The feature flags defined in the Move source. This must stay aligned with the constants there.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, FromRepr, EnumString)]
#[allow(non_camel_case_types)]
pub enum FeatureFlag {
    CODE_DEPENDENCY_CHECK = 1,
    TREAT_FRIEND_AS_PRIVATE = 2,
    SHA_512_AND_RIPEMD_160_NATIVES = 3,
    APTOS_STD_CHAIN_ID_NATIVES = 4,
    VM_BINARY_FORMAT_V6 = 5,
    _DEPRECATED_COLLECT_AND_DISTRIBUTE_GAS_FEES = 6,
    MULTI_ED25519_PK_VALIDATE_V2_NATIVES = 7,
    BLAKE2B_256_NATIVE = 8,
    RESOURCE_GROUPS = 9,
    MULTISIG_ACCOUNTS = 10,
    DELEGATION_POOLS = 11,
    CRYPTOGRAPHY_ALGEBRA_NATIVES = 12,
    BLS12_381_STRUCTURES = 13,
    ED25519_PUBKEY_VALIDATE_RETURN_FALSE_WRONG_LENGTH = 14,
    STRUCT_CONSTRUCTORS = 15,
    PERIODICAL_REWARD_RATE_DECREASE = 16,
    PARTIAL_GOVERNANCE_VOTING = 17,
    /// Enabled on mainnet and cannot be disabled
    _SIGNATURE_CHECKER_V2 = 18,
    STORAGE_SLOT_METADATA = 19,
    CHARGE_INVARIANT_VIOLATION = 20,
    DELEGATION_POOL_PARTIAL_GOVERNANCE_VOTING = 21,
    GAS_PAYER_ENABLED = 22,
    APTOS_UNIQUE_IDENTIFIERS = 23,
    BULLETPROOFS_NATIVES = 24,
    SIGNER_NATIVE_FORMAT_FIX = 25,
    MODULE_EVENT = 26,
    EMIT_FEE_STATEMENT = 27,
    STORAGE_DELETION_REFUND = 28,
    SIGNATURE_CHECKER_V2_SCRIPT_FIX = 29,
    AGGREGATOR_V2_API = 30,
    SAFER_RESOURCE_GROUPS = 31,
    SAFER_METADATA = 32,
    SINGLE_SENDER_AUTHENTICATOR = 33,
    SPONSORED_AUTOMATIC_ACCOUNT_V1_CREATION = 34,
    FEE_PAYER_ACCOUNT_OPTIONAL = 35,
    AGGREGATOR_V2_DELAYED_FIELDS = 36,
    CONCURRENT_TOKEN_V2 = 37,
    LIMIT_MAX_IDENTIFIER_LENGTH = 38,
    OPERATOR_BENEFICIARY_CHANGE = 39,
    VM_BINARY_FORMAT_V7 = 40,
    RESOURCE_GROUPS_SPLIT_IN_VM_CHANGE_SET = 41,
    COMMISSION_CHANGE_DELEGATION_POOL = 42,
    BN254_STRUCTURES = 43,
    WEBAUTHN_SIGNATURE = 44,
    _DEPRECATED_RECONFIGURE_WITH_DKG = 45,
    KEYLESS_ACCOUNTS = 46,
    KEYLESS_BUT_ZKLESS_ACCOUNTS = 47,
    /// This feature was never used.
    _DEPRECATED_REMOVE_DETAILED_ERROR_FROM_HASH = 48,
    JWK_CONSENSUS = 49,
    CONCURRENT_FUNGIBLE_ASSETS = 50,
    REFUNDABLE_BYTES = 51,
    OBJECT_CODE_DEPLOYMENT = 52,
    MAX_OBJECT_NESTING_CHECK = 53,
    KEYLESS_ACCOUNTS_WITH_PASSKEYS = 54,
    MULTISIG_V2_ENHANCEMENT = 55,
    DELEGATION_POOL_ALLOWLISTING = 56,
    MODULE_EVENT_MIGRATION = 57,
    /// Enabled on mainnet, can never be disabled.
    _REJECT_UNSTABLE_BYTECODE = 58,
    TRANSACTION_CONTEXT_EXTENSION = 59,
    COIN_TO_FUNGIBLE_ASSET_MIGRATION = 60,
    PRIMARY_APT_FUNGIBLE_STORE_AT_USER_ADDRESS = 61,
    // Feature rolled out, no longer can be disabled.
    _OBJECT_NATIVE_DERIVED_ADDRESS = 62,
    DISPATCHABLE_FUNGIBLE_ASSET = 63,
    NEW_ACCOUNTS_DEFAULT_TO_FA_APT_STORE = 64,
    OPERATIONS_DEFAULT_TO_FA_APT_STORE = 65,
    // Feature rolled out, no longer can be disabled.
    _AGGREGATOR_V2_IS_AT_LEAST_API = 66,
    CONCURRENT_FUNGIBLE_BALANCE = 67,
    DEFAULT_TO_CONCURRENT_FUNGIBLE_BALANCE = 68,
    /// Enabled on mainnet, cannot be disabled.
    _LIMIT_VM_TYPE_SIZE = 69,
    ABORT_IF_MULTISIG_PAYLOAD_MISMATCH = 70,
    /// Enabled on mainnet, cannot be disabled.
    _DISALLOW_USER_NATIVES = 71,
    ALLOW_SERIALIZED_SCRIPT_ARGS = 72,
    /// Enabled on mainnet, cannot be disabled.
    _USE_COMPATIBILITY_CHECKER_V2 = 73,
    ENABLE_ENUM_TYPES = 74,
    ENABLE_RESOURCE_ACCESS_CONTROL = 75,
    /// Enabled on mainnet, can never be disabled.
    _REJECT_UNSTABLE_BYTECODE_FOR_SCRIPT = 76,
    FEDERATED_KEYLESS = 77,
    TRANSACTION_SIMULATION_ENHANCEMENT = 78,
    COLLECTION_OWNER = 79,
    /// Enabled on mainnet, cannot be rolled back. Was gating `mem::swap` and `vector::move_range`
    /// natives. For more details, see:
    ///   AIP-105 (https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-105.md)
    _NATIVE_MEMORY_OPERATIONS = 80,
    /// The feature was used to gate the rollout of new loader used by Move VM. It was enabled on
    /// mainnet and can no longer be disabled.
    _ENABLE_LOADER_V2 = 81,
    /// Prior to this feature flag, it was possible to attempt 'init_module' to publish modules
    /// that results in a new package created but without any code. With this feature, it is no
    /// longer possible and an explicit error is returned if publishing is attempted. The feature
    /// was enabled on mainnet and will not be disabled.
    _DISALLOW_INIT_MODULE_TO_PUBLISH_MODULES = 82,
    /// We keep the Call Tree cache and instruction (per-instruction)
    /// cache together here.  Generally, we could allow Call Tree
    /// cache and disallow instruction cache, however there's little
    /// benefit of such approach: First, instruction cache requires
    /// call-tree cache to be enabled, and provides relatively little
    /// overhead in terms of memory footprint. On the other side,
    /// providing separate choices could lead to code bloat, as the
    /// dynamic config is converted into multiple different
    /// implementations. If required in the future, we can add a flag
    /// to explicitly disable the instruction cache.
    ENABLE_CALL_TREE_AND_INSTRUCTION_VM_CACHE = 83,
    /// AIP-103 (https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-103.md)
    PERMISSIONED_SIGNER = 84,
    ACCOUNT_ABSTRACTION = 85,
    /// Enables bytecode version v8
    VM_BINARY_FORMAT_V8 = 86,
    BULLETPROOFS_BATCH_NATIVES = 87,
    DERIVABLE_ACCOUNT_ABSTRACTION = 88,
    /// Whether function values are enabled.
    ENABLE_FUNCTION_VALUES = 89,
    NEW_ACCOUNTS_DEFAULT_TO_FA_STORE = 90,
    DEFAULT_ACCOUNT_RESOURCE = 91,
    JWK_CONSENSUS_PER_KEY_MODE = 92,
    TRANSACTION_PAYLOAD_V2 = 93,
    ORDERLESS_TRANSACTIONS = 94,
    /// With lazy loading, modules are loaded lazily (as opposed to loading the transitive closure
    /// of dependencies). For more details, see:
    ///   AIP-127 (https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-127.md)
    ENABLE_LAZY_LOADING = 95,
    CALCULATE_TRANSACTION_FEE_FOR_DISTRIBUTION = 96,
    DISTRIBUTE_TRANSACTION_FEE = 97,
    MONOTONICALLY_INCREASING_COUNTER = 98,
    ENABLE_CAPTURE_OPTION = 99,
    /// Whether to allow trusted code optimizations.
    ENABLE_TRUSTED_CODE = 100,
    ENABLE_ENUM_OPTION = 101,
    /// Enables bytecode version v9
    VM_BINARY_FORMAT_V9 = 102,
    ENABLE_FRAMEWORK_FOR_OPTION = 103,
    /// If enabled, new single session is used by the VM to avoid squashing write-sets and cache
    /// reads between sessions (e.g., between transaction prologue, user session and epilogue).
    SESSION_CONTINUATION = 104,
    /// Enables function value reflection in the stdlib
    ENABLE_FUNCTION_REFLECTION = 105,
    /// Enables bytecode version v10
    VM_BINARY_FORMAT_V10 = 106,
    /// Whether SLH-DSA-SHA2-128s signature scheme is enabled for transaction authentication.
    SLH_DSA_SHA2_128S_SIGNATURE = 107,
}

impl FeatureFlag {
    pub fn default_features() -> Vec<Self> {
        vec![
            Self::CODE_DEPENDENCY_CHECK,
            Self::TREAT_FRIEND_AS_PRIVATE,
            Self::SHA_512_AND_RIPEMD_160_NATIVES,
            Self::APTOS_STD_CHAIN_ID_NATIVES,
            // Feature flag V6 is used to enable metadata v1 format and needs to stay on, even
            // if we enable a higher version.
            Self::VM_BINARY_FORMAT_V6,
            Self::VM_BINARY_FORMAT_V7,
            Self::MULTI_ED25519_PK_VALIDATE_V2_NATIVES,
            Self::BLAKE2B_256_NATIVE,
            Self::RESOURCE_GROUPS,
            Self::MULTISIG_ACCOUNTS,
            Self::DELEGATION_POOLS,
            Self::CRYPTOGRAPHY_ALGEBRA_NATIVES,
            Self::BLS12_381_STRUCTURES,
            Self::ED25519_PUBKEY_VALIDATE_RETURN_FALSE_WRONG_LENGTH,
            Self::STRUCT_CONSTRUCTORS,
            Self::PERIODICAL_REWARD_RATE_DECREASE,
            Self::PARTIAL_GOVERNANCE_VOTING,
            Self::_SIGNATURE_CHECKER_V2,
            Self::STORAGE_SLOT_METADATA,
            Self::CHARGE_INVARIANT_VIOLATION,
            Self::DELEGATION_POOL_PARTIAL_GOVERNANCE_VOTING,
            Self::APTOS_UNIQUE_IDENTIFIERS,
            Self::GAS_PAYER_ENABLED,
            Self::BULLETPROOFS_NATIVES,
            Self::SIGNER_NATIVE_FORMAT_FIX,
            Self::MODULE_EVENT,
            Self::EMIT_FEE_STATEMENT,
            Self::STORAGE_DELETION_REFUND,
            Self::SIGNATURE_CHECKER_V2_SCRIPT_FIX,
            Self::AGGREGATOR_V2_API,
            Self::SAFER_RESOURCE_GROUPS,
            Self::SAFER_METADATA,
            Self::SINGLE_SENDER_AUTHENTICATOR,
            Self::SPONSORED_AUTOMATIC_ACCOUNT_V1_CREATION,
            Self::FEE_PAYER_ACCOUNT_OPTIONAL,
            Self::AGGREGATOR_V2_DELAYED_FIELDS,
            Self::CONCURRENT_TOKEN_V2,
            Self::LIMIT_MAX_IDENTIFIER_LENGTH,
            Self::OPERATOR_BENEFICIARY_CHANGE,
            Self::BN254_STRUCTURES,
            Self::RESOURCE_GROUPS_SPLIT_IN_VM_CHANGE_SET,
            Self::COMMISSION_CHANGE_DELEGATION_POOL,
            Self::WEBAUTHN_SIGNATURE,
            Self::KEYLESS_ACCOUNTS,
            Self::FEDERATED_KEYLESS,
            Self::KEYLESS_BUT_ZKLESS_ACCOUNTS,
            Self::JWK_CONSENSUS,
            Self::REFUNDABLE_BYTES,
            Self::OBJECT_CODE_DEPLOYMENT,
            Self::MAX_OBJECT_NESTING_CHECK,
            Self::KEYLESS_ACCOUNTS_WITH_PASSKEYS,
            Self::MULTISIG_V2_ENHANCEMENT,
            Self::DELEGATION_POOL_ALLOWLISTING,
            Self::MODULE_EVENT_MIGRATION,
            Self::_REJECT_UNSTABLE_BYTECODE,
            Self::TRANSACTION_CONTEXT_EXTENSION,
            Self::COIN_TO_FUNGIBLE_ASSET_MIGRATION,
            Self::_OBJECT_NATIVE_DERIVED_ADDRESS,
            Self::DISPATCHABLE_FUNGIBLE_ASSET,
            Self::NEW_ACCOUNTS_DEFAULT_TO_FA_APT_STORE,
            Self::OPERATIONS_DEFAULT_TO_FA_APT_STORE,
            Self::CONCURRENT_FUNGIBLE_ASSETS,
            Self::_AGGREGATOR_V2_IS_AT_LEAST_API,
            Self::CONCURRENT_FUNGIBLE_BALANCE,
            Self::_LIMIT_VM_TYPE_SIZE,
            Self::ABORT_IF_MULTISIG_PAYLOAD_MISMATCH,
            Self::_DISALLOW_USER_NATIVES,
            Self::ALLOW_SERIALIZED_SCRIPT_ARGS,
            Self::_USE_COMPATIBILITY_CHECKER_V2,
            Self::ENABLE_ENUM_TYPES,
            Self::ENABLE_RESOURCE_ACCESS_CONTROL,
            Self::_REJECT_UNSTABLE_BYTECODE_FOR_SCRIPT,
            Self::TRANSACTION_SIMULATION_ENHANCEMENT,
            Self::_NATIVE_MEMORY_OPERATIONS,
            Self::_ENABLE_LOADER_V2,
            Self::_DISALLOW_INIT_MODULE_TO_PUBLISH_MODULES,
            Self::COLLECTION_OWNER,
            Self::PERMISSIONED_SIGNER,
            Self::ENABLE_CALL_TREE_AND_INSTRUCTION_VM_CACHE,
            Self::ACCOUNT_ABSTRACTION,
            Self::BULLETPROOFS_BATCH_NATIVES,
            Self::DERIVABLE_ACCOUNT_ABSTRACTION,
            Self::VM_BINARY_FORMAT_V8,
            Self::ENABLE_FUNCTION_VALUES,
            Self::NEW_ACCOUNTS_DEFAULT_TO_FA_STORE,
            Self::DEFAULT_ACCOUNT_RESOURCE,
            Self::JWK_CONSENSUS_PER_KEY_MODE,
            Self::TRANSACTION_PAYLOAD_V2,
            Self::ORDERLESS_TRANSACTIONS,
            Self::CALCULATE_TRANSACTION_FEE_FOR_DISTRIBUTION,
            Self::DISTRIBUTE_TRANSACTION_FEE,
            Self::ENABLE_LAZY_LOADING,
            Self::MONOTONICALLY_INCREASING_COUNTER,
            Self::ENABLE_CAPTURE_OPTION,
            Self::ENABLE_TRUSTED_CODE,
            Self::ENABLE_ENUM_OPTION,
            Self::VM_BINARY_FORMAT_V9,
            Self::ENABLE_FRAMEWORK_FOR_OPTION,
            Self::ENABLE_FUNCTION_REFLECTION,
            Self::VM_BINARY_FORMAT_V10,
            Self::SLH_DSA_SHA2_128S_SIGNATURE,
        ]
    }
}

/// Representation of features on chain as a bitset.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Features {
    #[serde(with = "serde_bytes")]
    pub features: Vec<u8>,
}

impl Default for Features {
    fn default() -> Self {
        let mut features = Features {
            features: vec![0; 5],
        };

        for feature in FeatureFlag::default_features() {
            features.enable(feature);
        }
        features
    }
}

impl OnChainConfig for Features {
    const MODULE_IDENTIFIER: &'static str = "features";
    const TYPE_IDENTIFIER: &'static str = "Features";
}

impl Features {
    /// Returns default features for testing. This is the same as `default()` except features are
    /// added or removed to ensure available runtime checks are enabled during tests.
    pub fn default_for_tests() -> Self {
        let mut features = Self::default();
        // Do not trust any code during testing, but verify it at runtime.
        features.disable(FeatureFlag::ENABLE_TRUSTED_CODE);
        features
    }

    fn resize_for_flag(&mut self, flag: FeatureFlag) -> (usize, u8) {
        let byte_index = (flag as u64 / 8) as usize;
        let bit_mask = 1 << (flag as u64 % 8);
        while self.features.len() <= byte_index {
            self.features.push(0);
        }
        (byte_index, bit_mask)
    }

    pub fn enable(&mut self, flag: FeatureFlag) {
        let (byte_index, bit_mask) = self.resize_for_flag(flag);
        self.features[byte_index] |= bit_mask;
    }

    pub fn disable(&mut self, flag: FeatureFlag) {
        let (byte_index, bit_mask) = self.resize_for_flag(flag);
        self.features[byte_index] &= !bit_mask;
    }

    pub fn into_flag_vec(self) -> Vec<FeatureFlag> {
        let Self { features } = self;
        features
            .into_iter()
            .flat_map(|byte| (0..8).map(move |bit_idx| byte & (1 << bit_idx) != 0))
            .enumerate()
            .filter(|(_feature_idx, enabled)| *enabled)
            .map(|(feature_idx, _)| FeatureFlag::from_repr(feature_idx).unwrap())
            .collect()
    }

    pub fn is_enabled(&self, flag: FeatureFlag) -> bool {
        let val = flag as u64;
        let byte_index = (val / 8) as usize;
        let bit_mask = 1 << (val % 8);
        byte_index < self.features.len() && (self.features[byte_index] & bit_mask != 0)
    }

    pub fn are_resource_groups_enabled(&self) -> bool {
        self.is_enabled(FeatureFlag::RESOURCE_GROUPS)
    }

    pub fn is_storage_slot_metadata_enabled(&self) -> bool {
        self.is_enabled(FeatureFlag::STORAGE_SLOT_METADATA)
    }

    pub fn is_account_abstraction_enabled(&self) -> bool {
        self.is_enabled(FeatureFlag::ACCOUNT_ABSTRACTION)
    }

    pub fn is_derivable_account_abstraction_enabled(&self) -> bool {
        self.is_enabled(FeatureFlag::DERIVABLE_ACCOUNT_ABSTRACTION)
    }

    pub fn is_module_event_enabled(&self) -> bool {
        self.is_enabled(FeatureFlag::MODULE_EVENT)
    }

    pub fn is_emit_fee_statement_enabled(&self) -> bool {
        // requires module events
        self.is_module_event_enabled() && self.is_enabled(FeatureFlag::EMIT_FEE_STATEMENT)
    }

    pub fn is_storage_deletion_refund_enabled(&self) -> bool {
        // requires emit fee statement
        self.is_emit_fee_statement_enabled()
            && self.is_enabled(FeatureFlag::STORAGE_DELETION_REFUND)
    }

    /// Whether the Aggregator V2 delayed fields feature is enabled.
    /// Once enabled, Aggregator V2 functions become parallel.
    pub fn is_aggregator_v2_delayed_fields_enabled(&self) -> bool {
        // This feature depends on resource groups being split inside VMChange set,
        // which is gated by RESOURCE_GROUPS_SPLIT_IN_VM_CHANGE_SET feature, so
        // require that feature to be enabled as well.
        self.is_enabled(FeatureFlag::AGGREGATOR_V2_DELAYED_FIELDS)
            && self.is_resource_groups_split_in_vm_change_set_enabled()
    }

    pub fn is_resource_groups_split_in_vm_change_set_enabled(&self) -> bool {
        self.is_enabled(FeatureFlag::RESOURCE_GROUPS_SPLIT_IN_VM_CHANGE_SET)
    }

    /// Whether the keyless accounts feature is enabled, specifically the ZK path with ZKP-based signatures.
    /// The ZK-less path is controlled via a different `FeatureFlag::KEYLESS_BUT_ZKLESS_ACCOUNTS` flag.
    pub fn is_zk_keyless_enabled(&self) -> bool {
        self.is_enabled(FeatureFlag::KEYLESS_ACCOUNTS)
    }

    /// If `FeatureFlag::KEYLESS_ACCOUNTS` is enabled, this feature additionally allows for a "ZK-less
    /// path" where the blockchain can verify OpenID signatures directly. This ZK-less mode exists
    /// for two reasons. First, it gives as a simpler way to test the feature. Second, it acts as a
    /// safety precaution in case of emergency (e.g., if the ZK-based signatures must be temporarily
    /// turned off due to a zeroday exploit, the ZK-less path will still allow users to transact,
    /// but without privacy).
    pub fn is_zkless_keyless_enabled(&self) -> bool {
        self.is_enabled(FeatureFlag::KEYLESS_BUT_ZKLESS_ACCOUNTS)
    }

    pub fn is_keyless_with_passkeys_enabled(&self) -> bool {
        self.is_enabled(FeatureFlag::KEYLESS_ACCOUNTS_WITH_PASSKEYS)
    }

    pub fn is_federated_keyless_enabled(&self) -> bool {
        self.is_enabled(FeatureFlag::FEDERATED_KEYLESS)
    }

    pub fn is_refundable_bytes_enabled(&self) -> bool {
        self.is_enabled(FeatureFlag::REFUNDABLE_BYTES)
    }

    pub fn is_abort_if_multisig_payload_mismatch_enabled(&self) -> bool {
        self.is_enabled(FeatureFlag::ABORT_IF_MULTISIG_PAYLOAD_MISMATCH)
    }

    pub fn is_transaction_simulation_enhancement_enabled(&self) -> bool {
        self.is_enabled(FeatureFlag::TRANSACTION_SIMULATION_ENHANCEMENT)
    }

    pub fn is_call_tree_and_instruction_vm_cache_enabled(&self) -> bool {
        self.is_enabled(FeatureFlag::ENABLE_CALL_TREE_AND_INSTRUCTION_VM_CACHE)
    }

    pub fn is_lazy_loading_enabled(&self) -> bool {
        self.is_enabled(FeatureFlag::ENABLE_LAZY_LOADING)
    }

    pub fn is_trusted_code_enabled(&self) -> bool {
        self.is_enabled(FeatureFlag::ENABLE_TRUSTED_CODE)
    }

    pub fn is_default_account_resource_enabled(&self) -> bool {
        self.is_enabled(FeatureFlag::DEFAULT_ACCOUNT_RESOURCE)
    }

    pub fn is_new_account_default_to_fa_store(&self) -> bool {
        self.is_enabled(FeatureFlag::NEW_ACCOUNTS_DEFAULT_TO_FA_STORE)
    }

    pub fn is_transaction_payload_v2_enabled(&self) -> bool {
        self.is_enabled(FeatureFlag::TRANSACTION_PAYLOAD_V2)
    }

    pub fn is_orderless_txns_enabled(&self) -> bool {
        self.is_enabled(FeatureFlag::ORDERLESS_TRANSACTIONS)
    }

    pub fn is_calculate_transaction_fee_for_distribution_enabled(&self) -> bool {
        self.is_enabled(FeatureFlag::CALCULATE_TRANSACTION_FEE_FOR_DISTRIBUTION)
    }

    pub fn is_distribute_transaction_fee_enabled(&self) -> bool {
        self.is_enabled(FeatureFlag::DISTRIBUTE_TRANSACTION_FEE)
    }

    pub fn is_enum_option_enabled(&self) -> bool {
        self.is_enabled(FeatureFlag::ENABLE_ENUM_OPTION)
    }

    pub fn is_session_continuation_enabled(&self) -> bool {
        self.is_enabled(FeatureFlag::SESSION_CONTINUATION)
    }

    pub fn get_max_identifier_size(&self) -> u64 {
        if self.is_enabled(FeatureFlag::LIMIT_MAX_IDENTIFIER_LENGTH) {
            IDENTIFIER_SIZE_MAX
        } else {
            LEGACY_IDENTIFIER_SIZE_MAX
        }
    }

    pub fn get_max_binary_format_version(&self) -> u32 {
        if self.is_enabled(FeatureFlag::VM_BINARY_FORMAT_V10) {
            file_format_common::VERSION_10
        } else if self.is_enabled(FeatureFlag::VM_BINARY_FORMAT_V9) {
            file_format_common::VERSION_9
        } else if self.is_enabled(FeatureFlag::VM_BINARY_FORMAT_V8) {
            file_format_common::VERSION_8
        } else if self.is_enabled(FeatureFlag::VM_BINARY_FORMAT_V7) {
            file_format_common::VERSION_7
        } else if self.is_enabled(FeatureFlag::VM_BINARY_FORMAT_V6) {
            file_format_common::VERSION_6
        } else {
            file_format_common::VERSION_5
        }
    }
}

pub fn aptos_test_feature_flags_genesis() -> ChangeSet {
    let features_value = bcs::to_bytes(&Features::default_for_tests()).unwrap();

    let mut change_set = ChangeSet::new();
    // we need to initialize features to their defaults.
    change_set
        .add_resource_op(
            CORE_CODE_ADDRESS,
            Features::struct_tag(),
            Op::New(features_value.into()),
        )
        .expect("adding genesis Feature resource must succeed");

    change_set
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_features_into_flag_vec() {
        let mut features = Features { features: vec![] };
        features.enable(FeatureFlag::BLS12_381_STRUCTURES);
        features.enable(FeatureFlag::BN254_STRUCTURES);

        assert_eq!(
            vec![
                FeatureFlag::BLS12_381_STRUCTURES,
                FeatureFlag::BN254_STRUCTURES
            ],
            features.into_flag_vec()
        );
    }

    #[test]
    fn test_min_max_binary_format() {
        // Ensure querying max binary format implementation is correct and checks
        // versions 5 to 8.
        assert_eq!(
            file_format_common::VERSION_5,
            file_format_common::VERSION_MIN
        );
        assert_eq!(
            file_format_common::VERSION_10,
            file_format_common::VERSION_MAX
        );
    }
}
