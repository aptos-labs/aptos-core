// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    aptos_vm::SerializedSigners,
    errors::{convert_epilogue_error, convert_prologue_error, expect_only_successful_execution},
    move_vm_ext::{AptosMoveResolver, SessionExt},
    system_module_names::{
        EMIT_FEE_STATEMENT, MULTISIG_ACCOUNT_MODULE, TRANSACTION_FEE_MODULE,
        VALIDATE_MULTISIG_TRANSACTION,
    },
    testing::{maybe_raise_injected_error, InjectedError},
    transaction_metadata::TransactionMetadata,
};
use aptos_gas_algebra::Gas;
use aptos_types::{
    account_config::constants::CORE_CODE_ADDRESS,
    fee_statement::FeeStatement,
    move_utils::as_move_value::AsMoveValue,
    on_chain_config::Features,
    transaction::{ReplayProtector, TransactionExecutableRef},
};
use aptos_vm_logging::log_schema::AdapterLogSchema;
use fail::fail_point;
use move_binary_format::errors::VMResult;
use move_core_types::{
    account_address::AccountAddress,
    ident_str,
    identifier::{IdentStr, Identifier},
    language_storage::ModuleId,
    value::{serialize_values, MoveValue},
    vm_status::{AbortLocation, StatusCode, VMStatus},
};
use move_vm_runtime::{
    logging::expect_no_verification_errors, module_traversal::TraversalContext, ModuleStorage,
};
use move_vm_types::gas::UnmeteredGasMeter;
use once_cell::sync::Lazy;

pub static APTOS_TRANSACTION_VALIDATION: Lazy<TransactionValidation> =
    Lazy::new(|| TransactionValidation {
        module_addr: CORE_CODE_ADDRESS,
        module_name: Identifier::new("transaction_validation").unwrap(),
        fee_payer_prologue_name: Identifier::new("fee_payer_script_prologue").unwrap(),
        script_prologue_name: Identifier::new("script_prologue").unwrap(),
        multi_agent_prologue_name: Identifier::new("multi_agent_script_prologue").unwrap(),
        user_epilogue_name: Identifier::new("epilogue").unwrap(),
        user_epilogue_gas_payer_name: Identifier::new("epilogue_gas_payer").unwrap(),
        fee_payer_prologue_extended_name: Identifier::new("fee_payer_script_prologue_extended")
            .unwrap(),
        script_prologue_extended_name: Identifier::new("script_prologue_extended").unwrap(),
        multi_agent_prologue_extended_name: Identifier::new("multi_agent_script_prologue_extended")
            .unwrap(),
        user_epilogue_extended_name: Identifier::new("epilogue_extended").unwrap(),
        user_epilogue_gas_payer_extended_name: Identifier::new("epilogue_gas_payer_extended")
            .unwrap(),
        unified_prologue_name: Identifier::new("unified_prologue").unwrap(),
        unified_prologue_fee_payer_name: Identifier::new("unified_prologue_fee_payer").unwrap(),
        unified_epilogue_name: Identifier::new("unified_epilogue").unwrap(),

        unified_prologue_v2_name: Identifier::new("unified_prologue_v2").unwrap(),
        unified_prologue_fee_payer_v2_name: Identifier::new("unified_prologue_fee_payer_v2")
            .unwrap(),
        unified_epilogue_v2_name: Identifier::new("unified_epilogue_v2").unwrap(),
    });

/// On-chain functions used to validate transactions
#[derive(Clone, Debug)]
pub struct TransactionValidation {
    pub module_addr: AccountAddress,
    pub module_name: Identifier,
    pub fee_payer_prologue_name: Identifier,
    pub script_prologue_name: Identifier,
    pub multi_agent_prologue_name: Identifier,
    pub user_epilogue_name: Identifier,
    pub user_epilogue_gas_payer_name: Identifier,
    pub fee_payer_prologue_extended_name: Identifier,
    pub script_prologue_extended_name: Identifier,
    pub multi_agent_prologue_extended_name: Identifier,
    pub user_epilogue_extended_name: Identifier,
    pub user_epilogue_gas_payer_extended_name: Identifier,
    pub unified_prologue_name: Identifier,
    pub unified_prologue_fee_payer_name: Identifier,
    pub unified_epilogue_name: Identifier,

    // Only these v2 functions support Txn Payload V2 format and Orderless transactions
    pub unified_prologue_v2_name: Identifier,
    pub unified_prologue_fee_payer_v2_name: Identifier,
    pub unified_epilogue_v2_name: Identifier,
}

impl TransactionValidation {
    pub fn module_id(&self) -> ModuleId {
        ModuleId::new(self.module_addr, self.module_name.clone())
    }

    pub fn is_account_module_abort(&self, location: &AbortLocation) -> bool {
        location == &AbortLocation::Module(self.module_id())
            || location
                == &AbortLocation::Module(ModuleId::new(
                    CORE_CODE_ADDRESS,
                    ident_str!("transaction_validation").to_owned(),
                ))
    }
}

pub(crate) fn common_prologue_serialized_args(
    txn_data: &TransactionMetadata,
    features: &Features,
    serialized_signers: &SerializedSigners,
    is_simulation: bool,
) -> Result<(&'static IdentStr, Vec<Vec<u8>>), VMStatus> {
    let txn_replay_protector = txn_data.replay_protector();
    let txn_authentication_key = txn_data.authentication_proof().optional_auth_key();
    let txn_gas_price = txn_data.gas_unit_price();
    let txn_max_gas_units = txn_data.max_gas_amount();
    let txn_expiration_timestamp_secs = txn_data.expiration_timestamp_secs();
    let chain_id = txn_data.chain_id();

    Ok(
        if features.is_account_abstraction_enabled()
            || features.is_derivable_account_abstraction_enabled()
        {
            let secondary_auth_keys: Vec<MoveValue> = txn_data
                .secondary_authentication_proofs
                .iter()
                .map(|auth_key| auth_key.optional_auth_key().as_move_value())
                .collect();
            let replay_protector_move_value = if features.is_transaction_payload_v2_enabled() {
                txn_replay_protector
                    .to_move_value()
                    .simple_serialize()
                    .unwrap()
            } else {
                match txn_replay_protector {
                    ReplayProtector::SequenceNumber(seq_num) => {
                        MoveValue::U64(seq_num).simple_serialize().unwrap()
                    },
                    ReplayProtector::Nonce(_) => {
                        unreachable!("Orderless transactions are discarded already")
                    },
                }
            };

            if let (Some(_fee_payer), Some(fee_payer_auth_key)) = (
                txn_data.fee_payer(),
                txn_data
                    .fee_payer_authentication_proof
                    .as_ref()
                    .map(|proof| proof.optional_auth_key()),
            ) {
                let serialized_args = vec![
                    serialized_signers.sender(),
                    serialized_signers
                        .fee_payer()
                        .ok_or_else(|| VMStatus::error(StatusCode::UNREACHABLE, None))?,
                    txn_authentication_key
                        .as_move_value()
                        .simple_serialize()
                        .unwrap(),
                    fee_payer_auth_key
                        .as_move_value()
                        .simple_serialize()
                        .unwrap(),
                    replay_protector_move_value,
                    MoveValue::vector_address(txn_data.secondary_signers())
                        .simple_serialize()
                        .unwrap(),
                    MoveValue::Vector(secondary_auth_keys)
                        .simple_serialize()
                        .unwrap(),
                    MoveValue::U64(txn_gas_price.into())
                        .simple_serialize()
                        .unwrap(),
                    MoveValue::U64(txn_max_gas_units.into())
                        .simple_serialize()
                        .unwrap(),
                    MoveValue::U64(txn_expiration_timestamp_secs)
                        .simple_serialize()
                        .unwrap(),
                    MoveValue::U8(chain_id.id()).simple_serialize().unwrap(),
                    MoveValue::Bool(is_simulation).simple_serialize().unwrap(),
                ];
                (
                    if features.is_transaction_payload_v2_enabled() {
                        &APTOS_TRANSACTION_VALIDATION.unified_prologue_fee_payer_v2_name
                    } else {
                        &APTOS_TRANSACTION_VALIDATION.unified_prologue_fee_payer_name
                    },
                    serialized_args,
                )
            } else {
                let serialized_args = vec![
                    serialized_signers.sender(),
                    txn_authentication_key
                        .as_move_value()
                        .simple_serialize()
                        .unwrap(),
                    replay_protector_move_value,
                    MoveValue::vector_address(txn_data.secondary_signers())
                        .simple_serialize()
                        .unwrap(),
                    MoveValue::Vector(secondary_auth_keys)
                        .simple_serialize()
                        .unwrap(),
                    MoveValue::U64(txn_gas_price.into())
                        .simple_serialize()
                        .unwrap(),
                    MoveValue::U64(txn_max_gas_units.into())
                        .simple_serialize()
                        .unwrap(),
                    MoveValue::U64(txn_expiration_timestamp_secs)
                        .simple_serialize()
                        .unwrap(),
                    MoveValue::U8(chain_id.id()).simple_serialize().unwrap(),
                    MoveValue::Bool(is_simulation).simple_serialize().unwrap(),
                ];
                (
                    if features.is_transaction_payload_v2_enabled() {
                        &APTOS_TRANSACTION_VALIDATION.unified_prologue_v2_name
                    } else {
                        &APTOS_TRANSACTION_VALIDATION.unified_prologue_name
                    },
                    serialized_args,
                )
            }
        } else {
            // Txn payload v2 format and orderless transactions are only supported with unified_prologue methods.
            // Old prologue functions do not support these features.
            let txn_sequence_number = match txn_replay_protector {
                ReplayProtector::SequenceNumber(seq_num) => seq_num,
                ReplayProtector::Nonce(_) => {
                    return Err(VMStatus::error(
                        StatusCode::FEATURE_UNDER_GATING,
                        Some(
                            "Orderless transactions is not supported without unified_prologue methods"
                                .to_string(),
                        ),
                    ));
                },
            };

            let secondary_auth_keys: Vec<MoveValue> = txn_data
                .secondary_authentication_proofs
                .iter()
                .map(|auth_key| {
                    MoveValue::vector_u8(auth_key.optional_auth_key().unwrap_or_default())
                })
                .collect();
            if let (Some(fee_payer), Some(fee_payer_auth_key)) = (
                txn_data.fee_payer(),
                txn_data
                    .fee_payer_authentication_proof
                    .as_ref()
                    .map(|proof| proof.optional_auth_key()),
            ) {
                if features.is_transaction_simulation_enhancement_enabled() {
                    let args = vec![
                        MoveValue::Signer(txn_data.sender),
                        MoveValue::U64(txn_sequence_number),
                        MoveValue::vector_u8(txn_authentication_key.unwrap_or_default()),
                        MoveValue::vector_address(txn_data.secondary_signers()),
                        MoveValue::Vector(secondary_auth_keys),
                        MoveValue::Address(fee_payer),
                        MoveValue::vector_u8(fee_payer_auth_key.unwrap_or_default()),
                        MoveValue::U64(txn_gas_price.into()),
                        MoveValue::U64(txn_max_gas_units.into()),
                        MoveValue::U64(txn_expiration_timestamp_secs),
                        MoveValue::U8(chain_id.id()),
                        MoveValue::Bool(is_simulation),
                    ];
                    (
                        &APTOS_TRANSACTION_VALIDATION.fee_payer_prologue_extended_name,
                        serialize_values(&args),
                    )
                } else {
                    let args = vec![
                        MoveValue::Signer(txn_data.sender),
                        MoveValue::U64(txn_sequence_number),
                        MoveValue::vector_u8(txn_authentication_key.unwrap_or_default()),
                        MoveValue::vector_address(txn_data.secondary_signers()),
                        MoveValue::Vector(secondary_auth_keys),
                        MoveValue::Address(fee_payer),
                        MoveValue::vector_u8(fee_payer_auth_key.unwrap_or_default()),
                        MoveValue::U64(txn_gas_price.into()),
                        MoveValue::U64(txn_max_gas_units.into()),
                        MoveValue::U64(txn_expiration_timestamp_secs),
                        MoveValue::U8(chain_id.id()),
                    ];
                    (
                        &APTOS_TRANSACTION_VALIDATION.fee_payer_prologue_name,
                        serialize_values(&args),
                    )
                }
            } else if txn_data.is_multi_agent() {
                if features.is_transaction_simulation_enhancement_enabled() {
                    let args = vec![
                        MoveValue::Signer(txn_data.sender),
                        MoveValue::U64(txn_sequence_number),
                        MoveValue::vector_u8(txn_authentication_key.unwrap_or_default()),
                        MoveValue::vector_address(txn_data.secondary_signers()),
                        MoveValue::Vector(secondary_auth_keys),
                        MoveValue::U64(txn_gas_price.into()),
                        MoveValue::U64(txn_max_gas_units.into()),
                        MoveValue::U64(txn_expiration_timestamp_secs),
                        MoveValue::U8(chain_id.id()),
                        MoveValue::Bool(is_simulation),
                    ];
                    (
                        &APTOS_TRANSACTION_VALIDATION.multi_agent_prologue_extended_name,
                        serialize_values(&args),
                    )
                } else {
                    let args = vec![
                        MoveValue::Signer(txn_data.sender),
                        MoveValue::U64(txn_sequence_number),
                        MoveValue::vector_u8(txn_authentication_key.unwrap_or_default()),
                        MoveValue::vector_address(txn_data.secondary_signers()),
                        MoveValue::Vector(secondary_auth_keys),
                        MoveValue::U64(txn_gas_price.into()),
                        MoveValue::U64(txn_max_gas_units.into()),
                        MoveValue::U64(txn_expiration_timestamp_secs),
                        MoveValue::U8(chain_id.id()),
                    ];
                    (
                        &APTOS_TRANSACTION_VALIDATION.multi_agent_prologue_name,
                        serialize_values(&args),
                    )
                }
            } else {
                #[allow(clippy::collapsible_else_if)]
                if features.is_transaction_simulation_enhancement_enabled() {
                    let args = vec![
                        MoveValue::Signer(txn_data.sender),
                        MoveValue::U64(txn_sequence_number),
                        MoveValue::vector_u8(txn_authentication_key.unwrap_or_default()),
                        MoveValue::U64(txn_gas_price.into()),
                        MoveValue::U64(txn_max_gas_units.into()),
                        MoveValue::U64(txn_expiration_timestamp_secs),
                        MoveValue::U8(chain_id.id()),
                        MoveValue::vector_u8(txn_data.script_hash.clone()),
                        MoveValue::Bool(is_simulation),
                    ];
                    (
                        &APTOS_TRANSACTION_VALIDATION.script_prologue_extended_name,
                        serialize_values(&args),
                    )
                } else {
                    let args = vec![
                        MoveValue::Signer(txn_data.sender),
                        MoveValue::U64(txn_sequence_number),
                        MoveValue::vector_u8(txn_authentication_key.unwrap_or_default()),
                        MoveValue::U64(txn_gas_price.into()),
                        MoveValue::U64(txn_max_gas_units.into()),
                        MoveValue::U64(txn_expiration_timestamp_secs),
                        MoveValue::U8(chain_id.id()),
                        MoveValue::vector_u8(txn_data.script_hash.clone()),
                    ];
                    (
                        &APTOS_TRANSACTION_VALIDATION.script_prologue_name,
                        serialize_values(&args),
                    )
                }
            }
        },
    )
}

pub(crate) fn multisig_prologue_args(
    txn_metadata: &TransactionMetadata,
    features: &Features,
    multisig_address: AccountAddress,
    executable: &TransactionExecutableRef,
) -> Result<Vec<MoveValue>, VMStatus> {
    let provided_payload = executable.get_provided_payload_bytes(features)?;
    Ok(vec![
        MoveValue::Signer(txn_metadata.sender),
        MoveValue::Address(multisig_address),
        MoveValue::vector_u8(provided_payload),
    ])
}

pub(crate) fn run_script_prologue(
    session: &mut SessionExt<impl AptosMoveResolver>,
    module_storage: &impl ModuleStorage,
    serialized_signers: &SerializedSigners,
    txn_data: &TransactionMetadata,
    features: &Features,
    log_context: &AdapterLogSchema,
    traversal_context: &mut TraversalContext,
    is_simulation: bool,
) -> Result<(), VMStatus> {
    let (prologue_function_name, args) =
        common_prologue_serialized_args(txn_data, features, serialized_signers, is_simulation)?;

    session
        .execute_function_bypass_visibility(
            &APTOS_TRANSACTION_VALIDATION.module_id(),
            prologue_function_name,
            vec![],
            args,
            &mut UnmeteredGasMeter,
            traversal_context,
            module_storage,
        )
        .map(|_return_vals| ())
        .map_err(expect_no_verification_errors)
        .or_else(|err| convert_prologue_error(err, log_context))
}

/// Run the prologue for a multisig transaction. This needs to verify that:
/// 1. The multisig tx exists
/// 2. It has received enough approvals to meet the signature threshold of the multisig account
/// 3. If only the payload hash was stored on chain, the provided payload in execution should
/// match that hash.
pub(crate) fn run_multisig_prologue(
    session: &mut SessionExt<impl AptosMoveResolver>,
    module_storage: &impl ModuleStorage,
    txn_data: &TransactionMetadata,
    executable: TransactionExecutableRef,
    multisig_address: AccountAddress,
    features: &Features,
    log_context: &AdapterLogSchema,
    traversal_context: &mut TraversalContext,
) -> Result<(), VMStatus> {
    let args = multisig_prologue_args(txn_data, features, multisig_address, &executable)?;
    session
        .execute_function_bypass_visibility(
            &MULTISIG_ACCOUNT_MODULE,
            VALIDATE_MULTISIG_TRANSACTION,
            vec![],
            serialize_values(&args),
            &mut UnmeteredGasMeter,
            traversal_context,
            module_storage,
        )
        .map(|_return_vals| ())
        .map_err(expect_no_verification_errors)
        .or_else(|err| convert_prologue_error(err, log_context))
}

pub(crate) fn epilogue_serialized_args(
    txn_data: &TransactionMetadata,
    features: &Features,
    serialized_signers: &SerializedSigners,
    fee_statement: &FeeStatement,
    gas_remaining: Gas,
    is_simulation: bool,
) -> (&'static IdentStr, Vec<Vec<u8>>) {
    let txn_gas_price = txn_data.gas_unit_price();
    let txn_max_gas_units = txn_data.max_gas_amount();
    let is_orderless_txn = txn_data.is_orderless();

    if features.is_account_abstraction_enabled()
        || features.is_derivable_account_abstraction_enabled()
    {
        let mut serialize_args = vec![
            serialized_signers.sender(),
            serialized_signers
                .fee_payer()
                .unwrap_or(serialized_signers.sender()),
            MoveValue::U64(fee_statement.storage_fee_refund())
                .simple_serialize()
                .unwrap(),
            MoveValue::U64(txn_gas_price.into())
                .simple_serialize()
                .unwrap(),
            MoveValue::U64(txn_max_gas_units.into())
                .simple_serialize()
                .unwrap(),
            MoveValue::U64(gas_remaining.into())
                .simple_serialize()
                .unwrap(),
            MoveValue::Bool(is_simulation).simple_serialize().unwrap(),
        ];
        if features.is_transaction_payload_v2_enabled() {
            serialize_args.push(
                MoveValue::Bool(is_orderless_txn)
                    .simple_serialize()
                    .unwrap(),
            );
        }

        let function_name = if features.is_transaction_payload_v2_enabled() {
            &APTOS_TRANSACTION_VALIDATION.unified_epilogue_v2_name
        } else {
            &APTOS_TRANSACTION_VALIDATION.unified_epilogue_name
        };

        (function_name, serialize_args)
    } else {
        // We can unconditionally do this as this condition can only be true if the prologue
        // accepted it, in which case the gas payer feature is enabled.
        if let Some(fee_payer) = txn_data.fee_payer() {
            if features.is_transaction_simulation_enhancement_enabled() {
                let args = vec![
                    MoveValue::Signer(txn_data.sender),
                    MoveValue::Address(fee_payer),
                    MoveValue::U64(fee_statement.storage_fee_refund()),
                    MoveValue::U64(txn_gas_price.into()),
                    MoveValue::U64(txn_max_gas_units.into()),
                    MoveValue::U64(gas_remaining.into()),
                    MoveValue::Bool(is_simulation),
                ];
                (
                    &APTOS_TRANSACTION_VALIDATION.user_epilogue_gas_payer_extended_name,
                    serialize_values(&args),
                )
            } else {
                let args = vec![
                    MoveValue::Signer(txn_data.sender),
                    MoveValue::Address(fee_payer),
                    MoveValue::U64(fee_statement.storage_fee_refund()),
                    MoveValue::U64(txn_gas_price.into()),
                    MoveValue::U64(txn_max_gas_units.into()),
                    MoveValue::U64(gas_remaining.into()),
                ];
                (
                    &APTOS_TRANSACTION_VALIDATION.user_epilogue_gas_payer_name,
                    serialize_values(&args),
                )
            }
        } else {
            // Regular tx, run the normal epilogue
            if features.is_transaction_simulation_enhancement_enabled() {
                let args = vec![
                    MoveValue::Signer(txn_data.sender),
                    MoveValue::U64(fee_statement.storage_fee_refund()),
                    MoveValue::U64(txn_gas_price.into()),
                    MoveValue::U64(txn_max_gas_units.into()),
                    MoveValue::U64(gas_remaining.into()),
                    MoveValue::Bool(is_simulation),
                ];
                (
                    &APTOS_TRANSACTION_VALIDATION.user_epilogue_extended_name,
                    serialize_values(&args),
                )
            } else {
                let args = vec![
                    MoveValue::Signer(txn_data.sender),
                    MoveValue::U64(fee_statement.storage_fee_refund()),
                    MoveValue::U64(txn_gas_price.into()),
                    MoveValue::U64(txn_max_gas_units.into()),
                    MoveValue::U64(gas_remaining.into()),
                ];
                (
                    &APTOS_TRANSACTION_VALIDATION.user_epilogue_name,
                    serialize_values(&args),
                )
            }
        }
    }
}

fn run_epilogue(
    session: &mut SessionExt<impl AptosMoveResolver>,
    module_storage: &impl ModuleStorage,
    serialized_signers: &SerializedSigners,
    gas_remaining: Gas,
    fee_statement: FeeStatement,
    txn_data: &TransactionMetadata,
    features: &Features,
    traversal_context: &mut TraversalContext,
    is_simulation: bool,
) -> VMResult<()> {
    let (function_name, args) = epilogue_serialized_args(
        txn_data,
        features,
        serialized_signers,
        &fee_statement,
        gas_remaining,
        is_simulation,
    );

    session
        .execute_function_bypass_visibility(
            &APTOS_TRANSACTION_VALIDATION.module_id(),
            function_name,
            vec![],
            args,
            &mut UnmeteredGasMeter,
            traversal_context,
            module_storage,
        )
        .map_err(expect_no_verification_errors)?;

    // Emit the FeeStatement event
    if features.is_emit_fee_statement_enabled() {
        emit_fee_statement(session, module_storage, fee_statement, traversal_context)?;
    }

    maybe_raise_injected_error(InjectedError::EndOfRunEpilogue)?;

    Ok(())
}

fn emit_fee_statement(
    session: &mut SessionExt<impl AptosMoveResolver>,
    module_storage: &impl ModuleStorage,
    fee_statement: FeeStatement,
    traversal_context: &mut TraversalContext,
) -> VMResult<()> {
    session.execute_function_bypass_visibility(
        &TRANSACTION_FEE_MODULE,
        EMIT_FEE_STATEMENT,
        vec![],
        vec![bcs::to_bytes(&fee_statement).expect("Failed to serialize fee statement")],
        &mut UnmeteredGasMeter,
        traversal_context,
        module_storage,
    )?;
    Ok(())
}

/// Run the epilogue of a transaction by calling into `EPILOGUE_NAME` function stored
/// in the `ACCOUNT_MODULE` on chain.
pub(crate) fn run_success_epilogue(
    session: &mut SessionExt<impl AptosMoveResolver>,
    module_storage: &impl ModuleStorage,
    serialized_signers: &SerializedSigners,
    gas_remaining: Gas,
    fee_statement: FeeStatement,
    features: &Features,
    txn_data: &TransactionMetadata,
    log_context: &AdapterLogSchema,
    traversal_context: &mut TraversalContext,
    is_simulation: bool,
) -> Result<(), VMStatus> {
    fail_point!("move_adapter::run_success_epilogue", |_| {
        Err(VMStatus::error(
            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
            None,
        ))
    });

    run_epilogue(
        session,
        module_storage,
        serialized_signers,
        gas_remaining,
        fee_statement,
        txn_data,
        features,
        traversal_context,
        is_simulation,
    )
    .or_else(|err| convert_epilogue_error(err, log_context))
}

/// Run the failure epilogue of a transaction by calling into `USER_EPILOGUE_NAME` function
/// stored in the `ACCOUNT_MODULE` on chain.
pub(crate) fn run_failure_epilogue(
    session: &mut SessionExt<impl AptosMoveResolver>,
    module_storage: &impl ModuleStorage,
    serialized_signers: &SerializedSigners,
    gas_remaining: Gas,
    fee_statement: FeeStatement,
    features: &Features,
    txn_data: &TransactionMetadata,
    log_context: &AdapterLogSchema,
    traversal_context: &mut TraversalContext,
    is_simulation: bool,
) -> Result<(), VMStatus> {
    run_epilogue(
        session,
        module_storage,
        serialized_signers,
        gas_remaining,
        fee_statement,
        txn_data,
        features,
        traversal_context,
        is_simulation,
    )
    .or_else(|err| {
        expect_only_successful_execution(
            err,
            APTOS_TRANSACTION_VALIDATION.user_epilogue_name.as_str(),
            log_context,
        )
    })
}
