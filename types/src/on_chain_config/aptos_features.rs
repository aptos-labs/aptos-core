// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::on_chain_config::OnChainConfig;
use serde::{Deserialize, Serialize};

/// The feature flags define in the Move source. This must stay aligned with the constants there.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[allow(non_camel_case_types)]
pub enum FeatureFlag {
    CODE_DEPENDENCY_CHECK = 1,
    TREAT_FRIEND_AS_PRIVATE = 2,
    SHA_512_AND_RIPEMD_160_NATIVES = 3,
    APTOS_STD_CHAIN_ID_NATIVES = 4,
    VM_BINARY_FORMAT_V6 = 5,
    COLLECT_AND_DISTRIBUTE_GAS_FEES = 6,
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
    SIGNATURE_CHECKER_V2 = 18,
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
    CONCURRENT_ASSETS = 37,
    LIMIT_MAX_IDENTIFIER_LENGTH = 38,
    OPERATOR_BENEFICIARY_CHANGE = 39,
    VM_BINARY_FORMAT_V7 = 40,
    RESOURCE_GROUPS_CHARGE_AS_SIZE_SUM = 41,
    COMMISSION_CHANGE_DELEGATION_POOL = 42,
    BN254_STRUCTURES = 43,
    WEBAUTHN_SIGNATURE = 44,
    RECONFIGURE_WITH_DKG = 45,
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

        use FeatureFlag::*;
        features.enable(VM_BINARY_FORMAT_V6);
        features.enable(BLS12_381_STRUCTURES);
        features.enable(SIGNATURE_CHECKER_V2);
        features.enable(STORAGE_SLOT_METADATA);
        features.enable(APTOS_UNIQUE_IDENTIFIERS);
        features.enable(SIGNATURE_CHECKER_V2_SCRIPT_FIX);
        features.enable(AGGREGATOR_V2_API);
        features.enable(BN254_STRUCTURES);
        features
    }
}

impl OnChainConfig for Features {
    const MODULE_IDENTIFIER: &'static str = "features";
    const TYPE_IDENTIFIER: &'static str = "Features";
}

impl Features {
    pub fn enable(&mut self, flag: FeatureFlag) {
        let byte_index = (flag as u64 / 8) as usize;
        let bit_mask = 1 << (flag as u64 % 8);
        while self.features.len() <= byte_index {
            self.features.push(0);
        }

        self.features[byte_index] |= bit_mask;
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

    /// Whether the Aggregator V2 API feature is enabled.
    /// Once enabled, the functions from aggregator_v2.move will be available for use.
    pub fn is_aggregator_v2_api_enabled(&self) -> bool {
        self.is_enabled(FeatureFlag::AGGREGATOR_V2_API)
    }

    /// Whether the Aggregator V2 delayed fields feature is enabled.
    /// Once enabled, Aggregator V2 functions become parallel.
    pub fn is_aggregator_v2_delayed_fields_enabled(&self) -> bool {
        // This feature depends on resource groups being split inside VMChange set,
        // which is gated by RESOURCE_GROUPS_CHARGE_AS_SIZE_SUM feature, so
        // require that feature to be enabled as well.
        self.is_enabled(FeatureFlag::AGGREGATOR_V2_DELAYED_FIELDS)
            && self.is_resource_group_charge_as_size_sum_enabled()
    }

    pub fn is_resource_group_charge_as_size_sum_enabled(&self) -> bool {
        self.is_enabled(FeatureFlag::RESOURCE_GROUPS_CHARGE_AS_SIZE_SUM)
    }
}
