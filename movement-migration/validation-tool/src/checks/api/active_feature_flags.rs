// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::checks::error::ValidationError;
use crate::types::api::MovementAptosRestClient;
use aptos_rest_client::aptos_api_types::ViewFunction;
use move_core_types::identifier::Identifier;
use move_core_types::language_storage::ModuleId;
use std::str::FromStr;
use tracing::debug;

pub struct GlobalFeatureCheck;

impl GlobalFeatureCheck {
    pub async fn satisfies(
        movement_aptos_rest_client: &MovementAptosRestClient,
    ) -> Result<(), ValidationError> {
        let mut errors = vec![];
        let expected_active: Vec<u64> = vec![
            1,  // FeatureFlag::CODE_DEPENDENCY_CHECK
            2,  // FeatureFlag::TREAT_FRIEND_AS_PRIVATE
            3,  // FeatureFlag::SHA_512_AND_RIPEMD_160_NATIVES
            4,  // FeatureFlag::APTOS_STD_CHAIN_ID_NATIVES
            5,  // FeatureFlag::VM_BINARY_FORMAT_V6
            7,  // FeatureFlag::MULTI_ED25519_PK_VALIDATE_V2_NATIVES
            8,  // FeatureFlag::BLAKE2B_256_NATIVE
            9,  // FeatureFlag::RESOURCE_GROUPS
            10, // FeatureFlag::MULTISIG_ACCOUNTS
            11, // FeatureFlag::DELEGATION_POOLS
            12, // FeatureFlag::CRYPTOGRAPHY_ALGEBRA_NATIVES
            13, // FeatureFlag::BLS12_381_STRUCTURES
            14, // FeatureFlag::ED25519_PUBKEY_VALIDATE_RETURN_FALSE_WRONG_LENGTH
            15, // FeatureFlag::STRUCT_CONSTRUCTORS
            18, // FeatureFlag::SIGNATURE_CHECKER_V2
            19, // FeatureFlag::STORAGE_SLOT_METADATA
            20, // FeatureFlag::CHARGE_INVARIANT_VIOLATION
            22, // FeatureFlag::GAS_PAYER_ENABLED
            23, // FeatureFlag::APTOS_UNIQUE_IDENTIFIERS
            24, // FeatureFlag::BULLETPROOFS_NATIVES
            25, // FeatureFlag::SIGNER_NATIVE_FORMAT_FIX
            26, // FeatureFlag::MODULE_EVENT
            27, // FeatureFlag::EMIT_FEE_STATEMENT
            28, // FeatureFlag::STORAGE_DELETION_REFUND
            29, // FeatureFlag::SIGNATURE_CHECKER_V2_SCRIPT_FIX
            30, // FeatureFlag::AGGREGATOR_V2_API
            31, // FeatureFlag::SAFER_RESOURCE_GROUPS
            32, // FeatureFlag::SAFER_METADATA
            33, // FeatureFlag::SINGLE_SENDER_AUTHENTICATOR
            34, // FeatureFlag::SPONSORED_AUTOMATIC_ACCOUNT_V1_CREATION
            35, // FeatureFlag::FEE_PAYER_ACCOUNT_OPTIONAL
            36, // FeatureFlag::AGGREGATOR_V2_DELAYED_FIELDS
            37, // FeatureFlag::CONCURRENT_TOKEN_V2
            38, // FeatureFlag::LIMIT_MAX_IDENTIFIER_LENGTH
            39, // FeatureFlag::OPERATOR_BENEFICIARY_CHANGE
            41, // FeatureFlag::RESOURCE_GROUPS_SPLIT_IN_VM_CHANGE_SET
            42, // FeatureFlag::COMMISSION_CHANGE_DELEGATION_POOL
            43, // FeatureFlag::BN254_STRUCTURES
            44, // FeatureFlag::WEBAUTHN_SIGNATURE
            46, // FeatureFlag::KEYLESS_ACCOUNTS
            47, // FeatureFlag::KEYLESS_BUT_ZKLESS_ACCOUNTS
            48, // FeatureFlag::REMOVE_DETAILED_ERROR_FROM_HASH
            49, // FeatureFlag::JWK_CONSENSUS
            50, // FeatureFlag::CONCURRENT_FUNGIBLE_ASSETS
            51, // FeatureFlag::REFUNDABLE_BYTES
            52, // FeatureFlag::OBJECT_CODE_DEPLOYMENT
            53, // FeatureFlag::MAX_OBJECT_NESTING_CHECK
            54, // FeatureFlag::KEYLESS_ACCOUNTS_WITH_PASSKEYS
            55, // FeatureFlag::MULTISIG_V2_ENHANCEMENT
            56, // FeatureFlag::DELEGATION_POOL_ALLOWLISTING
            57, // FeatureFlag::MODULE_EVENT_MIGRATION
            58, // FeatureFlag::REJECT_UNSTABLE_BYTECODE
            59, // FeatureFlag::TRANSACTION_CONTEXT_EXTENSION
            60, // FeatureFlag::COIN_TO_FUNGIBLE_ASSET_MIGRATION
            62, // FeatureFlag::OBJECT_NATIVE_DERIVED_ADDRESS
            63, // FeatureFlag::DISPATCHABLE_FUNGIBLE_ASSET
            66, // FeatureFlag::AGGREGATOR_V2_IS_AT_LEAST_API
            67, // FeatureFlag::CONCURRENT_FUNGIBLE_BALANCE
            69, // FeatureFlag::LIMIT_VM_TYPE_SIZE
            70, // FeatureFlag::ABORT_IF_MULTISIG_PAYLOAD_MISMATCH
            73, // FeatureFlag::GOVERNED_GAS_POOL
        ];

        let module =
            ModuleId::from_str("0x1::features").map_err(|e| ValidationError::Internal(e.into()))?;
        let function =
            Identifier::from_str("is_enabled").map_err(|e| ValidationError::Internal(e.into()))?;

        let mut view_function = ViewFunction {
            module,
            function,
            ty_args: vec![],
            args: vec![],
        };

        for feature_id in expected_active {
            debug!("checking feature flag {}", feature_id);
            let bytes =
                bcs::to_bytes(&feature_id).map_err(|e| ValidationError::Internal(e.into()))?;
            view_function.args = vec![bytes];

            // Check feature for Maptos executor
            let maptos_active = movement_aptos_rest_client
                .view_bcs_with_json_response(&view_function, None)
                .await
                .map_err(|e| {
                    ValidationError::Internal(
                        format!(
                            "failed to get Movement feature flag {}: {:?}",
                            feature_id, e
                        )
                        .into(),
                    )
                })?
                .into_inner();

            let maptos_active = maptos_active.get(0).ok_or_else(|| {
                ValidationError::Internal(
                    format!(
                        "failed to get Movement feature flag {}: response is empty",
                        feature_id
                    )
                    .into(),
                )
            })?;

            let maptos_active = maptos_active.as_bool().ok_or_else(|| {
                ValidationError::Internal(
                    format!(
                        "failed to get Movement feature flag {}: can't convert {:?} into a bool",
                        feature_id, maptos_active
                    )
                    .into(),
                )
            })?;

            if !maptos_active {
                errors.push(format!(
                    "Feature {}: Aptos={} — expected to be active",
                    feature_id, maptos_active,
                ));
            }

            // Slow down to avoid Cloudflare rate limiting
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }

        if !errors.is_empty() {
            return Err(ValidationError::Unsatisfied(errors.join("\n").into()));
        }

        Ok(())
    }
}
