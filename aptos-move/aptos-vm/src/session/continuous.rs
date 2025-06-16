// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    aptos_vm::should_create_account_resource,
    move_vm_ext::{
        convert_modules_into_write_ops,
        session::{aptos_extensions, user_transaction_sessions::user::run_init_modules},
        AptosMoveResolver, SessionId,
    },
    session::{
        common::{abort_hook_try_create_account, abort_hook_verify_gas_charge_for_slot_creation},
        Session, TransactionalSession,
    },
    transaction_metadata::TransactionMetadata,
    AptosVM,
};
use aptos_aggregator::{delayed_change::DelayedChange, delta_change_set::DeltaOp};
use aptos_framework::natives::{
    code::{NativeCodeContext, PublishRequest},
    event::NativeEventContext,
    randomness::RandomnessContext,
};
use aptos_gas_algebra::Fee;
use aptos_gas_meter::AptosGasMeter;
use aptos_logger::info;
use aptos_types::{
    contract_event::ContractEvent,
    fee_statement::FeeStatement,
    on_chain_config::Features,
    state_store::state_key::StateKey,
    transaction::{
        user_transaction_context::UserTransactionContext, ExecutionStatus, ModuleBundle,
        TransactionStatus,
    },
    write_set::{TransactionWrite, WriteOp, WriteOpSize},
};
use aptos_vm_logging::log_schema::AdapterLogSchema;
use aptos_vm_types::{
    abstract_write_op::AbstractResourceWriteOp,
    change_set::{ChangeSetInterface, VMChangeSet, WriteOpInfo},
    module_and_script_storage::module_storage::AptosModuleStorage,
    module_write_set::{ModuleWrite, ModuleWriteSet},
    output::VMOutput,
    resolver::ExecutorView,
    storage::change_set_configs::ChangeSetConfigs,
};
use move_binary_format::{
    compatibility::Compatibility,
    errors::{Location, PartialVMResult, VMResult},
    CompiledModule,
};
use move_core_types::{
    account_address::AccountAddress,
    identifier::IdentStr,
    language_storage::{ModuleId, TypeTag},
    vm_status::VMStatus,
};
use move_vm_runtime::{
    data_cache::TransactionDataCache,
    module_traversal::TraversalContext,
    move_vm::{MoveVM, SerializedReturnValues},
    native_extensions::NativeContextExtensions,
    LoadedFunction, ModuleStorage,
};
use move_vm_types::{delayed_values::delayed_field_id::DelayedFieldID, gas::GasMeter};
use std::{borrow::Borrow, collections::BTreeMap};

pub struct ContinuousSession<'a, DataView> {
    /// Scratchpad for changes made by this session to the state (but not yet committed).
    data_cache: TransactionDataCache<'a, DataView>,
    /// Base view of data before this session started. Represents an original view before a
    /// transaction is executed.
    data_view: &'a DataView,

    /// Extensions for the Move VM.
    extensions: NativeContextExtensions<'a>,

    module_writes: BTreeMap<StateKey, ModuleWrite<WriteOp>>,
    resource_write_set: BTreeMap<StateKey, AbstractResourceWriteOp>,
    delayed_field_change_set: BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>>,
    aggregator_v1_write_set: BTreeMap<StateKey, WriteOp>,
    aggregator_v1_delta_set: BTreeMap<StateKey, DeltaOp>,

    /// Metadata of transaction that is being executed.
    txn_metadata: &'a TransactionMetadata,
    /// Gas feature version for the current environment.
    gas_feature_version: u64,
    /// Features for the current environment.
    features: &'a Features,
    /// Configs to charge for the change set from the current environment.
    #[allow(dead_code)]
    change_set_configs: &'a ChangeSetConfigs,
    #[allow(dead_code)]
    is_storage_slot_metadata_enabled: bool,
}

impl<'a, DataView> Session for ContinuousSession<'a, DataView>
where
    DataView: AptosMoveResolver,
{
    fn execute_function_bypass_visibility(
        &mut self,
        module_id: &ModuleId,
        function_name: &IdentStr,
        ty_args: Vec<TypeTag>,
        args: Vec<impl Borrow<[u8]>>,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        module_storage: &impl ModuleStorage,
    ) -> VMResult<SerializedReturnValues> {
        let func = module_storage.load_function(module_id, function_name, &ty_args)?;
        MoveVM::execute_loaded_function(
            func,
            args,
            &mut self.data_cache,
            gas_meter,
            traversal_context,
            &mut self.extensions,
            module_storage,
        )
    }

    fn execute_loaded_function(
        &mut self,
        func: LoadedFunction,
        args: Vec<impl Borrow<[u8]>>,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        module_storage: &impl ModuleStorage,
    ) -> VMResult<SerializedReturnValues> {
        MoveVM::execute_loaded_function(
            func,
            args,
            &mut self.data_cache,
            gas_meter,
            traversal_context,
            &mut self.extensions,
            module_storage,
        )
    }

    fn extract_publish_request(&mut self) -> Option<PublishRequest> {
        let ctx = self.extensions.get_mut::<NativeCodeContext>();
        ctx.extract_publish_request()
    }

    fn mark_unbiasable(&mut self) {
        let txn_context = self.extensions.get_mut::<RandomnessContext>();
        txn_context.mark_unbiasable();
    }
}

impl<'a, DataView> TransactionalSession<'a, DataView> for ContinuousSession<'a, DataView>
where
    DataView: AptosMoveResolver,
{
    fn data_view(&self) -> &DataView {
        self.data_view
    }

    fn end_prologue_and_start_user_session(
        &mut self,
        _vm: &AptosVM,
        _module_storage: &impl AptosModuleStorage,
    ) -> Result<(), VMStatus> {
        // FIXME
        // self.data_cache.save();
        self.extensions.apply_to_all(|ext| {
            ext.save();
        });
        self.update_extensions(SessionId::txn_meta(self.txn_metadata));
        Ok(())
    }

    fn end_user_session_without_publish_request(
        &mut self,
        _module_storage: &impl AptosModuleStorage,
    ) -> Result<(), VMStatus> {
        // No-op: we do not need to do anything if there are no modules published.
        Ok(())
    }

    fn end_user_session_with_publish_request(
        &mut self,
        module_storage: &impl AptosModuleStorage,
        gas_meter: &mut impl AptosGasMeter,
        traversal_context: &mut TraversalContext,
        destination: AccountAddress,
        bundle: ModuleBundle,
        modules: &[CompiledModule],
        compatability_checks: Compatibility,
    ) -> Result<(), VMStatus> {
        let gas_feature_version = self.gas_feature_version;
        let staging_module_storage = run_init_modules(
            self,
            module_storage,
            gas_meter,
            traversal_context,
            gas_feature_version,
            destination,
            bundle,
            modules,
            compatability_checks,
        )?;

        // TODO: Make sure cache caches metadata so that new writes for groups from init_module can
        //  be resolved correctly

        // Materialize module writes straight away: we do not expect any new publishes in epilogue
        // session.

        convert_modules_into_write_ops(
            &mut self.module_writes,
            self.data_view,
            self.features,
            module_storage,
            staging_module_storage.release_verified_module_bundle(),
        )
        .map_err(|e| e.finish(Location::Undefined))?;

        // Note: we do not check the change set size here. We will check once we materialize the
        // pending changes when charging gas.

        Ok(())
    }

    fn charge_change_set_and_start_success_epilogue(
        &mut self,
        vm: &AptosVM,
        gas_meter: &mut impl AptosGasMeter,
        module_storage: &impl AptosModuleStorage,
    ) -> Result<Fee, VMStatus> {
        let storage_refund = {
            let mut change_set_view = ChangeSetView::new(
                &self.data_cache,
                &self.extensions,
                &mut self.module_writes,
                &mut self.resource_write_set,
                &mut self.delayed_field_change_set,
                &mut self.aggregator_v1_write_set,
                &mut self.aggregator_v1_delta_set,
            )?;

            vm.charge_change_set(
                &mut change_set_view,
                gas_meter,
                self.txn_metadata,
                self.data_view,
                module_storage,
            )?
        };

        self.update_extensions(SessionId::epilogue_meta(self.txn_metadata));
        Ok(storage_refund)
    }

    fn end_success_epilogue(
        &mut self,
        fee_statement: FeeStatement,
        _module_storage: &impl AptosModuleStorage,
    ) -> Result<(VMStatus, VMOutput), VMStatus> {
        // TODO: Finalize write sets: epilogue run so we need to extract remaining changes from
        //   extensions and data cache. mem::take is enough, if this fails then we go to failure
        //   epilogue, but extensions cloned prologue before taking.
        let event_context = self.extensions.get_mut::<NativeEventContext>();
        let events = event_context.take_events();

        // TODO: we need to enforce some bounds here like before?
        let output = VMOutput::new(
            VMChangeSet::new(
                std::mem::take(&mut self.resource_write_set),
                events,
                std::mem::take(&mut self.delayed_field_change_set),
                std::mem::take(&mut self.aggregator_v1_write_set),
                std::mem::take(&mut self.aggregator_v1_delta_set),
            ),
            ModuleWriteSet::new(std::mem::take(&mut self.module_writes)),
            fee_statement,
            TransactionStatus::Keep(ExecutionStatus::Success),
        );
        Ok((VMStatus::Executed, output))
    }

    fn mark_multisig_payload_execution_failure_and_start_success_epilogue(
        &mut self,
        _vm: &AptosVM,
    ) {
        // FIXME
        // self.data_cache.undo();
        self.extensions.apply_to_all(|ext| {
            ext.undo();
        });
        self.update_extensions(SessionId::epilogue_meta(self.txn_metadata));
    }

    fn start_failure_epilogue_with_abort_hook(
        &mut self,
        vm: &AptosVM,
        gas_meter: &mut impl AptosGasMeter,
        module_storage: &impl AptosModuleStorage,
        log_context: &AdapterLogSchema,
        traversal_context: &mut TraversalContext,
    ) -> Result<FeeStatement, VMStatus> {
        // Storage refund is zero since no slots are deleted in aborted transactions.
        const ZERO_STORAGE_REFUND: u64 = 0;

        let should_create_account_resource = should_create_account_resource(
            self.txn_metadata,
            self.features,
            self.data_view,
            module_storage,
        )?;

        // FIXME
        // self.data_cache.undo();
        self.extensions.apply_to_all(|ext| {
            ext.undo();
        });
        let fee_statement = if should_create_account_resource {
            self.update_extensions(SessionId::run_on_abort(self.txn_metadata));

            let sender = self.txn_metadata.sender();
            abort_hook_try_create_account(
                self,
                sender,
                gas_meter,
                traversal_context,
                module_storage,
                log_context,
            )?;

            let mut change_set_view = ChangeSetView::new(
                &self.data_cache,
                &self.extensions,
                &mut self.module_writes,
                &mut self.resource_write_set,
                &mut self.delayed_field_change_set,
                &mut self.aggregator_v1_write_set,
                &mut self.aggregator_v1_delta_set,
            )?;

            if let Err(err) = vm.charge_change_set(
                &mut change_set_view,
                gas_meter,
                self.txn_metadata,
                self.data_view,
                module_storage,
            ) {
                info!(
                    *log_context,
                    "Failed during charge_change_set: {:?}. Most likely exceeded gas limited.", err,
                );
            };

            let fee_statement = AptosVM::fee_statement_from_gas_meter(
                self.txn_metadata,
                gas_meter,
                ZERO_STORAGE_REFUND,
            );

            // Verify we charged sufficiently for creating an account slot.
            abort_hook_verify_gas_charge_for_slot_creation(
                vm,
                self.txn_metadata,
                log_context,
                gas_meter,
                &fee_statement,
            )?;

            fee_statement
        } else {
            AptosVM::fee_statement_from_gas_meter(self.txn_metadata, gas_meter, ZERO_STORAGE_REFUND)
        };

        self.update_extensions(SessionId::epilogue_meta(self.txn_metadata));
        Ok(fee_statement)
    }

    fn end_failure_epilogue(
        &mut self,
        fee_statement: FeeStatement,
        status: ExecutionStatus,
        _module_storage: &impl AptosModuleStorage,
    ) -> Result<VMOutput, VMStatus> {
        // TODO: Finalize write sets: epilogue run so we need to extract remaining changes from
        //   extensions and data cache. mem::take is enough, if this fails then we go to failure
        //   epilogue, but extensions cloned prologue before taking.
        let event_context = self.extensions.get_mut::<NativeEventContext>();
        let events = event_context.take_events();

        // If transaction fails, it cannot publish modules.
        assert!(self.module_writes.is_empty());

        // TODO: we need to enforce some bounds here like before?
        Ok(VMOutput::new(
            VMChangeSet::new(
                std::mem::take(&mut self.resource_write_set),
                events,
                std::mem::take(&mut self.delayed_field_change_set),
                std::mem::take(&mut self.aggregator_v1_write_set),
                std::mem::take(&mut self.aggregator_v1_delta_set),
            ),
            ModuleWriteSet::empty(),
            fee_statement,
            TransactionStatus::Keep(status),
        ))
    }
}

struct ChangeSetView<'a, 'b, DataView> {
    #[allow(dead_code)]
    data_cache: &'a TransactionDataCache<'b, DataView>,
    extensions: &'a NativeContextExtensions<'b>,
    module_writes: &'a mut BTreeMap<StateKey, ModuleWrite<WriteOp>>,
    resource_write_set: &'a mut BTreeMap<StateKey, AbstractResourceWriteOp>,
    #[allow(dead_code)]
    delayed_field_change_set: &'a mut BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>>,
    aggregator_v1_write_set: &'a mut BTreeMap<StateKey, WriteOp>,
    #[allow(dead_code)]
    aggregator_v1_delta_set: &'a mut BTreeMap<StateKey, DeltaOp>,
}

impl<'a, 'b, DataView> ChangeSetView<'a, 'b, DataView>
where
    DataView: AptosMoveResolver,
{
    fn new(
        data_cache: &'a TransactionDataCache<'b, DataView>,
        extensions: &'a NativeContextExtensions<'b>,
        module_writes: &'a mut BTreeMap<StateKey, ModuleWrite<WriteOp>>,
        resource_write_set: &'a mut BTreeMap<StateKey, AbstractResourceWriteOp>,
        delayed_field_change_set: &'a mut BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>>,
        aggregator_v1_write_set: &'a mut BTreeMap<StateKey, WriteOp>,
        aggregator_v1_delta_set: &'a mut BTreeMap<StateKey, DeltaOp>,
    ) -> Result<Self, VMStatus> {
        // TODO: populate info from extensions and data cache into maps. Do not do this for events.
        Ok(Self {
            data_cache,
            extensions,
            module_writes,
            resource_write_set,
            delayed_field_change_set,
            aggregator_v1_write_set,
            aggregator_v1_delta_set,
        })
    }
}

impl<'a, 'b, DataView> ChangeSetInterface for ChangeSetView<'a, 'b, DataView>
where
    DataView: AptosMoveResolver,
{
    fn num_write_ops(&self) -> usize {
        self.resource_write_set.len()
            + self.aggregator_v1_write_set.len()
            + self.module_writes.len()
    }

    fn write_set_size_iter(&self) -> impl Iterator<Item = (&StateKey, WriteOpSize)> {
        self.resource_write_set
            .iter()
            .map(|(k, v)| (k, v.materialized_size()))
            .chain(
                self.aggregator_v1_write_set
                    .iter()
                    .map(|(k, v)| (k, v.write_op_size())),
            )
    }

    fn events_iter(&self) -> impl Iterator<Item = &ContractEvent> {
        self.extensions.get::<NativeEventContext>().events_iter()
    }

    fn write_op_info_iter_mut<'c>(
        &'c mut self,
        executor_view: &'c dyn ExecutorView,
        module_storage: &'c impl AptosModuleStorage,
        fix_prev_materialized_size: bool,
    ) -> impl Iterator<Item = PartialVMResult<WriteOpInfo<'c>>> {
        let resources = self.resource_write_set.iter_mut().map(move |(key, op)| {
            Ok(WriteOpInfo {
                key,
                op_size: op.materialized_size(),
                prev_size: op.prev_materialized_size(
                    key,
                    executor_view,
                    fix_prev_materialized_size,
                )?,
                metadata_mut: op.metadata_mut(),
            })
        });

        let v1_aggregators = self.aggregator_v1_write_set.iter_mut().map(|(key, op)| {
            Ok(WriteOpInfo {
                key,
                op_size: op.write_op_size(),
                prev_size: executor_view
                    .get_aggregator_v1_state_value_size(key)?
                    .unwrap_or(0),
                metadata_mut: op.metadata_mut(),
            })
        });

        let modules = self.module_writes.iter_mut().map(move |(key, write)| {
            let prev_size = module_storage
                .unmetered_get_module_size(write.module_address(), write.module_name())
                .map_err(|e| e.to_partial())?
                .unwrap_or(0) as u64;
            Ok(WriteOpInfo {
                key,
                op_size: write.write_op().write_op_size(),
                prev_size,
                metadata_mut: write.write_op_mut().metadata_mut(),
            })
        });

        resources.chain(v1_aggregators).chain(modules)
    }
}

impl<'a, DataView> ContinuousSession<'a, DataView>
where
    DataView: AptosMoveResolver,
{
    pub fn new(
        vm: &'a AptosVM,
        data_view: &'a DataView,
        txn_metadata: &'a TransactionMetadata,
        change_set_configs: &'a ChangeSetConfigs,
        session_id: SessionId,
        maybe_user_transaction_context: Option<UserTransactionContext>,
    ) -> Self {
        let extensions = aptos_extensions(
            data_view,
            vm.chain_id(),
            vm.runtime_environment().vm_config(),
            session_id,
            maybe_user_transaction_context,
        );
        let is_storage_slot_metadata_enabled = vm.features().is_storage_slot_metadata_enabled();
        Self {
            data_cache: TransactionDataCache::empty(data_view),
            data_view,
            extensions,
            module_writes: BTreeMap::new(),
            resource_write_set: BTreeMap::new(),
            delayed_field_change_set: BTreeMap::new(),
            aggregator_v1_write_set: BTreeMap::new(),
            aggregator_v1_delta_set: BTreeMap::new(),
            txn_metadata,
            gas_feature_version: vm.gas_feature_version(),
            features: vm.features(),
            change_set_configs,
            is_storage_slot_metadata_enabled,
        }
    }

    fn update_extensions(&mut self, session_id: SessionId) {
        let (txn_hash, script_hash) = session_id.txn_hash_and_script_hash();
        self.extensions.apply_to_all(|ext| {
            ext.update(&txn_hash, script_hash);
        });
    }
}
