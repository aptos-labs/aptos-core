// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    access_path_cache::AccessPathCache,
    data_cache::StorageAdapter,
    errors::{convert_epilogue_error, convert_prologue_error, expect_only_successful_execution},
    move_vm_ext::{MoveResolverExt, MoveVmExt, SessionExt, SessionId},
    system_module_names::{MULTISIG_ACCOUNT_MODULE, VALIDATE_MULTISIG_TRANSACTION},
    transaction_metadata::TransactionMetadata,
};
use aptos_framework::RuntimeModuleMetadataV1;
use aptos_gas::{
    AbstractValueSizeGasParameters, AptosGasParameters, ChangeSetConfigs, FromOnChainGasSchedule,
    Gas, NativeGasParameters, StorageGasParameters, StoragePricing,
};
use aptos_logger::{enabled, prelude::*, Level};
use aptos_state_view::StateView;
use aptos_types::{
    account_config::{TransactionValidation, APTOS_TRANSACTION_VALIDATION, CORE_CODE_ADDRESS},
    chain_id::ChainId,
    fee_statement::FeeStatement,
    on_chain_config::{
        ApprovedExecutionHashes, ConfigurationResource, FeatureFlag, Features, GasSchedule,
        GasScheduleV2, OnChainConfig, TimedFeatures, Version,
    },
    transaction::{AbortInfo, ExecutionStatus, Multisig, TransactionStatus},
    vm_status::{StatusCode, VMStatus},
};
use aptos_vm_logging::{log_schema::AdapterLogSchema, prelude::*};
use aptos_vm_types::output::VMOutput;
use fail::fail_point;
use move_binary_format::{errors::VMResult, CompiledModule};
use move_core_types::{
    gas_algebra::NumArgs,
    language_storage::ModuleId,
    move_resource::MoveStructType,
    value::{serialize_values, MoveValue},
};
use move_vm_runtime::logging::expect_no_verification_errors;
use move_vm_types::gas::UnmeteredGasMeter;
use std::sync::Arc;

pub const MAXIMUM_APPROVED_TRANSACTION_SIZE: u64 = 1024 * 1024;

/// A wrapper to make VMRuntime standalone
pub struct AptosVMImpl {
    move_vm: MoveVmExt,
    gas_feature_version: u64,
    gas_params: Option<AptosGasParameters>,
    storage_gas_params: Option<StorageGasParameters>,
    version: Option<Version>,
    transaction_validation: Option<TransactionValidation>,
    features: Features,
}

pub fn gas_config(storage: &impl MoveResolverExt) -> (Option<AptosGasParameters>, u64) {
    match GasScheduleV2::fetch_config(storage) {
        Some(gas_schedule) => {
            let feature_version = gas_schedule.feature_version;
            let map = gas_schedule.to_btree_map();
            (
                AptosGasParameters::from_on_chain_gas_schedule(&map, feature_version),
                feature_version,
            )
        },
        None => match GasSchedule::fetch_config(storage) {
            Some(gas_schedule) => {
                let map = gas_schedule.to_btree_map();
                (AptosGasParameters::from_on_chain_gas_schedule(&map, 0), 0)
            },
            None => (None, 0),
        },
    }
}

impl AptosVMImpl {
    #[allow(clippy::new_without_default)]
    pub fn new(state: &impl StateView) -> Self {
        let storage = StorageAdapter::new(state);

        // Get the gas parameters
        let (mut gas_params, gas_feature_version): (Option<AptosGasParameters>, u64) =
            gas_config(&storage);

        let storage_gas_params = if let Some(gas_params) = &mut gas_params {
            let storage_gas_params =
                StorageGasParameters::new(gas_feature_version, gas_params, &storage);

            if let StoragePricing::V2(pricing) = &storage_gas_params.pricing {
                // Overwrite table io gas parameters with global io pricing.
                let g = &mut gas_params.natives.table.common;
                match gas_feature_version {
                    0..=1 => (),
                    2..=6 => {
                        g.load_base_legacy = pricing.per_item_read * NumArgs::new(1);
                        g.load_base_new = 0.into();
                        g.load_per_byte = pricing.per_byte_read;
                        g.load_failure = 0.into();
                    },
                    7.. => {
                        g.load_base_legacy = 0.into();
                        g.load_base_new = pricing.per_item_read * NumArgs::new(1);
                        g.load_per_byte = pricing.per_byte_read;
                        g.load_failure = 0.into();
                    },
                }
            }
            Some(storage_gas_params)
        } else {
            None
        };

        // TODO(Gas): Right now, we have to use some dummy values for gas parameters if they are not found on-chain.
        //            This only happens in a edge case that is probably related to write set transactions or genesis,
        //            which logically speaking, shouldn't be handled by the VM at all.
        //            We should clean up the logic here once we get that refactored.
        let (native_gas_params, abs_val_size_gas_params) = match &gas_params {
            Some(gas_params) => (gas_params.natives.clone(), gas_params.misc.abs_val.clone()),
            None => (
                NativeGasParameters::zeros(),
                AbstractValueSizeGasParameters::zeros(),
            ),
        };

        let features = Features::fetch_config(&storage).unwrap_or_default();

        // If no chain ID is in storage, we assume we are in a testing environment and use ChainId::TESTING
        let chain_id = ChainId::fetch_config(&storage).unwrap_or_else(ChainId::test);

        let timestamp = ConfigurationResource::fetch_config(&storage)
            .map(|config| config.last_reconfiguration_time())
            .unwrap_or(0);

        let mut timed_features = TimedFeatures::new(chain_id, timestamp);
        if let Some(profile) = crate::AptosVM::get_timed_feature_override() {
            timed_features = timed_features.with_override_profile(profile)
        }

        let move_vm = MoveVmExt::new(
            native_gas_params,
            abs_val_size_gas_params,
            gas_feature_version,
            chain_id.id(),
            features.clone(),
            timed_features,
        )
        .expect("should be able to create Move VM; check if there are duplicated natives");

        let version = Version::fetch_config(&storage);
        let transaction_validation = Self::get_transaction_validation(&storage);

        Self {
            move_vm,
            gas_feature_version,
            gas_params,
            storage_gas_params,
            version,
            transaction_validation,
            features,
        }
    }

    pub(crate) fn mark_loader_cache_as_invalid(&self) {
        self.move_vm.mark_loader_cache_as_invalid();
    }

    /// Provides access to some internal APIs of the VM.
    pub fn internals(&self) -> AptosVMInternals {
        AptosVMInternals(self)
    }

    pub(crate) fn transaction_validation(&self) -> &TransactionValidation {
        self.transaction_validation
            .as_ref()
            .unwrap_or(&APTOS_TRANSACTION_VALIDATION)
    }

    // TODO: Move this to an on-chain config once those are a part of the core framework
    fn get_transaction_validation(
        resolver: &impl MoveResolverExt,
    ) -> Option<TransactionValidation> {
        match resolver
            .get_resource(&CORE_CODE_ADDRESS, &TransactionValidation::struct_tag())
            .ok()?
        {
            Some(blob) => bcs::from_bytes::<TransactionValidation>(&blob).ok(),
            _ => None,
        }
    }

    pub fn get_gas_parameters(
        &self,
        log_context: &AdapterLogSchema,
    ) -> Result<&AptosGasParameters, VMStatus> {
        self.gas_params.as_ref().ok_or_else(|| {
            speculative_error!(
                log_context,
                "VM Startup Failed. Gas Parameters Not Found".into()
            );
            VMStatus::Error(StatusCode::VM_STARTUP_FAILURE, None)
        })
    }

    pub fn get_storage_gas_parameters(
        &self,
        log_context: &AdapterLogSchema,
    ) -> Result<&StorageGasParameters, VMStatus> {
        self.storage_gas_params.as_ref().ok_or_else(|| {
            speculative_error!(
                log_context,
                "VM Startup Failed. Storage Gas Parameters Not Found".into()
            );
            VMStatus::Error(StatusCode::VM_STARTUP_FAILURE, None)
        })
    }

    pub fn get_gas_feature_version(&self) -> u64 {
        self.gas_feature_version
    }

    pub fn get_version(&self) -> Result<Version, VMStatus> {
        self.version.clone().ok_or_else(|| {
            alert!("VM Startup Failed. Version Not Found");
            VMStatus::Error(StatusCode::VM_STARTUP_FAILURE, None)
        })
    }

    pub fn get_features(&self) -> &Features {
        &self.features
    }

    pub fn check_gas(
        &self,
        resolver: &impl MoveResolverExt,
        txn_data: &TransactionMetadata,
        log_context: &AdapterLogSchema,
    ) -> Result<(), VMStatus> {
        let txn_gas_params = &self.get_gas_parameters(log_context)?.txn;
        let raw_bytes_len = txn_data.transaction_size;
        // The transaction is too large.
        if txn_data.transaction_size > txn_gas_params.max_transaction_size_in_bytes {
            let data =
                resolver.get_resource(&CORE_CODE_ADDRESS, &ApprovedExecutionHashes::struct_tag());

            let valid = if let Ok(Some(data)) = data {
                let approved_execution_hashes =
                    bcs::from_bytes::<ApprovedExecutionHashes>(&data).ok();
                let valid = approved_execution_hashes
                    .map(|aeh| {
                        aeh.entries
                            .into_iter()
                            .any(|(_, hash)| hash == txn_data.script_hash)
                    })
                    .unwrap_or(false);
                valid
                    // If it is valid ensure that it is only the approved payload that exceeds the
                    // maximum. The (unknown) user input should be restricted to the original
                    // maximum transaction size.
                    && (txn_data.script_size + txn_gas_params.max_transaction_size_in_bytes
                        > txn_data.transaction_size)
                    // Since an approved transaction can be sent by anyone, the system is safer by
                    // enforcing an upper limit on governance transactions just so something really
                    // bad doesn't happen.
                    && txn_data.transaction_size <= MAXIMUM_APPROVED_TRANSACTION_SIZE.into()
            } else {
                false
            };

            if !valid {
                speculative_warn!(
                    log_context,
                    format!(
                        "[VM] Transaction size too big {} (max {})",
                        raw_bytes_len, txn_gas_params.max_transaction_size_in_bytes
                    ),
                );
                return Err(VMStatus::Error(
                    StatusCode::EXCEEDED_MAX_TRANSACTION_SIZE,
                    None,
                ));
            }
        }

        // The submitted max gas units that the transaction can consume is greater than the
        // maximum number of gas units bound that we have set for any
        // transaction.
        if txn_data.max_gas_amount() > txn_gas_params.maximum_number_of_gas_units {
            speculative_warn!(
                log_context,
                format!(
                    "[VM] Gas unit error; max {}, submitted {}",
                    txn_gas_params.maximum_number_of_gas_units,
                    txn_data.max_gas_amount()
                ),
            );
            return Err(VMStatus::Error(
                StatusCode::MAX_GAS_UNITS_EXCEEDS_MAX_GAS_UNITS_BOUND,
                None,
            ));
        }

        // The submitted transactions max gas units needs to be at least enough to cover the
        // intrinsic cost of the transaction as calculated against the size of the
        // underlying `RawTransaction`
        let intrinsic_gas: Gas = txn_gas_params
            .calculate_intrinsic_gas(raw_bytes_len)
            .to_unit_round_up_with_params(txn_gas_params);

        if txn_data.max_gas_amount() < intrinsic_gas {
            speculative_warn!(
                log_context,
                format!(
                    "[VM] Gas unit error; min {}, submitted {}",
                    intrinsic_gas,
                    txn_data.max_gas_amount()
                ),
            );
            return Err(VMStatus::Error(
                StatusCode::MAX_GAS_UNITS_BELOW_MIN_TRANSACTION_GAS_UNITS,
                None,
            ));
        }

        // The submitted gas price is less than the minimum gas unit price set by the VM.
        // NB: MIN_PRICE_PER_GAS_UNIT may equal zero, but need not in the future. Hence why
        // we turn off the clippy warning.
        #[allow(clippy::absurd_extreme_comparisons)]
        let below_min_bound = txn_data.gas_unit_price() < txn_gas_params.min_price_per_gas_unit;
        if below_min_bound {
            speculative_warn!(
                log_context,
                format!(
                    "[VM] Gas unit error; min {}, submitted {}",
                    txn_gas_params.min_price_per_gas_unit,
                    txn_data.gas_unit_price()
                ),
            );
            return Err(VMStatus::Error(
                StatusCode::GAS_UNIT_PRICE_BELOW_MIN_BOUND,
                None,
            ));
        }

        // The submitted gas price is greater than the maximum gas unit price set by the VM.
        if txn_data.gas_unit_price() > txn_gas_params.max_price_per_gas_unit {
            speculative_warn!(
                log_context,
                format!(
                    "[VM] Gas unit error; min {}, submitted {}",
                    txn_gas_params.max_price_per_gas_unit,
                    txn_data.gas_unit_price()
                ),
            );
            return Err(VMStatus::Error(
                StatusCode::GAS_UNIT_PRICE_ABOVE_MAX_BOUND,
                None,
            ));
        }
        Ok(())
    }

    /// Run the prologue of a transaction by calling into either `SCRIPT_PROLOGUE_NAME` function
    /// or `MULTI_AGENT_SCRIPT_PROLOGUE_NAME` function stored in the `ACCOUNT_MODULE` on chain.
    pub(crate) fn run_script_prologue(
        &self,
        session: &mut SessionExt,
        txn_data: &TransactionMetadata,
        log_context: &AdapterLogSchema,
    ) -> Result<(), VMStatus> {
        let transaction_validation = self.transaction_validation();
        let txn_sequence_number = txn_data.sequence_number();
        let txn_authentication_key = txn_data.authentication_key().to_vec();
        let txn_gas_price = txn_data.gas_unit_price();
        let txn_max_gas_units = txn_data.max_gas_amount();
        let txn_expiration_timestamp_secs = txn_data.expiration_timestamp_secs();
        let chain_id = txn_data.chain_id();
        let mut gas_meter = UnmeteredGasMeter;
        let secondary_auth_keys: Vec<MoveValue> = txn_data
            .secondary_authentication_keys
            .iter()
            .map(|auth_key| MoveValue::vector_u8(auth_key.to_vec()))
            .collect();
        let args = if txn_data.is_multi_agent() {
            vec![
                MoveValue::Signer(txn_data.sender),
                MoveValue::U64(txn_sequence_number),
                MoveValue::vector_u8(txn_authentication_key),
                MoveValue::vector_address(txn_data.secondary_signers()),
                MoveValue::Vector(secondary_auth_keys),
                MoveValue::U64(txn_gas_price.into()),
                MoveValue::U64(txn_max_gas_units.into()),
                MoveValue::U64(txn_expiration_timestamp_secs),
                MoveValue::U8(chain_id.id()),
            ]
        } else {
            vec![
                MoveValue::Signer(txn_data.sender),
                MoveValue::U64(txn_sequence_number),
                MoveValue::vector_u8(txn_authentication_key),
                MoveValue::U64(txn_gas_price.into()),
                MoveValue::U64(txn_max_gas_units.into()),
                MoveValue::U64(txn_expiration_timestamp_secs),
                MoveValue::U8(chain_id.id()),
                MoveValue::vector_u8(txn_data.script_hash.clone()),
            ]
        };
        let prologue_function_name = if txn_data.is_multi_agent() {
            &transaction_validation.multi_agent_prologue_name
        } else {
            &transaction_validation.script_prologue_name
        };
        session
            .execute_function_bypass_visibility(
                &transaction_validation.module_id(),
                prologue_function_name,
                // TODO: Deprecate this once we remove gas currency on the Move side.
                vec![],
                serialize_values(&args),
                &mut gas_meter,
            )
            .map(|_return_vals| ())
            .map_err(expect_no_verification_errors)
            .or_else(|err| convert_prologue_error(transaction_validation, err, log_context))
    }

    /// Run the prologue of a transaction by calling into `MODULE_PROLOGUE_NAME` function stored
    /// in the `ACCOUNT_MODULE` on chain.
    pub(crate) fn run_module_prologue(
        &self,
        session: &mut SessionExt,
        txn_data: &TransactionMetadata,
        log_context: &AdapterLogSchema,
    ) -> Result<(), VMStatus> {
        let transaction_validation = self.transaction_validation();

        let txn_sequence_number = txn_data.sequence_number();
        let txn_authentication_key = txn_data.authentication_key();
        let txn_gas_price = txn_data.gas_unit_price();
        let txn_max_gas_units = txn_data.max_gas_amount();
        let txn_expiration_timestamp_secs = txn_data.expiration_timestamp_secs();
        let chain_id = txn_data.chain_id();
        let mut gas_meter = UnmeteredGasMeter;
        session
            .execute_function_bypass_visibility(
                &transaction_validation.module_id(),
                &transaction_validation.module_prologue_name,
                // TODO: Deprecate this once we remove gas currency on the Move side.
                vec![],
                serialize_values(&vec![
                    MoveValue::Signer(txn_data.sender),
                    MoveValue::U64(txn_sequence_number),
                    MoveValue::vector_u8(txn_authentication_key.to_vec()),
                    MoveValue::U64(txn_gas_price.into()),
                    MoveValue::U64(txn_max_gas_units.into()),
                    MoveValue::U64(txn_expiration_timestamp_secs),
                    MoveValue::U8(chain_id.id()),
                ]),
                &mut gas_meter,
            )
            .map(|_return_vals| ())
            .map_err(expect_no_verification_errors)
            .or_else(|err| convert_prologue_error(transaction_validation, err, log_context))
    }

    /// Run the prologue for a multisig transaction. This needs to verify that:
    /// 1. The the multisig tx exists
    /// 2. It has received enough approvals to meet the signature threshold of the multisig account
    /// 3. If only the payload hash was stored on chain, the provided payload in execution should
    /// match that hash.
    pub(crate) fn run_multisig_prologue(
        &self,
        session: &mut SessionExt,
        txn_data: &TransactionMetadata,
        payload: &Multisig,
        log_context: &AdapterLogSchema,
    ) -> Result<(), VMStatus> {
        let transaction_validation = self.transaction_validation();
        let unreachable_error = VMStatus::Error(StatusCode::UNREACHABLE, None);
        let provided_payload = if let Some(payload) = &payload.transaction_payload {
            bcs::to_bytes(&payload).map_err(|_| unreachable_error.clone())?
        } else {
            // Default to empty bytes if payload is not provided.
            bcs::to_bytes::<Vec<u8>>(&vec![]).map_err(|_| unreachable_error)?
        };

        session
            .execute_function_bypass_visibility(
                &MULTISIG_ACCOUNT_MODULE,
                VALIDATE_MULTISIG_TRANSACTION,
                vec![],
                serialize_values(&vec![
                    MoveValue::Signer(txn_data.sender),
                    MoveValue::Address(payload.multisig_address),
                    MoveValue::vector_u8(provided_payload),
                ]),
                &mut UnmeteredGasMeter,
            )
            .map(|_return_vals| ())
            .map_err(expect_no_verification_errors)
            .or_else(|err| convert_prologue_error(transaction_validation, err, log_context))
    }

    /// Run the epilogue of a transaction by calling into `EPILOGUE_NAME` function stored
    /// in the `ACCOUNT_MODULE` on chain.
    pub(crate) fn run_success_epilogue(
        &self,
        session: &mut SessionExt,
        gas_remaining: Gas,
        txn_data: &TransactionMetadata,
        log_context: &AdapterLogSchema,
    ) -> Result<(), VMStatus> {
        fail_point!("move_adapter::run_success_epilogue", |_| {
            Err(VMStatus::Error(
                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                None,
            ))
        });

        let transaction_validation = self.transaction_validation();
        let txn_sequence_number = txn_data.sequence_number();
        let txn_gas_price = txn_data.gas_unit_price();
        let txn_max_gas_units = txn_data.max_gas_amount();
        session
            .execute_function_bypass_visibility(
                &transaction_validation.module_id(),
                &transaction_validation.user_epilogue_name,
                // TODO: Deprecate this once we remove gas currency on the Move side.
                vec![],
                serialize_values(&vec![
                    MoveValue::Signer(txn_data.sender),
                    MoveValue::U64(txn_sequence_number),
                    MoveValue::U64(txn_gas_price.into()),
                    MoveValue::U64(txn_max_gas_units.into()),
                    MoveValue::U64(gas_remaining.into()),
                ]),
                &mut UnmeteredGasMeter,
            )
            .map(|_return_vals| ())
            .map_err(expect_no_verification_errors)
            .or_else(|err| convert_epilogue_error(transaction_validation, err, log_context))
    }

    /// Run the failure epilogue of a transaction by calling into `USER_EPILOGUE_NAME` function
    /// stored in the `ACCOUNT_MODULE` on chain.
    pub(crate) fn run_failure_epilogue(
        &self,
        session: &mut SessionExt,
        gas_remaining: Gas,
        txn_data: &TransactionMetadata,
        log_context: &AdapterLogSchema,
    ) -> Result<(), VMStatus> {
        let transaction_validation = self.transaction_validation();
        let txn_sequence_number = txn_data.sequence_number();
        let txn_gas_price = txn_data.gas_unit_price();
        let txn_max_gas_units = txn_data.max_gas_amount();
        session
            .execute_function_bypass_visibility(
                &transaction_validation.module_id(),
                &transaction_validation.user_epilogue_name,
                // TODO: Deprecate this once we remove gas currency on the Move side.
                vec![],
                serialize_values(&vec![
                    MoveValue::Signer(txn_data.sender),
                    MoveValue::U64(txn_sequence_number),
                    MoveValue::U64(txn_gas_price.into()),
                    MoveValue::U64(txn_max_gas_units.into()),
                    MoveValue::U64(gas_remaining.into()),
                ]),
                &mut UnmeteredGasMeter,
            )
            .map(|_return_vals| ())
            .map_err(expect_no_verification_errors)
            .or_else(|e| {
                expect_only_successful_execution(
                    e,
                    transaction_validation.user_epilogue_name.as_str(),
                    log_context,
                )
            })
    }

    pub(crate) fn extract_abort_info(
        &self,
        module: &ModuleId,
        abort_code: u64,
    ) -> Option<AbortInfo> {
        if let Some(m) = self.extract_module_metadata(module) {
            m.extract_abort_info(abort_code)
        } else {
            None
        }
    }

    pub(crate) fn extract_module_metadata(
        &self,
        module: &ModuleId,
    ) -> Option<RuntimeModuleMetadataV1> {
        if self.features.is_enabled(FeatureFlag::VM_BINARY_FORMAT_V6) {
            aptos_framework::get_vm_metadata(&self.move_vm, module)
        } else {
            aptos_framework::get_vm_metadata_v0(&self.move_vm, module)
        }
    }

    pub fn new_session<'r>(
        &self,
        resolver: &'r impl MoveResolverExt,
        session_id: SessionId,
        aggregator_enabled: bool,
    ) -> SessionExt<'r, '_> {
        self.move_vm
            .new_session(resolver, session_id, aggregator_enabled)
    }

    pub fn load_module<'r>(
        &self,
        module_id: &ModuleId,
        resolver: &'r impl MoveResolverExt,
    ) -> VMResult<Arc<CompiledModule>> {
        self.move_vm.load_module(module_id, resolver)
    }
}

/// Internal APIs for the VM, primarily used for testing.
#[derive(Clone, Copy)]
pub struct AptosVMInternals<'a>(&'a AptosVMImpl);

impl<'a> AptosVMInternals<'a> {
    pub fn new(internal: &'a AptosVMImpl) -> Self {
        Self(internal)
    }

    /// Returns the internal Move VM instance.
    pub fn move_vm(self) -> &'a MoveVmExt {
        &self.0.move_vm
    }

    /// Returns the internal gas schedule if it has been loaded, or an error if it hasn't.
    pub fn gas_params(
        self,
        log_context: &AdapterLogSchema,
    ) -> Result<&'a AptosGasParameters, VMStatus> {
        self.0.get_gas_parameters(log_context)
    }

    /// Returns the version of Move Runtime.
    pub fn version(self) -> Result<Version, VMStatus> {
        self.0.get_version()
    }
}

pub(crate) fn get_transaction_output<A: AccessPathCache>(
    ap_cache: &mut A,
    session: SessionExt,
    fee_statement: FeeStatement,
    status: ExecutionStatus,
    change_set_configs: &ChangeSetConfigs,
) -> Result<VMOutput, VMStatus> {
    let change_set = session.finish(ap_cache, change_set_configs)?;

    Ok(VMOutput::new(
        change_set,
        fee_statement,
        TransactionStatus::Keep(status),
    ))
}

#[test]
fn vm_thread_safe() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    use crate::AptosVM;

    assert_send::<AptosVM>();
    assert_sync::<AptosVM>();
    assert_send::<MoveVmExt>();
    assert_sync::<MoveVmExt>();
}
