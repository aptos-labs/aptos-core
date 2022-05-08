// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    access_path_cache::AccessPathCache,
    counters::*,
    data_cache::RemoteStorage,
    errors::{convert_epilogue_error, convert_prologue_error, expect_only_successful_execution},
    logging::AdapterLogSchema,
    move_vm_ext::{MoveResolverExt, MoveVmExt, SessionExt, SessionId},
    transaction_metadata::TransactionMetadata,
};
use aptos_crypto::HashValue;
use aptos_logger::prelude::*;
use aptos_state_view::StateView;
use aptos_types::{
    account_config,
    account_config::ChainSpecificAccountInfo,
    on_chain_config::{
        ConfigStorage, OnChainConfig, VMConfig, VMPublishingOption, Version, APTOS_VERSION_3,
    },
    transaction::{ExecutionStatus, TransactionOutput, TransactionStatus},
    vm_status::{StatusCode, VMStatus},
};
use fail::fail_point;
use move_deps::{
    move_binary_format::{
        errors::{Location, VMResult},
        CompiledModule,
    },
    move_core_types::{
        account_address::AccountAddress,
        gas_schedule::{CostTable, GasAlgebra, GasCarrier, GasUnits, InternalGasUnits},
        language_storage::ModuleId,
        move_resource::MoveStructType,
        resolver::ResourceResolver,
        value::{serialize_values, MoveValue},
    },
    move_vm_runtime::{logging::expect_no_verification_errors, session::Session},
    move_vm_types::gas_schedule::GasStatus,
};
use std::sync::Arc;

#[derive(Clone)]
/// A wrapper to make VMRuntime standalone and thread safe.
pub struct AptosVMImpl {
    move_vm: Arc<MoveVmExt>,
    on_chain_config: Option<VMConfig>,
    version: Option<Version>,
    publishing_option: Option<VMPublishingOption>,
    chain_account_info: Option<ChainSpecificAccountInfo>,
}

impl AptosVMImpl {
    #[allow(clippy::new_without_default)]
    pub fn new<S: StateView>(state: &S) -> Self {
        let inner = MoveVmExt::new()
            .expect("should be able to create Move VM; check if there are duplicated natives");
        let mut vm = Self {
            move_vm: Arc::new(inner),
            on_chain_config: None,
            version: None,
            publishing_option: None,
            chain_account_info: None,
        };
        vm.load_configs_impl(&RemoteStorage::new(state));
        vm.chain_account_info = Self::get_chain_specific_account_info(&RemoteStorage::new(state));
        vm
    }

    pub fn init_with_config(
        version: Version,
        on_chain_config: VMConfig,
        publishing_option: VMPublishingOption,
    ) -> Self {
        let inner = MoveVmExt::new()
            .expect("should be able to create Move VM; check if there are duplicated natives");
        Self {
            move_vm: Arc::new(inner),
            on_chain_config: Some(on_chain_config),
            version: Some(version),
            publishing_option: Some(publishing_option),
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
            .unwrap_or(&account_config::DPN_CHAIN_INFO)
    }

    pub(crate) fn publishing_option(
        &self,
        log_context: &AdapterLogSchema,
    ) -> Result<&VMPublishingOption, VMStatus> {
        self.publishing_option.as_ref().ok_or_else(|| {
            log_context.alert();
            error!(
                *log_context,
                "VM Startup Failed. PublishingOption Not Found"
            );
            VMStatus::Error(StatusCode::VM_STARTUP_FAILURE)
        })
    }

    fn load_configs_impl<S: ConfigStorage>(&mut self, data_cache: &S) {
        self.on_chain_config = VMConfig::fetch_config(data_cache);
        self.version = Version::fetch_config(data_cache);
        self.publishing_option = VMPublishingOption::fetch_config(data_cache);
    }

    // TODO: Move this to an on-chain config once those are a part of the core framework
    fn get_chain_specific_account_info<S: ResourceResolver>(
        remote_cache: &S,
    ) -> Option<ChainSpecificAccountInfo> {
        match remote_cache
            .get_resource(
                &account_config::aptos_root_address(),
                &account_config::ChainSpecificAccountInfo::struct_tag(),
            )
            .ok()?
        {
            Some(blob) => bcs::from_bytes::<ChainSpecificAccountInfo>(&blob).ok(),
            _ => None,
        }
    }

    pub fn get_gas_schedule(&self, log_context: &AdapterLogSchema) -> Result<&CostTable, VMStatus> {
        self.on_chain_config
            .as_ref()
            .map(|config| &config.gas_schedule)
            .ok_or_else(|| {
                log_context.alert();
                error!(*log_context, "VM Startup Failed. Gas Schedule Not Found");
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
        let gas_constants = &self.get_gas_schedule(log_context)?.gas_constants;
        let raw_bytes_len = txn_data.transaction_size;
        // The transaction is too large.
        if txn_data.transaction_size.get() > gas_constants.max_transaction_size_in_bytes {
            warn!(
                *log_context,
                "[VM] Transaction size too big {} (max {})",
                raw_bytes_len.get(),
                gas_constants.max_transaction_size_in_bytes,
            );
            return Err(VMStatus::Error(StatusCode::EXCEEDED_MAX_TRANSACTION_SIZE));
        }

        // Check is performed on `txn.raw_txn_bytes_len()` which is the same as
        // `raw_bytes_len`
        assume!(raw_bytes_len.get() <= gas_constants.max_transaction_size_in_bytes);

        // The submitted max gas units that the transaction can consume is greater than the
        // maximum number of gas units bound that we have set for any
        // transaction.
        if txn_data.max_gas_amount().get() > gas_constants.maximum_number_of_gas_units.get() {
            warn!(
                *log_context,
                "[VM] Gas unit error; max {}, submitted {}",
                gas_constants.maximum_number_of_gas_units.get(),
                txn_data.max_gas_amount().get(),
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

            if raw_bytes_len.get() > gas_constants.large_transaction_cutoff.get() {
                let excess = raw_bytes_len.sub(gas_constants.large_transaction_cutoff);
                min_transaction_fee.add(gas_constants.intrinsic_gas_per_byte.mul(excess))
            } else {
                min_transaction_fee.unitary_cast()
            }
        };
        let min_txn_fee = gas_constants.to_external_units(min_txn_fee);

        if txn_data.max_gas_amount().get() < min_txn_fee.get() {
            warn!(
                *log_context,
                "[VM] Gas unit error; min {}, submitted {}",
                min_txn_fee.get(),
                txn_data.max_gas_amount().get(),
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
            txn_data.gas_unit_price().get() < gas_constants.min_price_per_gas_unit.get();
        if below_min_bound {
            warn!(
                *log_context,
                "[VM] Gas unit error; min {}, submitted {}",
                gas_constants.min_price_per_gas_unit.get(),
                txn_data.gas_unit_price().get(),
            );
            return Err(VMStatus::Error(StatusCode::GAS_UNIT_PRICE_BELOW_MIN_BOUND));
        }

        // The submitted gas price is greater than the maximum gas unit price set by the VM.
        if txn_data.gas_unit_price().get() > gas_constants.max_price_per_gas_unit.get() {
            warn!(
                *log_context,
                "[VM] Gas unit error; min {}, submitted {}",
                gas_constants.max_price_per_gas_unit.get(),
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
        let txn_max_gas_units = txn_data.max_gas_amount().get();
        let txn_expiration_timestamp_secs = txn_data.expiration_timestamp_secs();
        let chain_id = txn_data.chain_id();
        let mut gas_status = GasStatus::new_unmetered();
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
                &mut gas_status,
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
        let txn_max_gas_units = txn_data.max_gas_amount().get();
        let txn_expiration_timestamp_secs = txn_data.expiration_timestamp_secs();
        let chain_id = txn_data.chain_id();
        let mut gas_status = GasStatus::new_unmetered();
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
                &mut gas_status,
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
        gas_status: &mut GasStatus,
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
        let txn_max_gas_units = txn_data.max_gas_amount().get();
        let gas_remaining = gas_status.remaining_gas().get();
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
                gas_status,
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
        gas_status: &mut GasStatus,
        txn_data: &TransactionMetadata,
        log_context: &AdapterLogSchema,
    ) -> Result<(), VMStatus> {
        let gas_currency = vec![];
        let chain_specific_info = self.chain_info();
        let txn_sequence_number = txn_data.sequence_number();
        let txn_gas_price = txn_data.gas_unit_price().get();
        let txn_max_gas_units = txn_data.max_gas_amount().get();
        let gas_remaining = gas_status.remaining_gas().get();
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
                gas_status,
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

        let mut gas_status = GasStatus::new_unmetered();
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
                &mut gas_status,
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
        let mut gas_status = GasStatus::new_unmetered();
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
                &mut gas_status,
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
    pub fn gas_schedule(self, log_context: &AdapterLogSchema) -> Result<&'a CostTable, VMStatus> {
        self.0.get_gas_schedule(log_context)
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

pub(crate) fn charge_global_write_gas_usage<R: MoveResolverExt>(
    gas_status: &mut GasStatus,
    session: &Session<R>,
    sender: &AccountAddress,
) -> Result<(), VMStatus> {
    let total_cost = session.num_mutated_accounts(sender)
        * gas_status
            .cost_table()
            .gas_constants
            .global_memory_per_byte_write_cost
            .mul(gas_status.cost_table().gas_constants.default_account_size)
            .get();
    gas_status
        .deduct_gas(InternalGasUnits::new(total_cost))
        .map_err(|p_err| p_err.finish(Location::Undefined).into_vm_status())
}

pub(crate) fn get_transaction_output<A: AccessPathCache, S: MoveResolverExt>(
    ap_cache: &mut A,
    session: SessionExt<S>,
    gas_left: GasUnits<GasCarrier>,
    txn_data: &TransactionMetadata,
    status: ExecutionStatus,
) -> Result<TransactionOutput, VMStatus> {
    let gas_used: u64 = txn_data.max_gas_amount().sub(gas_left).get();

    let session_out = session.finish().map_err(|e| e.into_vm_status())?;
    let (write_set, events) = session_out.into_change_set(ap_cache)?.into_inner();

    Ok(TransactionOutput::new(
        write_set,
        events,
        gas_used,
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
