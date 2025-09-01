// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    aptos_vm::SerializedSigners,
    counters::{SYSTEM_TRANSACTIONS_EXECUTED, TXN_GAS_USAGE},
    errors::expect_only_successful_execution,
    gas::make_prod_gas_meter,
    move_vm_ext::{resource_state_key, AptosMoveResolver, SessionId},
    system_module_names::{ACCOUNT_MODULE, CREATE_ACCOUNT_IF_DOES_NOT_EXIST},
    transaction_metadata::TransactionMetadata,
    v2::{
        data_cache::{TransactionDataCache, TransactionDataCacheAdapter},
        loader::AptosLoader,
    },
    verifier::{event_validation, script_validation},
};
use aptos_framework::natives::{
    aggregator_natives::NativeAggregatorContext, event::NativeEventContext,
    randomness::RandomnessContext,
};
use aptos_gas_algebra::{Fee, Gas};
use aptos_gas_meter::AptosGasMeter;
use aptos_gas_schedule::{
    gas_feature_versions::{RELEASE_V1_10, RELEASE_V1_27},
    AptosGasParameters, VMGasParameters,
};
use aptos_table_natives::NativeTableContext;
use aptos_types::{
    account_config::AccountResource,
    chain_id::ChainId,
    fee_statement::FeeStatement,
    on_chain_config::{FeatureFlag, Features},
    state_store::state_value::StateValueMetadata,
    transaction::{
        EntryFunction, ExecutionStatus, ReplayProtector, Script, SignedTransaction,
        TransactionExecutableRef, TransactionExtraConfig, TransactionStatus,
    },
    vm::module_metadata::{get_metadata, get_randomness_annotation_for_entry_function},
};
use aptos_vm_logging::log_schema::AdapterLogSchema;
use aptos_vm_types::{
    change_set::VMChangeSet,
    module_write_set::ModuleWriteSet,
    output::VMOutput,
    resolver::{BlockSynchronizationKillSwitch, NoopBlockSynchronizationKillSwitch},
    storage::{change_set_configs::ChangeSetSizeTracker, StorageGasParameters},
};
use move_binary_format::errors::{Location, PartialVMError, VMResult};
use move_core_types::{
    identifier::IdentStr,
    language_storage::{ModuleId, TypeTag},
    move_resource::MoveStructType,
    value::{serialize_values, MoveValue},
    vm_status::{AbortLocation, StatusCode, VMStatus},
};
use move_vm_metrics::{Timer, VM_TIMER};
use move_vm_runtime::{
    config::VMConfig,
    module_traversal::TraversalContext,
    move_vm::{MoveVM, SerializedReturnValues},
    native_extensions::NativeContextExtensions,
    FunctionValueExtensionAdapter, LegacyLoaderConfig, LoadedFunction, Loader, RuntimeEnvironment,
    ScriptLoader, WithRuntimeEnvironment,
};
use move_vm_types::gas::{GasMeter, UnmeteredGasMeter};

pub trait AptosSession {
    type CodeView: Loader;

    fn execute_function_bypass_visibility(
        &mut self,
        module_id: &ModuleId,
        function_name: &IdentStr,
        ty_args: Vec<TypeTag>,
        args: Vec<Vec<u8>>,
        gas_meter: &mut impl GasMeter,
    ) -> VMResult<SerializedReturnValues>;

    fn execute_loaded_function(
        &mut self,
        func: LoadedFunction,
        args: Vec<Vec<u8>>,
        gas_meter: &mut impl GasMeter,
    ) -> VMResult<SerializedReturnValues>;

    fn code_view(&self) -> &Self::CodeView;

    fn runtime_environment(&self) -> &RuntimeEnvironment {
        self.code_view().runtime_environment()
    }
}

pub struct Session<'a, DataView, CodeLoader> {
    /// Base view of data before this session started. Represents an original view before a
    /// transaction is executed.
    pub(crate) data_view: &'a DataView,
    /// Code loader, based on the view of the state before this session started.
    pub(crate) loader: &'a CodeLoader,
    /// Stores modules visited during execution.
    pub(crate) traversal_context: &'a mut TraversalContext<'a>,
    /// Scratchpad for changes to Move resources.
    pub(crate) data_cache: TransactionDataCache,
    /// Extensions for the Move VM. May be mutated by the session.
    pub(crate) extensions: NativeContextExtensions<'a>,
    /// Gas feature version for the current environment.
    pub(crate) gas_feature_version: u64,
    /// Features for the current environment.
    pub(crate) features: &'a Features,
    pub(crate) chain_id: ChainId,
    /// Gas parameters for the current environment.
    pub(crate) gas_params: &'a AptosGasParameters,
    /// Storage gas parameters for the current environment.
    pub(crate) storage_gas_params: &'a StorageGasParameters,
    pub(crate) vm_config: &'a VMConfig,
    pub(crate) new_slot_metadata: Option<StateValueMetadata>,

    /// Context for logs.
    pub(crate) log_context: &'a AdapterLogSchema,
}

impl<'a, DataView, CodeLoader> AptosSession for Session<'a, DataView, CodeLoader>
where
    DataView: AptosMoveResolver,
    CodeLoader: Loader,
{
    type CodeView = CodeLoader;

    fn execute_function_bypass_visibility(
        &mut self,
        module_id: &ModuleId,
        function_name: &IdentStr,
        ty_args: Vec<TypeTag>,
        args: Vec<Vec<u8>>,
        gas_meter: &mut impl GasMeter,
    ) -> VMResult<SerializedReturnValues> {
        let func = self.loader.load_instantiated_function(
            // Only for eager loading.
            &LegacyLoaderConfig::unmetered(),
            gas_meter,
            self.traversal_context,
            module_id,
            function_name,
            &ty_args,
        )?;
        self.execute_loaded_function(func, args, gas_meter)
    }

    fn execute_loaded_function(
        &mut self,
        func: LoadedFunction,
        args: Vec<Vec<u8>>,
        gas_meter: &mut impl GasMeter,
    ) -> VMResult<SerializedReturnValues> {
        MoveVM::execute_loaded_function(
            func,
            args,
            &mut TransactionDataCacheAdapter::new(
                &mut self.data_cache,
                self.data_view,
                self.loader,
            ),
            gas_meter,
            self.traversal_context,
            &mut self.extensions,
            self.loader,
        )
    }

    fn code_view(&self) -> &Self::CodeView {
        self.loader
    }
}

impl<'a, DataView, CodeLoader> Session<'a, DataView, CodeLoader>
where
    DataView: AptosMoveResolver,
    CodeLoader: Loader,
{
    pub(crate) fn execute_unmetered_system_function(
        &mut self,
        module_id: &ModuleId,
        function_name: &IdentStr,
        args: Vec<Vec<u8>>,
    ) -> VMResult<()> {
        self.execute_function_bypass_visibility(
            module_id,
            function_name,
            vec![],
            args,
            &mut UnmeteredGasMeter,
        )?;
        Ok(())
    }

    pub(crate) fn execute_unmetered_system_function_once(
        mut self,
        module_id: &ModuleId,
        function_name: &IdentStr,
        args: Vec<MoveValue>,
    ) -> Result<VMOutput, VMStatus> {
        self.execute_unmetered_system_function(module_id, function_name, serialize_values(&args))
            .or_else(|err| {
                // TODO(aptos-vm-v2):
                //   V1 implementation remaps prologue error. Does it even make sense now with AA?
                //   Revisit and consider cleaning it up.
                expect_only_successful_execution(err, function_name.as_str(), self.log_context)
            })?;
        SYSTEM_TRANSACTIONS_EXECUTED.inc();

        let change_set = self.take_change_set()?;
        Ok(VMOutput::new(
            change_set,
            ModuleWriteSet::empty(),
            FeeStatement::zero(),
            TransactionStatus::Keep(ExecutionStatus::Success),
        ))
    }

    pub(crate) fn materialize_writes(&mut self) -> Result<(), VMStatus> {
        let aggregator_context = self.extensions.get::<NativeAggregatorContext>();
        let delayed_field_ids = aggregator_context
            .materialize_write_op_info(&self.new_slot_metadata)
            .map_err(|err| err.finish(Location::Undefined))?;

        let table_context = self.extensions.get::<NativeTableContext>();
        table_context
            .materialize(
                self.data_view.as_executor_view(),
                &FunctionValueExtensionAdapter {
                    module_storage: self.loader.unmetered_module_storage(),
                },
                &self.new_slot_metadata,
                &delayed_field_ids,
            )
            .map_err(|err| err.finish(Location::Undefined))?;

        // Note: events do not require materialization.

        self.data_cache
            .materialize(
                self.data_view,
                self.loader,
                &self.new_slot_metadata,
                &delayed_field_ids,
            )
            .map_err(|err| err.finish(Location::Undefined))?;

        Ok(())
    }

    fn take_change_set(&mut self) -> Result<VMChangeSet, VMStatus> {
        self.materialize_writes()?;

        let event_context = self.extensions.get_mut::<NativeEventContext>();
        let events = event_context.take_events();

        let aggregator_context = self.extensions.get_mut::<NativeAggregatorContext>();
        let (delayed_field_change_set, aggregator_v1_write_set, aggregator_v1_delta_set) =
            aggregator_context.take_writes()?;
        let delayed_field_ids = delayed_field_change_set.keys().copied().collect();

        let mut resource_write_set = self.data_cache.take_writes()?;

        let table_context = self.extensions.get_mut::<NativeTableContext>();
        let table_change_set = table_context.take_writes(
            self.data_view.as_executor_view(),
            &FunctionValueExtensionAdapter {
                module_storage: self.loader.unmetered_module_storage(),
            },
            &delayed_field_ids,
        )?;
        resource_write_set.extend(table_change_set);

        Ok(VMChangeSet::new(
            resource_write_set,
            events,
            delayed_field_change_set,
            aggregator_v1_write_set,
            aggregator_v1_delta_set,
        ))
    }

    pub(crate) fn build_gas_meter<'b, F, K, G>(
        &self,
        make_gas_meter: F,
        initial_balance: Gas,
        kill_switch: &'b K,
    ) -> G
    where
        G: AptosGasMeter,
        K: BlockSynchronizationKillSwitch,
        F: FnOnce(u64, VMGasParameters, StorageGasParameters, bool, Gas, &'b K) -> G + 'b,
    {
        let is_approved_gov_script = false;
        make_gas_meter(
            self.gas_feature_version,
            self.gas_params.vm.clone(),
            self.storage_gas_params.clone(),
            is_approved_gov_script,
            initial_balance,
            kill_switch,
        )
    }
}

pub struct UserTransactionSession<'a, DataView, CodeLoader> {
    /// Underlying session for executing Move code.
    pub(crate) session: Session<'a, DataView, CodeLoader>,
    /// Signature-verified signed transaction to be executed.
    pub(crate) txn: &'a SignedTransaction,
    /// Metadata that stores pre-processed information for the signed transaction.
    pub(crate) txn_metadata: TransactionMetadata,
    /// Extra transaction configs.
    pub(crate) txn_extra_config: TransactionExtraConfig,
    /// Contains transaction executable payload (entry function, script, etc.).
    pub(crate) executable: TransactionExecutableRef<'a>,
    /// If true, this transaction is an approved governance proposal script.
    pub(crate) is_approved_gov_script: bool,
    /// If true, this session runs in simulation mode.
    pub(crate) is_simulation: bool,
    pub(crate) storage_refund: Fee,
    pub(crate) serialized_signers: Option<SerializedSigners>,
    pub(crate) module_write_set: Option<ModuleWriteSet>,
}

impl<'a, DataView, CodeLoader> UserTransactionSession<'a, DataView, CodeLoader>
where
    DataView: AptosMoveResolver,
    CodeLoader: Loader,
{
    pub(crate) fn build_gas_meter<F, K, G>(&self, make_gas_meter: F, kill_switch: &'a K) -> G
    where
        G: AptosGasMeter,
        K: BlockSynchronizationKillSwitch,
        F: FnOnce(u64, VMGasParameters, StorageGasParameters, bool, Gas, &'a K) -> G + 'a,
    {
        let initial_balance = if let Some(max_aa_gas) = self.get_max_aa_gas() {
            max_aa_gas.min(self.txn.max_gas_amount().into())
        } else {
            self.txn.max_gas_amount().into()
        };

        make_gas_meter(
            self.gas_feature_version(),
            self.session.gas_params.vm.clone(),
            self.session.storage_gas_params.clone(),
            self.is_approved_gov_script,
            initial_balance,
            kill_switch,
        )
    }

    pub(crate) fn materialize_output(
        &mut self,
        fee_statement: FeeStatement,
        execution_status: ExecutionStatus,
    ) -> Result<VMOutput, VMStatus> {
        let change_set = self.session.take_change_set()?;
        let module_write_set = self
            .module_write_set
            .take()
            .unwrap_or_else(ModuleWriteSet::empty);

        Ok(VMOutput::new(
            change_set,
            module_write_set,
            fee_statement,
            TransactionStatus::Keep(execution_status),
        ))
    }
}

impl<'a, DataView, CodeLoader> UserTransactionSession<'a, DataView, CodeLoader>
where
    DataView: AptosMoveResolver,
    CodeLoader: AptosLoader + ScriptLoader,
{
    pub(crate) fn execute_user_transaction(
        &mut self,
        gas_meter: &mut impl AptosGasMeter,
    ) -> VMOutput {
        // When running prologue, AA authentication charges gas up to some fixed amount. This
        // amount is injected when the gas meter is built for the session and is restored when
        // prologue terminates.
        // TODO(aptos-vm-v2):
        //   Right now prologue is not charged, even though it can be relatively expensive.
        //   Consider using a single gas meter and charge for prologue + user session.
        let initial_gas = gas_meter.balance();

        let prologue_result = {
            let _timer = VM_TIMER.timer_with_label("execute_user_transaction_prologue");
            self.execute_user_transaction_prologue(gas_meter)
        };

        // If prologue failed, discard the transaction.
        if let Err(status) = prologue_result {
            return VMOutput::discarded(status.status_code());
        };

        if let Err(status) = self.restore_gas_meter_balance_after_prologue(gas_meter, initial_gas) {
            return VMOutput::discarded(status.status_code());
        }

        // Reset hashes and extensions, do not save yet!
        self.update_extensions(SessionId::txn_meta(&self.txn_metadata));

        if let Err(err) = self.create_account_for_sponsored_txns(gas_meter) {
            return VMOutput::discarded(err.major_status());
        }

        // TODO(aptos-vm-v2):
        //   In the end, we need to uncomment this to be able prologue writes are always accounted
        //   for. Keep commented out for now for debugging & replay purposes.
        // if let Err(status) = self.check_gas_for_state_changes(gas_meter) {
        //     return VMOutput::discarded(status.status_code());
        // }

        // Prologue passed. Save the current state, so we can restore it later in case user payload
        // fails, or we fail successful epilogue.
        self.save_state_changes();

        let user_payload_result = {
            let _timer = VM_TIMER.timer_with_label("execute_user_transaction_payload");
            self.execute_user_transaction_payload(gas_meter)
        };

        let gas_usage = gas_used(self.txn_metadata.max_gas_amount(), gas_meter);
        TXN_GAS_USAGE.observe(gas_usage as f64);

        // If user payload execution is successful (together with success epilogue), return the
        // obtained output. Otherwise, we need to revert to the prologue state and execute failure
        // epilogue to still charge gas.
        user_payload_result.unwrap_or_else(|status| {
            // TODO(aptos-vm-v2):
            //   We reset refund to 0, but if charge for I/O and storage of prologue writes, we
            //   might as well just use prologue refund.
            self.storage_refund = 0.into();
            self.undo_state_changes();
            self.execute_user_transaction_failure_epilogue(gas_meter, status)
                .unwrap_or_else(|status| {
                    // If failure epilogue fails, there is nothing we can do and the transaction is
                    // discarded.
                    VMOutput::discarded(status.status_code())
                })
        })
    }
}

impl<'a, DataView, CodeLoader> UserTransactionSession<'a, DataView, CodeLoader>
where
    DataView: AptosMoveResolver,
    CodeLoader: AptosLoader + ScriptLoader,
{
    fn execute_user_transaction_payload(
        &mut self,
        gas_meter: &mut impl AptosGasMeter,
    ) -> Result<VMOutput, VMStatus> {
        gas_meter.charge_intrinsic_gas_for_transaction(self.txn_metadata.transaction_size())?;
        if self.txn_metadata.is_keyless() {
            gas_meter.charge_keyless()?;
        }

        if let Some(multisig_address) = self.txn.multisig_address() {
            let (payload_bytes, payload) =
                self.extract_multisig_payload(gas_meter, multisig_address)?;
            if let Err(status) = self.execute_multisig_payload(gas_meter, &payload) {
                // Note 1: Failure hook updates session ID to epilogue.
                // Note 2: There is no need to charge gas for the write sets because they will not
                //         be committed and the hook reverts all changes made when execution the
                //         multisig payload.
                self.execute_multisig_payload_failure_hook(
                    multisig_address,
                    payload_bytes,
                    status,
                )?;
                assert_eq!(self.storage_refund, 0.into());
            } else {
                // Note: Success hook updates session ID to epilogue.
                let refund = self.charge_gas_for_state_changes(gas_meter)?;
                self.storage_refund = refund;
                self.execute_multisig_payload_success_hook(multisig_address, payload_bytes)?;
            }
        } else {
            match self.executable {
                TransactionExecutableRef::Script(script) => {
                    self.execute_script(gas_meter, script)?
                },
                TransactionExecutableRef::EntryFunction(entry_func) => {
                    self.execute_entry_function(gas_meter, entry_func)?
                },

                // Not reachable as this function should only be invoked for entry or script
                // transaction payload. This is checked in prologue.
                TransactionExecutableRef::Empty => {
                    unreachable!("Only scripts or entry functions are executed")
                },
            }

            let refund = self.charge_gas_for_state_changes(gas_meter)?;
            self.storage_refund = refund;
            self.update_extensions(SessionId::epilogue_meta(&self.txn_metadata));
        };

        self.execute_user_transaction_success_epilogue(gas_meter)
    }

    fn execute_script(
        &mut self,
        gas_meter: &mut impl AptosGasMeter,
        serialized_script: &Script,
    ) -> Result<(), VMStatus> {
        let legacy_loader_config = LegacyLoaderConfig {
            charge_for_dependencies: self.gas_feature_version() >= RELEASE_V1_10,
            charge_for_ty_tag_dependencies: self.gas_feature_version() >= RELEASE_V1_27,
        };

        let func = self.session.loader.load_script(
            &legacy_loader_config,
            gas_meter,
            self.session.traversal_context,
            serialized_script.code(),
            serialized_script.ty_args(),
        )?;

        // Check that unstable bytecode cannot be executed on mainnet and verify events.
        let script = func.owner_as_script()?;
        script_validation::reject_unstable_bytecode_for_script(script, self.chain_id())?;
        event_validation::verify_no_event_emission_in_compiled_script(script)?;

        let _struct_constructors_enabled =
            self.features().is_enabled(FeatureFlag::STRUCT_CONSTRUCTORS);
        let _serialized_signers = self
            .serialized_signers
            .as_ref()
            .expect("Serialized signers must be computed by prologue");

        // TODO(aptos-vm-v2): support argument validation for V2.
        let args = Vec::<Vec<u8>>::new();
        self.session
            .execute_loaded_function(func, args, gas_meter)?;

        self.resolve_published_modules(gas_meter)?;
        Ok(())
    }
}

impl<'a, DataView, CodeLoader> UserTransactionSession<'a, DataView, CodeLoader>
where
    DataView: AptosMoveResolver,
    CodeLoader: AptosLoader,
{
    pub(crate) fn execute_entry_function(
        &mut self,
        gas_meter: &mut impl AptosGasMeter,
        entry_func: &EntryFunction,
    ) -> Result<(), VMStatus> {
        let legacy_loader_config = LegacyLoaderConfig {
            charge_for_dependencies: self.gas_feature_version() >= RELEASE_V1_10,
            charge_for_ty_tag_dependencies: self.gas_feature_version() >= RELEASE_V1_27,
        };

        let function = self.session.loader.load_instantiated_function(
            &legacy_loader_config,
            gas_meter,
            self.session.traversal_context,
            entry_func.module(),
            entry_func.function(),
            entry_func.ty_args(),
        )?;

        // The function also must be an entry function.
        function.is_entry_or_err()?;

        // Native entry function is forbidden.
        if function.is_native() {
            return Err(
                PartialVMError::new(StatusCode::USER_DEFINED_NATIVE_NOT_ALLOWED)
                    .with_message(
                        "Executing user defined native entry function is not allowed".to_string(),
                    )
                    .finish(Location::Module(entry_func.module().clone()))
                    .into_vm_status(),
            );
        }

        if function.is_friend_or_private() {
            let maybe_randomness_annotation = get_randomness_annotation_for_entry_function(
                entry_func,
                &function.owner_as_module()?.metadata,
            );
            if maybe_randomness_annotation.is_some() {
                self.session
                    .extensions
                    .get_mut::<RandomnessContext>()
                    .mark_unbiasable();
            }
        }

        let _struct_constructors_enabled =
            self.features().is_enabled(FeatureFlag::STRUCT_CONSTRUCTORS);
        let serialized_signers = self
            .serialized_signers
            .as_ref()
            .expect("Serialized signers must be computed by prologue");

        // TODO(aptos-vm-v2): support argument validation for V2.
        let mut signer_param_cnt = 0;
        for ty in function.param_tys() {
            if ty.is_signer_or_signer_ref() {
                signer_param_cnt += 1;
            }
        }
        let args = if signer_param_cnt == 0 {
            entry_func.args().to_vec()
        } else {
            let mut args = serialized_signers.senders().clone();
            args.extend(entry_func.args().to_vec());
            args
        };

        self.session
            .execute_loaded_function(function, args, gas_meter)?;

        self.resolve_published_modules(gas_meter)?;
        Ok(())
    }
}

impl<'a, DataView, CodeLoader> UserTransactionSession<'a, DataView, CodeLoader>
where
    DataView: AptosMoveResolver,
    CodeLoader: Loader,
{
    fn get_max_aa_gas(&self) -> Option<Gas> {
        if self.features().is_account_abstraction_enabled()
            || self.features().is_derivable_account_abstraction_enabled()
        {
            Some(self.session.gas_params.vm.txn.max_aa_gas)
        } else {
            None
        }
    }

    fn restore_gas_meter_balance_after_prologue(
        &self,
        gas_meter: &mut impl AptosGasMeter,
        initial_gas: Gas,
    ) -> Result<(), VMStatus> {
        if let Some(max_aa_gas) = self.get_max_aa_gas() {
            let max_gas_amount = self.txn_metadata.max_gas_amount();
            if max_aa_gas < max_gas_amount {
                // Reset initial gas after validation with max_aa_gas.
                let balance = max_gas_amount.checked_sub(max_aa_gas).unwrap();
                gas_meter.inject_balance(balance)?;
            }
        } else {
            assert_eq!(initial_gas, gas_meter.balance());
        }
        Ok(())
    }

    #[allow(dead_code)]
    fn check_gas_for_state_changes(
        &mut self,
        gas_meter: &mut impl AptosGasMeter,
    ) -> Result<(), VMStatus> {
        // TODO(aptos-vm-v2):
        //   Check if this work with gas profiler... It looks like we would wrap things twice.
        let mut simulation_gas_meter = make_prod_gas_meter(
            gas_meter.feature_version(),
            gas_meter.vm_gas_params().clone(),
            gas_meter.storage_gas_params().clone(),
            self.is_approved_gov_script,
            gas_meter.balance(),
            // No need to kill switch here.
            &NoopBlockSynchronizationKillSwitch {},
        );

        self.charge_gas_for_state_changes(&mut simulation_gas_meter)?;
        Ok(())
    }

    fn charge_gas_for_state_changes(
        &mut self,
        gas_meter: &mut impl AptosGasMeter,
    ) -> Result<Fee, VMStatus> {
        self.session.materialize_writes()?;

        let change_set_configs = &self.session.storage_gas_params.change_set_configs;
        let pricing = &self.session.storage_gas_params.space_pricing;
        let params = &self.session.gas_params.vm.txn;
        let mut size_tracker =
            ChangeSetSizeTracker::new(change_set_configs, Some(pricing), Some(params));

        gas_meter.charge_io_gas_for_transaction(self.txn_metadata.transaction_size())?;

        let mut event_fee = Fee::new(0);
        let event_context = self.session.extensions.get::<NativeEventContext>();
        for event in event_context.events_iter() {
            event_fee += pricing.legacy_storage_fee_per_event(params, event);
            size_tracker.record_event(event)?;
            gas_meter.charge_io_gas_for_event(event)?;
        }
        let event_discount = pricing.legacy_storage_discount_for_events(params, event_fee);
        let event_net_fee = event_fee
            .checked_sub(event_discount)
            .expect("event discount should always be less than or equal to total amount");

        // Txn (no txn fee in v2)
        let txn_fee = pricing.legacy_storage_fee_for_transaction_storage(
            params,
            self.txn_metadata.transaction_size(),
        );

        let aggregator_context = self.session.extensions.get::<NativeAggregatorContext>();
        aggregator_context.charge_write_ops(&mut size_tracker, gas_meter)?;

        let table_context = self.session.extensions.get::<NativeTableContext>();
        table_context.charge_write_ops(&mut size_tracker, gas_meter)?;

        self.session
            .data_cache
            .charge_write_ops(&mut size_tracker, gas_meter)?;

        let fee = size_tracker.write_fee + event_net_fee + txn_fee;
        gas_meter
            .charge_storage_fee(fee, self.txn_metadata.gas_unit_price())
            .map_err(|err| err.finish(Location::Undefined))?;

        Ok(size_tracker.total_refund)
    }

    /// Given an execution status that is a Move abort, injects abort information (message, etc.)
    /// into it. For all other statuses, a no-op.
    pub(crate) fn inject_abort_info_if_available(
        &self,
        status: ExecutionStatus,
    ) -> ExecutionStatus {
        if let ExecutionStatus::MoveAbort {
            location: AbortLocation::Module(module_id),
            code,
            ..
        } = status
        {
            let info = self
                .session
                .loader
                .unmetered_module_storage()
                .unmetered_get_existing_module_metadata(module_id.address(), module_id.name())
                .ok()
                .and_then(|metadata| get_metadata(&metadata))
                .and_then(|m| m.extract_abort_info(code));
            ExecutionStatus::MoveAbort {
                location: AbortLocation::Module(module_id),
                code,
                info,
            }
        } else {
            status
        }
    }

    /// Updates states of native extensions with new transaction and script hashes extracted from
    /// the session ID.
    pub(crate) fn update_extensions(&mut self, session_id: SessionId) {
        let txn_hash = session_id.txn_hash();
        let script_hash = session_id.into_script_hash();
        self.session
            .extensions
            .for_each_mut(|e| e.update(&txn_hash, &script_hash));
    }

    /// Saves the current state of the data cache and the native extensions. This state can later
    /// be recovered via [AptosVMv2UserSession::undo_state_changes].
    pub(crate) fn save_state_changes(&mut self) {
        self.session.data_cache.save();
        self.session.extensions.for_each_mut(|e| e.save());
    }

    /// Rollbacks changes to the data cache and native extensions to the previously saved version.
    /// If there is no last saved version and there are no changes, a no-op.
    pub(crate) fn undo_state_changes(&mut self) {
        self.session.data_cache.undo();
        self.session.extensions.for_each_mut(|e| e.undo());
    }

    /// Returns features used by the current session.
    pub(crate) fn features(&self) -> &Features {
        self.session.features
    }

    /// Returns the chain ID used by the current session.
    pub(crate) fn chain_id(&self) -> ChainId {
        self.session.chain_id
    }

    /// Returns the gas feature version used by the current session.
    pub(crate) fn gas_feature_version(&self) -> u64 {
        self.session.gas_feature_version
    }

    /// Determines if an account should be automatically created as part of a transaction.
    /// This function checks several conditions that must all be met:
    ///
    ///   1. Either DEFAULT_ACCOUNT_RESOURCE is enabled, or transaction has a fee payer and
    ///      SPONSORED_AUTOMATIC_ACCOUNT_V1_CREATION is enabled.
    ///   2. Transaction sequence number is 0 (indicating a new account).
    ///   4. Account resource does not already exist for the sender address.
    ///
    /// This is used to support automatic account creation for sponsored transactions or after
    /// enabling default account resource feature, allowing new accounts to be created without
    /// requiring an explicit account creation transaction.
    fn is_account_creation_required(&self) -> VMResult<bool> {
        if (self
            .features()
            .is_enabled(FeatureFlag::DEFAULT_ACCOUNT_RESOURCE)
            || (self
                .features()
                .is_enabled(FeatureFlag::SPONSORED_AUTOMATIC_ACCOUNT_V1_CREATION)
                && self.txn_metadata.fee_payer.is_some()))
            && self.txn_metadata.replay_protector == ReplayProtector::SequenceNumber(0)
        {
            let account_tag = AccountResource::struct_tag();
            let state_key = resource_state_key(&self.txn_metadata.sender(), &account_tag)
                .map_err(|err| err.finish(Location::Undefined))?;

            return self
                .session
                .data_view
                // TODO(aptos-vm-v2):
                //   Refactor data view so that the cast is not needed and we have access to both
                //   resources and groups.
                .as_executor_view()
                .resource_exists(&state_key)
                .map_err(|err| err.finish(Location::Undefined));
        }
        Ok(false)
    }

    fn create_account_for_sponsored_txns(
        &mut self,
        gas_meter: &mut impl AptosGasMeter,
    ) -> VMResult<()> {
        if self.is_account_creation_required()? {
            self.session.execute_function_bypass_visibility(
                &ACCOUNT_MODULE,
                CREATE_ACCOUNT_IF_DOES_NOT_EXIST,
                vec![],
                serialize_values(&vec![MoveValue::Address(self.txn_metadata.sender())]),
                gas_meter,
            )?;
        }
        Ok(())
    }
}

pub(crate) fn gas_used(max_gas_amount: Gas, gas_meter: &impl AptosGasMeter) -> u64 {
    max_gas_amount
        .checked_sub(gas_meter.balance())
        .expect("Balance should always be less than or equal to max gas amount")
        .into()
}
