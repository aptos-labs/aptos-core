// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    access_path_cache::AccessPathCache,
    counters::*,
    data_cache::RemoteStorage,
    delta_ext::TransactionOutputExt,
    errors::{convert_epilogue_error, convert_prologue_error, expect_only_successful_execution},
    logging::AdapterLogSchema,
    move_vm_ext::{MoveResolverExt, MoveVmExt, SessionExt, SessionId},
    transaction_metadata::TransactionMetadata,
};
use aptos_crypto::HashValue;
use aptos_gas::{AptosGasParameters, FromOnChainGasSchedule, NativeGasParameters};
use aptos_logger::prelude::*;
use aptos_state_view::StateView;
use aptos_types::{
    account_config::{ChainSpecificAccountInfo, APTOS_CHAIN_INFO, CORE_CODE_ADDRESS},
    on_chain_config::{GasSchedule, OnChainConfig, Version, APTOS_VERSION_3},
    transaction::{ExecutionStatus, TransactionOutput, TransactionStatus},
    vm_status::{StatusCode, VMStatus},
};
use fail::fail_point;
use move_deps::{
    move_binary_format::{errors::VMResult, CompiledModule},
    move_core_types::{
        gas_schedule::GasAlgebra,
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
    gas_params: Option<AptosGasParameters>,
    version: Option<Version>,
    chain_account_info: Option<ChainSpecificAccountInfo>,
}

impl AptosVMImpl {
    #[allow(clippy::new_without_default)]
    pub fn new<S: StateView>(state: &S) -> Self {
        let storage = RemoteStorage::new(state);

        // TODO(Gas): this should not panic
        let gas_params = GasSchedule::fetch_config(&storage).and_then(|gas_schedule| {
            let gas_schedule = gas_schedule.to_btree_map();
            AptosGasParameters::from_on_chain_gas_schedule(&gas_schedule)
        });

        // TODO(Gas): this doesn't look right.
        let native_gas_params = match &gas_params {
            Some(gas_params) => gas_params.natives.clone(),
            None => NativeGasParameters::zeros(),
        };

        let inner = MoveVmExt::new(native_gas_params)
            .expect("should be able to create Move VM; check if there are duplicated natives");

        let mut vm = Self {
            move_vm: Arc::new(inner),
            gas_params,
            version: None,
            chain_account_info: None,
        };
        vm.version = Version::fetch_config(&storage);
        vm.chain_account_info = Self::get_chain_specific_account_info(&RemoteStorage::new(state));
        vm
    }

    pub fn init_with_config(version: Version, gas_schedule: GasSchedule) -> Self {
        // TODO(Gas): this should not panic
        let gas_params =
            AptosGasParameters::from_on_chain_gas_schedule(&gas_schedule.to_btree_map())
                .expect("failed to get gas parameters");

        let inner = MoveVmExt::new(gas_params.natives.clone())
            .expect("should be able to create Move VM; check if there are duplicated natives");

        Self {
            move_vm: Arc::new(inner),
            gas_params: Some(gas_params),
            version: Some(version),
            chain_account_info: None,
        }
    }

    /// Provides access to some internal APIs of the VM.
    pub fn internals(&self) -> AptosVMInternals {
        AptosVMInternals(self)
    }

    pub(crate) fn chain_info(&self) -> &ChainSpecificAccountInfo {
        self.chain_account_info
            .as_ref()
            .unwrap_or(&APTOS_CHAIN_INFO)
    }

    // TODO: Move this to an on-chain config once those are a part of the core framework
    fn get_chain_specific_account_info<S: ResourceResolver>(
        remote_cache: &S,
    ) -> Option<ChainSpecificAccountInfo> {
        match remote_cache
            .get_resource(&CORE_CODE_ADDRESS, &ChainSpecificAccountInfo::struct_tag())
            .ok()?
        {
            Some(blob) => bcs::from_bytes::<ChainSpecificAccountInfo>(&blob).ok(),
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

    pub fn get_version(&self) -> Result<Version, VMStatus> {
        self.version.clone().ok_or_else(|| {
            CRITICAL_ERRORS.inc();
            error!("VM Startup Failed. Version Not Found");
            VMStatus::Error(StatusCode::VM_STARTUP_FAILURE)
        })
    }

    pub fn check_gas(
        &self,
        txn_data: &TransactionMetadata,
        log_context: &AdapterLogSchema,
    ) -> Result<(), VMStatus> {
        let gas_constants = &self.get_gas_parameters(log_context)?.txn;
        let raw_bytes_len = txn_data.transaction_size;
        // The transaction is too large.
        if txn_data.transaction_size > gas_constants.max_transaction_size_in_bytes {
            warn!(
                *log_context,
                "[VM] Transaction size too big {} (max {})",
                raw_bytes_len,
                gas_constants.max_transaction_size_in_bytes,
            );
            return Err(VMStatus::Error(StatusCode::EXCEEDED_MAX_TRANSACTION_SIZE));
        }

        // Check is performed on `txn.raw_txn_bytes_len()` which is the same as
        // `raw_bytes_len`
        assume!(raw_bytes_len <= gas_constants.max_transaction_size_in_bytes);

        // The submitted max gas units that the transaction can consume is greater than the
        // maximum number of gas units bound that we have set for any
        // transaction.
        if txn_data.max_gas_amount() > gas_constants.maximum_number_of_gas_units {
            warn!(
                *log_context,
                "[VM] Gas unit error; max {}, submitted {}",
                gas_constants.maximum_number_of_gas_units,
                txn_data.max_gas_amount(),
            );
            return Err(VMStatus::Error(
                StatusCode::MAX_GAS_UNITS_EXCEEDS_MAX_GAS_UNITS_BOUND,
            ));
        }

        // The submitted transactions max gas units needs to be at least enough to cover the
        // intrinsic cost of the transaction as calculated against the size of the
        // underlying `RawTransaction`
        let min_txn_fee = {
            let min_transaction_fee = gas_constants.min_transaction_gas_units;

            if raw_bytes_len > gas_constants.large_transaction_cutoff {
                let excess = raw_bytes_len - gas_constants.large_transaction_cutoff;
                min_transaction_fee + gas_constants.intrinsic_gas_per_byte * excess
            } else {
                min_transaction_fee
            }
        };
        let min_txn_fee = gas_constants.to_external_units(min_txn_fee);

        if txn_data.max_gas_amount() < min_txn_fee {
            warn!(
                *log_context,
                "[VM] Gas unit error; min {}, submitted {}",
                min_txn_fee,
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
        let below_min_bound =
            txn_data.gas_unit_price().get() < gas_constants.min_price_per_gas_unit;
        if below_min_bound {
            warn!(
                *log_context,
                "[VM] Gas unit error; min {}, submitted {}",
                gas_constants.min_price_per_gas_unit,
                txn_data.gas_unit_price().get(),
            );
            return Err(VMStatus::Error(StatusCode::GAS_UNIT_PRICE_BELOW_MIN_BOUND));
        }

        // The submitted gas price is greater than the maximum gas unit price set by the VM.
        if txn_data.gas_unit_price().get() > gas_constants.max_price_per_gas_unit {
            warn!(
                *log_context,
                "[VM] Gas unit error; min {}, submitted {}",
                gas_constants.max_price_per_gas_unit,
                txn_data.gas_unit_price().get(),
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
        let chain_specific_info = self.chain_info();
        let gas_currency = vec![];
        let txn_sequence_number = txn_data.sequence_number();
        let txn_public_key = txn_data.authentication_key_preimage().to_vec();
        let txn_gas_price = txn_data.gas_unit_price().get();
        let txn_max_gas_units = txn_data.max_gas_amount();
        let txn_expiration_timestamp_secs = txn_data.expiration_timestamp_secs();
        let chain_id = txn_data.chain_id();
        let mut gas_meter = UnmeteredGasMeter;
        let secondary_public_key_hashes: Vec<MoveValue> = txn_data
            .secondary_authentication_key_preimages
            .iter()
            .map(|preimage| MoveValue::vector_u8(HashValue::sha3_256_of(preimage).to_vec()))
            .collect();
        let args = if self.get_version()? >= APTOS_VERSION_3 && txn_data.is_multi_agent() {
            vec![
                MoveValue::Signer(txn_data.sender),
                MoveValue::U64(txn_sequence_number),
                MoveValue::vector_u8(txn_public_key),
                MoveValue::vector_address(txn_data.secondary_signers()),
                MoveValue::Vector(secondary_public_key_hashes),
                MoveValue::U64(txn_gas_price),
                MoveValue::U64(txn_max_gas_units),
                MoveValue::U64(txn_expiration_timestamp_secs),
                MoveValue::U8(chain_id.id()),
            ]
        } else {
            vec![
                MoveValue::Signer(txn_data.sender),
                MoveValue::U64(txn_sequence_number),
                MoveValue::vector_u8(txn_public_key),
                MoveValue::U64(txn_gas_price),
                MoveValue::U64(txn_max_gas_units),
                MoveValue::U64(txn_expiration_timestamp_secs),
                MoveValue::U8(chain_id.id()),
                MoveValue::vector_u8(txn_data.script_hash.clone()),
            ]
        };
        let prologue_function_name =
            if self.get_version()? >= APTOS_VERSION_3 && txn_data.is_multi_agent() {
                &chain_specific_info.multi_agent_prologue_name
            } else {
                &chain_specific_info.script_prologue_name
            };
        session
            .execute_function_bypass_visibility(
                &chain_specific_info.module_id(),
                prologue_function_name,
                gas_currency,
                serialize_values(&args),
                &mut gas_meter,
            )
            .map(|_return_vals| ())
            .map_err(expect_no_verification_errors)
            .or_else(|err| convert_prologue_error(chain_specific_info, err, log_context))
    }

    /// Run the prologue of a transaction by calling into `MODULE_PROLOGUE_NAME` function stored
    /// in the `ACCOUNT_MODULE` on chain.
    pub(crate) fn run_module_prologue<S: MoveResolverExt>(
        &self,
        session: &mut SessionExt<S>,
        txn_data: &TransactionMetadata,
        log_context: &AdapterLogSchema,
    ) -> Result<(), VMStatus> {
        let chain_specific_info = self.chain_info();
        let gas_currency = vec![];
        let txn_sequence_number = txn_data.sequence_number();
        let txn_public_key = txn_data.authentication_key_preimage().to_vec();
        let txn_gas_price = txn_data.gas_unit_price().get();
        let txn_max_gas_units = txn_data.max_gas_amount();
        let txn_expiration_timestamp_secs = txn_data.expiration_timestamp_secs();
        let chain_id = txn_data.chain_id();
        let mut gas_meter = UnmeteredGasMeter;
        session
            .execute_function_bypass_visibility(
                &chain_specific_info.module_id(),
                &chain_specific_info.module_prologue_name,
                gas_currency,
                serialize_values(&vec![
                    MoveValue::Signer(txn_data.sender),
                    MoveValue::U64(txn_sequence_number),
                    MoveValue::vector_u8(txn_public_key),
                    MoveValue::U64(txn_gas_price),
                    MoveValue::U64(txn_max_gas_units),
                    MoveValue::U64(txn_expiration_timestamp_secs),
                    MoveValue::U8(chain_id.id()),
                ]),
                &mut gas_meter,
            )
            .map(|_return_vals| ())
            .map_err(expect_no_verification_errors)
            .or_else(|err| convert_prologue_error(chain_specific_info, err, log_context))
    }

    /// Run the epilogue of a transaction by calling into `EPILOGUE_NAME` function stored
    /// in the `ACCOUNT_MODULE` on chain.
    pub(crate) fn run_success_epilogue<S: MoveResolverExt>(
        &self,
        session: &mut SessionExt<S>,
        gas_remaining: u64,
        txn_data: &TransactionMetadata,
        log_context: &AdapterLogSchema,
    ) -> Result<(), VMStatus> {
        fail_point!("move_adapter::run_success_epilogue", |_| {
            Err(VMStatus::Error(
                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
            ))
        });

        let gas_currency = vec![];
        let chain_specific_info = self.chain_info();
        let txn_sequence_number = txn_data.sequence_number();
        let txn_gas_price = txn_data.gas_unit_price().get();
        let txn_max_gas_units = txn_data.max_gas_amount();
        session
            .execute_function_bypass_visibility(
                &chain_specific_info.module_id(),
                &chain_specific_info.user_epilogue_name,
                gas_currency,
                serialize_values(&vec![
                    MoveValue::Signer(txn_data.sender),
                    MoveValue::U64(txn_sequence_number),
                    MoveValue::U64(txn_gas_price),
                    MoveValue::U64(txn_max_gas_units),
                    MoveValue::U64(gas_remaining),
                ]),
                &mut UnmeteredGasMeter,
            )
            .map(|_return_vals| ())
            .map_err(expect_no_verification_errors)
            .or_else(|err| convert_epilogue_error(chain_specific_info, err, log_context))
    }

    /// Run the failure epilogue of a transaction by calling into `USER_EPILOGUE_NAME` function
    /// stored in the `ACCOUNT_MODULE` on chain.
    pub(crate) fn run_failure_epilogue<S: MoveResolverExt>(
        &self,
        session: &mut SessionExt<S>,
        gas_remaining: u64,
        txn_data: &TransactionMetadata,
        log_context: &AdapterLogSchema,
    ) -> Result<(), VMStatus> {
        let gas_currency = vec![];
        let chain_specific_info = self.chain_info();
        let txn_sequence_number = txn_data.sequence_number();
        let txn_gas_price = txn_data.gas_unit_price().get();
        let txn_max_gas_units = txn_data.max_gas_amount();
        session
            .execute_function_bypass_visibility(
                &chain_specific_info.module_id(),
                &chain_specific_info.user_epilogue_name,
                gas_currency,
                serialize_values(&vec![
                    MoveValue::Signer(txn_data.sender),
                    MoveValue::U64(txn_sequence_number),
                    MoveValue::U64(txn_gas_price),
                    MoveValue::U64(txn_max_gas_units),
                    MoveValue::U64(gas_remaining),
                ]),
                &mut UnmeteredGasMeter,
            )
            .map(|_return_vals| ())
            .map_err(expect_no_verification_errors)
            .or_else(|e| {
                expect_only_successful_execution(
                    e,
                    chain_specific_info.user_epilogue_name.as_str(),
                    log_context,
                )
            })
    }

    /// Run the prologue of a transaction by calling into `PROLOGUE_NAME` function stored
    /// in the `WRITESET_MODULE` on chain.
    pub(crate) fn run_writeset_prologue<S: MoveResolverExt>(
        &self,
        session: &mut SessionExt<S>,
        txn_data: &TransactionMetadata,
        log_context: &AdapterLogSchema,
    ) -> Result<(), VMStatus> {
        let txn_sequence_number = txn_data.sequence_number();
        let txn_public_key = txn_data.authentication_key_preimage().to_vec();
        let txn_expiration_timestamp_secs = txn_data.expiration_timestamp_secs();
        let chain_id = txn_data.chain_id();
        let chain_specific_info = self.chain_info();

        let mut gas_meter = UnmeteredGasMeter;
        session
            .execute_function_bypass_visibility(
                &chain_specific_info.module_id(),
                &chain_specific_info.writeset_prologue_name,
                vec![],
                serialize_values(&vec![
                    MoveValue::Signer(txn_data.sender),
                    MoveValue::U64(txn_sequence_number),
                    MoveValue::vector_u8(txn_public_key),
                    MoveValue::U64(txn_expiration_timestamp_secs),
                    MoveValue::U8(chain_id.id()),
                ]),
                &mut gas_meter,
            )
            .map(|_return_vals| ())
            .map_err(expect_no_verification_errors)
            .or_else(|err| convert_prologue_error(chain_specific_info, err, log_context))
    }

    /// Run the epilogue of a transaction by calling into `WRITESET_EPILOGUE_NAME` function stored
    /// in the `WRITESET_MODULE` on chain.
    pub(crate) fn run_writeset_epilogue<S: MoveResolverExt>(
        &self,
        session: &mut SessionExt<S>,
        txn_data: &TransactionMetadata,
        should_trigger_reconfiguration: bool,
        log_context: &AdapterLogSchema,
    ) -> Result<(), VMStatus> {
        let mut gas_meter = UnmeteredGasMeter;
        let chain_specific_info = self.chain_info();
        session
            .execute_function_bypass_visibility(
                &chain_specific_info.module_id(),
                &chain_specific_info.writeset_epilogue_name,
                vec![],
                serialize_values(&vec![
                    MoveValue::Signer(txn_data.sender),
                    MoveValue::U64(txn_data.sequence_number),
                    MoveValue::Bool(should_trigger_reconfiguration),
                ]),
                &mut gas_meter,
            )
            .map(|_return_vals| ())
            .map_err(expect_no_verification_errors)
            .or_else(|e| {
                expect_only_successful_execution(
                    e,
                    chain_specific_info.writeset_epilogue_name.as_str(),
                    log_context,
                )
            })
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
        f: impl for<'txn, 'r> FnOnce(SessionExt<'txn, 'r, RemoteStorage<S>>) -> T,
        session_id: SessionId,
    ) -> T {
        let remote_storage = RemoteStorage::new(state_view);
        let session = self.move_vm().new_session(&remote_storage, session_id);
        f(session)
    }
}

pub(crate) fn get_transaction_output<A: AccessPathCache, S: MoveResolverExt>(
    ap_cache: &mut A,
    session: SessionExt<S>,
    gas_left: u64,
    txn_data: &TransactionMetadata,
    status: ExecutionStatus,
) -> Result<TransactionOutputExt, VMStatus> {
    let gas_used: u64 = txn_data.max_gas_amount() - gas_left;

    let session_out = session.finish().map_err(|e| e.into_vm_status())?;
    let (delta_change_set, change_set) = session_out.into_change_set_ext(ap_cache)?.into_inner();
    let (write_set, events) = change_set.into_inner();

    let txn_output =
        TransactionOutput::new(write_set, events, gas_used, TransactionStatus::Keep(status));

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
