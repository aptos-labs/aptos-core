// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    access_path_cache::AccessPathCache,
    counters::*,
    data_cache::StorageAdapter,
    errors::{convert_epilogue_error, convert_prologue_error, expect_only_successful_execution},
    logging::AdapterLogSchema,
    move_vm_ext::{MoveResolverExt, MoveVmExt, SessionExt, SessionId},
    transaction_metadata::TransactionMetadata,
};
use aptos_aggregator::transaction::TransactionOutputExt;
use aptos_gas::{
    AbstractValueSizeGasParameters, AptosGasParameters, FromOnChainGasSchedule, Gas,
    NativeGasParameters, StorageGasParameters,
};
use aptos_logger::prelude::*;
use aptos_state_view::StateView;
use aptos_types::on_chain_config::{FeatureFlag, Features};
use aptos_types::transaction::AbortInfo;
use aptos_types::{
    account_config::{TransactionValidation, APTOS_TRANSACTION_VALIDATION, CORE_CODE_ADDRESS},
    on_chain_config::{
        ApprovedExecutionHashes, GasSchedule, GasScheduleV2, OnChainConfig, StorageGasSchedule,
        Version,
    },
    transaction::{ExecutionStatus, TransactionOutput, TransactionStatus},
    vm_status::{StatusCode, VMStatus},
};
use dashmap::DashMap;
use fail::fail_point;
use framework::{RuntimeModuleMetadata, APTOS_METADATA_KEY};
use move_deps::{
    move_binary_format::{errors::VMResult, CompiledModule},
    move_core_types::{
        language_storage::ModuleId,
        move_resource::MoveStructType,
        resolver::ResourceResolver,
        value::{serialize_values, MoveValue},
    },
    move_vm_runtime::logging::expect_no_verification_errors,
    move_vm_types::gas::UnmeteredGasMeter,
};
use std::sync::Arc;

#[derive(Clone)]
/// A wrapper to make VMRuntime standalone and thread safe.
pub struct AptosVMImpl {
    move_vm: Arc<MoveVmExt>,
    gas_feature_version: u64,
    gas_params: Option<AptosGasParameters>,
    storage_gas_params: Option<StorageGasParameters>,
    version: Option<Version>,
    transaction_validation: Option<TransactionValidation>,
    metadata_cache: DashMap<ModuleId, Option<RuntimeModuleMetadata>>,
}

impl AptosVMImpl {
    #[allow(clippy::new_without_default)]
    pub fn new<S: StateView>(state: &S) -> Self {
        let storage = StorageAdapter::new(state);

        // Get the gas parameters
        let (gas_params, gas_feature_version): (Option<AptosGasParameters>, u64) =
            match GasScheduleV2::fetch_config(&storage) {
                Some(gas_schedule) => {
                    let feature_version = gas_schedule.feature_version;
                    let map = gas_schedule.to_btree_map();
                    (
                        AptosGasParameters::from_on_chain_gas_schedule(&map),
                        feature_version,
                    )
                }
                None => match GasSchedule::fetch_config(&storage) {
                    Some(gas_schedule) => {
                        let map = gas_schedule.to_btree_map();
                        (AptosGasParameters::from_on_chain_gas_schedule(&map), 0)
                    }
                    None => (None, 0),
                },
            };

        let storage_gas_params = match gas_feature_version {
            0 => None,
            _ => StorageGasSchedule::fetch_config(&storage)
                .map(|storage_gas_schedule| storage_gas_schedule.into()),
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
        let inner = MoveVmExt::new(
            native_gas_params,
            abs_val_size_gas_params,
            features.is_enabled(FeatureFlag::TREAT_FRIEND_AS_PRIVATE),
        )
        .expect("should be able to create Move VM; check if there are duplicated natives");

        let mut vm = Self {
            move_vm: Arc::new(inner),
            gas_feature_version,
            gas_params,
            storage_gas_params,
            version: None,
            transaction_validation: None,
            metadata_cache: Default::default(),
        };
        vm.version = Version::fetch_config(&storage);
        vm.transaction_validation = Self::get_transaction_validation(&StorageAdapter::new(state));
        vm
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
    fn get_transaction_validation<S: ResourceResolver>(
        remote_cache: &S,
    ) -> Option<TransactionValidation> {
        match remote_cache
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
            log_context.alert();
            error!(*log_context, "VM Startup Failed. Gas Parameters Not Found");
            VMStatus::Error(StatusCode::VM_STARTUP_FAILURE)
        })
    }

    pub fn get_storage_gas_parameters(
        &self,
        log_context: &AdapterLogSchema,
    ) -> Result<Option<&StorageGasParameters>, VMStatus> {
        match self.gas_feature_version {
            0 => Ok(None),
            _ => Ok(Some(self.storage_gas_params.as_ref().ok_or_else(|| {
                log_context.alert();
                error!(
                    *log_context,
                    "VM Startup Failed. Storage Gas Parameters Not Found"
                );
                VMStatus::Error(StatusCode::VM_STARTUP_FAILURE)
            })?)),
        }
    }

    pub fn get_gas_feature_version(&self) -> u64 {
        self.gas_feature_version
    }

    pub fn get_version(&self) -> Result<Version, VMStatus> {
        self.version.clone().ok_or_else(|| {
            CRITICAL_ERRORS.inc();
            error!("VM Startup Failed. Version Not Found");
            VMStatus::Error(StatusCode::VM_STARTUP_FAILURE)
        })
    }

    pub fn check_gas<S: MoveResolverExt>(
        &self,
        storage: &S,
        txn_data: &TransactionMetadata,
        log_context: &AdapterLogSchema,
    ) -> Result<(), VMStatus> {
        let txn_gas_params = &self.get_gas_parameters(log_context)?.txn;
        let raw_bytes_len = txn_data.transaction_size;
        // The transaction is too large.
        if txn_data.transaction_size > txn_gas_params.max_transaction_size_in_bytes {
            let data =
                storage.get_resource(&CORE_CODE_ADDRESS, &ApprovedExecutionHashes::struct_tag());

            let valid = if let Ok(Some(data)) = data {
                let approved_execution_hashes =
                    bcs::from_bytes::<ApprovedExecutionHashes>(&data).ok();
                approved_execution_hashes
                    .map(|aeh| {
                        aeh.entries
                            .into_iter()
                            .any(|(_, hash)| hash == txn_data.script_hash)
                    })
                    .unwrap_or(false)
            } else {
                false
            };

            if !valid {
                warn!(
                    *log_context,
                    "[VM] Transaction size too big {} (max {})",
                    raw_bytes_len,
                    txn_gas_params.max_transaction_size_in_bytes,
                );
                return Err(VMStatus::Error(StatusCode::EXCEEDED_MAX_TRANSACTION_SIZE));
            }
        }

        // The submitted max gas units that the transaction can consume is greater than the
        // maximum number of gas units bound that we have set for any
        // transaction.
        if txn_data.max_gas_amount() > txn_gas_params.maximum_number_of_gas_units {
            warn!(
                *log_context,
                "[VM] Gas unit error; max {}, submitted {}",
                txn_gas_params.maximum_number_of_gas_units,
                txn_data.max_gas_amount(),
            );
            return Err(VMStatus::Error(
                StatusCode::MAX_GAS_UNITS_EXCEEDS_MAX_GAS_UNITS_BOUND,
            ));
        }

        // The submitted transactions max gas units needs to be at least enough to cover the
        // intrinsic cost of the transaction as calculated against the size of the
        // underlying `RawTransaction`
        let intrinsic_gas: Gas = txn_gas_params
            .calculate_intrinsic_gas(raw_bytes_len)
            .to_unit_round_up_with_params(txn_gas_params);

        if txn_data.max_gas_amount() < intrinsic_gas {
            warn!(
                *log_context,
                "[VM] Gas unit error; min {}, submitted {}",
                intrinsic_gas,
                txn_data.max_gas_amount(),
            );
            return Err(VMStatus::Error(
                StatusCode::MAX_GAS_UNITS_BELOW_MIN_TRANSACTION_GAS_UNITS,
            ));
        }

        // The submitted gas price is less than the minimum gas unit price set by the VM.
        // NB: MIN_PRICE_PER_GAS_UNIT may equal zero, but need not in the future. Hence why
        // we turn off the clippy warning.
        #[allow(clippy::absurd_extreme_comparisons)]
        let below_min_bound = txn_data.gas_unit_price() < txn_gas_params.min_price_per_gas_unit;
        if below_min_bound {
            warn!(
                *log_context,
                "[VM] Gas unit error; min {}, submitted {}",
                txn_gas_params.min_price_per_gas_unit,
                txn_data.gas_unit_price(),
            );
            return Err(VMStatus::Error(StatusCode::GAS_UNIT_PRICE_BELOW_MIN_BOUND));
        }

        // The submitted gas price is greater than the maximum gas unit price set by the VM.
        if txn_data.gas_unit_price() > txn_gas_params.max_price_per_gas_unit {
            warn!(
                *log_context,
                "[VM] Gas unit error; min {}, submitted {}",
                txn_gas_params.max_price_per_gas_unit,
                txn_data.gas_unit_price(),
            );
            return Err(VMStatus::Error(StatusCode::GAS_UNIT_PRICE_ABOVE_MAX_BOUND));
        }
        Ok(())
    }

    /// Run the prologue of a transaction by calling into either `SCRIPT_PROLOGUE_NAME` function
    /// or `MULTI_AGENT_SCRIPT_PROLOGUE_NAME` function stored in the `ACCOUNT_MODULE` on chain.
    pub(crate) fn run_script_prologue<S: MoveResolverExt>(
        &self,
        session: &mut SessionExt<S>,
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
    pub(crate) fn run_module_prologue<S: MoveResolverExt>(
        &self,
        session: &mut SessionExt<S>,
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

    /// Run the epilogue of a transaction by calling into `EPILOGUE_NAME` function stored
    /// in the `ACCOUNT_MODULE` on chain.
    pub(crate) fn run_success_epilogue<S: MoveResolverExt>(
        &self,
        session: &mut SessionExt<S>,
        gas_remaining: Gas,
        txn_data: &TransactionMetadata,
        log_context: &AdapterLogSchema,
    ) -> Result<(), VMStatus> {
        fail_point!("move_adapter::run_success_epilogue", |_| {
            Err(VMStatus::Error(
                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
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
    pub(crate) fn run_failure_epilogue<S: MoveResolverExt>(
        &self,
        session: &mut SessionExt<S>,
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
        let entry = self
            .metadata_cache
            .entry(module.clone())
            .or_insert_with(|| {
                if let Some(m) = self
                    .move_vm
                    .get_module_metadata(module.clone(), &APTOS_METADATA_KEY)
                {
                    bcs::from_bytes::<RuntimeModuleMetadata>(&m.value).ok()
                } else {
                    None
                }
            });
        if let Some(m) = entry.value() {
            m.extract_abort_info(abort_code)
        } else {
            None
        }
    }

    pub fn new_session<'r, R: MoveResolverExt>(
        &self,
        r: &'r R,
        session_id: SessionId,
    ) -> SessionExt<'r, '_, R> {
        self.move_vm.new_session(r, session_id)
    }

    pub fn load_module<'r, R: MoveResolverExt>(
        &self,
        module_id: &ModuleId,
        remote: &'r R,
    ) -> VMResult<Arc<CompiledModule>> {
        self.move_vm.load_module(module_id, remote)
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

    /// Executes the given code within the context of a transaction.
    ///
    /// The `TransactionDataCache` can be used as a `ChainState`.
    ///
    /// If you don't care about the transaction metadata, use `TransactionMetadata::default()`.
    pub fn with_txn_data_cache<T, S: StateView>(
        self,
        state_view: &S,
        f: impl for<'txn, 'r> FnOnce(SessionExt<'txn, 'r, StorageAdapter<S>>) -> T,
        session_id: SessionId,
    ) -> T {
        let remote_storage = StorageAdapter::new(state_view);
        let session = self.move_vm().new_session(&remote_storage, session_id);
        f(session)
    }
}

pub(crate) fn get_transaction_output<A: AccessPathCache, S: MoveResolverExt>(
    ap_cache: &mut A,
    session: SessionExt<S>,
    gas_left: Gas,
    txn_data: &TransactionMetadata,
    status: ExecutionStatus,
) -> Result<TransactionOutputExt, VMStatus> {
    let gas_used = txn_data
        .max_gas_amount()
        .checked_sub(gas_left)
        .expect("Balance should always be less than or equal to max gas amount");

    let session_out = session.finish().map_err(|e| e.into_vm_status())?;
    let change_set_ext = session_out.into_change_set(ap_cache)?;
    let (delta_change_set, change_set) = change_set_ext.into_inner();
    let (write_set, events) = change_set.into_inner();

    let txn_output = TransactionOutput::new(
        write_set,
        events,
        gas_used.into(),
        TransactionStatus::Keep(status),
    );

    Ok(TransactionOutputExt::new(delta_change_set, txn_output))
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
