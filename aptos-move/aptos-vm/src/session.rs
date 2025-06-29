// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    aptos_vm::{create_account_if_does_not_exist, should_create_account_resource},
    errors::expect_only_successful_execution,
    move_vm_ext::{
        convert_modules_into_write_ops,
        session::{
            aptos_extensions,
            respawned_session::RespawnedSession,
            user_transaction_sessions::{
                abort_hook::AbortHookSession,
                epilogue::EpilogueSession,
                prologue::PrologueSession,
                session_change_sets::{SystemSessionChangeSet, UserSessionChangeSet},
                user::{run_init_modules, UserSession},
            },
        },
        AptosMoveResolver, SessionId,
    },
    system_module_names::{ACCOUNT_MODULE, CREATE_ACCOUNT_IF_DOES_NOT_EXIST},
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
    errors::{Location, PartialVMError, PartialVMResult, VMResult},
    CompiledModule,
};
use move_core_types::{
    account_address::AccountAddress,
    identifier::IdentStr,
    language_storage::{ModuleId, TypeTag},
    vm_status::{StatusCode, VMStatus},
};
use move_vm_runtime::{
    data_cache::TransactionDataCache,
    logging::expect_no_verification_errors,
    module_traversal::TraversalContext,
    move_vm::{MoveVM, SerializedReturnValues},
    native_extensions::NativeContextExtensions,
    LoadedFunction, ModuleStorage,
};
use move_vm_types::{
    delayed_values::delayed_field_id::DelayedFieldID,
    gas::{GasMeter, UnmeteredGasMeter},
};
use std::{borrow::Borrow, collections::BTreeMap};

/// Represents any session that can be used by Aptos VM to execute Move functions.
pub trait Session {
    /// Executes Move function (ignoring its visibility), specified by its name, with the provided
    /// arguments.
    fn execute_function_bypass_visibility(
        &mut self,
        module_id: &ModuleId,
        function_name: &IdentStr,
        ty_args: Vec<TypeTag>,
        args: Vec<impl Borrow<[u8]>>,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        module_storage: &impl ModuleStorage,
    ) -> VMResult<SerializedReturnValues>;

    /// Executes already loaded Move function with the provided arguments.
    fn execute_loaded_function(
        &mut self,
        func: LoadedFunction,
        args: Vec<impl Borrow<[u8]>>,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        module_storage: &impl ModuleStorage,
    ) -> VMResult<SerializedReturnValues>;

    /// Returns the publish request made in the native context of execution. Returns [None] if
    /// it does not exist.
    fn extract_publish_request(&mut self) -> Option<PublishRequest>;

    /// Marks the randomness native context as unbiasable.
    fn mark_unbiasable(&mut self);
}

/// Represents different states of user transaction execution.
enum LegacySessionState<'a> {
    /// Prologue session: used for validation of the transaction. Any failures here result in
    /// transaction being discarded. State changes to [LegacySessionState::UserSession] once the
    /// prologue successfully terminates.
    PrologueSession { session: PrologueSession<'a> },
    /// User session: used to execute transactions' payload. If successful, the state transitions
    /// into [LegacySessionState::GasCharging]. If not successful, there are 2 options:
    ///   1. If transaction is not multisig transaction, then the error is propagated and handled
    ///      by the caller to initiate [LegacySessionState::FailureEpilogue].
    ///   2. If transaction is a multisig transaction, then there is no gas charging but the state
    ///      transitions to [LegacySessionState::SuccessEpilogue] directly. This is because for
    ///      multisig transactions failures are recorded on-chain, and so even if payload fails,
    ///      successful epilogue is invoked.
    UserSession {
        session: UserSession<'a>,
        prologue_change_set: SystemSessionChangeSet,
    },
    /// Represents teh state when user transaction payload is successfully executed. During this
    /// state, gas is charged for the produced change set.
    GasCharging {
        change_set: UserSessionChangeSet,
        prologue_change_set: SystemSessionChangeSet,
    },
    /// Success epilogue runs after either:
    ///   1. Gas has been successfully charged for transaction, or
    ///   2. Transaction is a multisig transaction and its payload failed.
    SuccessEpilogue {
        session: EpilogueSession<'a>,
        prologue_change_set: SystemSessionChangeSet,
    },
    /// Special state that is used only when success epilogue fails to produce transaction outputs.
    /// In this case, the session is already finished, and so it is not possible to stay in the
    /// existing state.
    SuccessEpilogueFailed {
        prologue_change_set: SystemSessionChangeSet,
    },
    /// When transaction execution fails, runs on top of prologue state changes. Can be reached
    /// from the following states:
    ///   1. [LegacySessionState::UserSession] - user code failed, e.g., there was a Move abort.
    ///   2. [LegacySessionState::GasCharging] - VM failed to charge gas for the change set.
    ///   3. [LegacySessionState::SuccessEpilogue] - VM failed to run epilogue successfully, e.g.,
    ///      user transferred all its funds out, so gas fees cannot be paid.
    ///   4. [LegacySessionState::SuccessEpilogueFailed] - when producing outputs in the success
    ///      epilogue, there was an error.
    FailureEpilogue { session: EpilogueSession<'a> },
}

/// Represents a legacy session to execute a user transaction. Throughout execution, Aptos VM
/// changes the inner states of this session in order to produce transaction outputs.
pub struct LegacySession<'a, DataView> {
    /// Current session state. Wrapped in option to be able to take ownership during state
    /// transitions. An invariant is maintained that the state is always set otherwise.
    state: Option<LegacySessionState<'a>>,
    /// Base view of data before this session started. Represents an original view before a
    /// transaction is executed.
    data_view: &'a DataView,
    /// Metadata of transaction that is being executed.
    txn_metadata: &'a TransactionMetadata,
    /// Gas feature version for the current environment.
    gas_feature_version: u64,
    /// Features for the current environment.
    features: &'a Features,
    /// Configs to charge for the change set from the current environment.
    change_set_configs: &'a ChangeSetConfigs,
}

impl<'a, DataView> Session for LegacySession<'a, DataView>
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
        self.respawned()
            .expect("Session must be set to execute a Move function")
            .execute(|session| {
                session.execute_function_bypass_visibility(
                    module_id,
                    function_name,
                    ty_args,
                    args,
                    gas_meter,
                    traversal_context,
                    module_storage,
                )
            })
    }

    fn execute_loaded_function(
        &mut self,
        func: LoadedFunction,
        args: Vec<impl Borrow<[u8]>>,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        module_storage: &impl ModuleStorage,
    ) -> VMResult<SerializedReturnValues> {
        self.respawned()
            .expect("Session must be set to execute a Move function")
            .execute(|session| {
                session.execute_loaded_function(
                    func,
                    args,
                    gas_meter,
                    traversal_context,
                    module_storage,
                )
            })
    }

    fn extract_publish_request(&mut self) -> Option<PublishRequest> {
        self.respawned()
            .expect("Session must be set to extract a publish request")
            .execute(|session| session.extract_publish_request())
    }

    fn mark_unbiasable(&mut self) {
        self.respawned()
            .expect("Session must be set to mark it as unbiasable")
            .execute(|session| session.mark_unbiasable())
    }
}

pub trait TransactionalSession<'a, DataView>: Session {
    /// Returns the base view used for data (resources, resource groups, configs, etc.).
    fn data_view(&self) -> &DataView;

    fn end_prologue_and_start_user_session(
        &mut self,
        vm: &AptosVM,
        module_storage: &impl AptosModuleStorage,
    ) -> Result<(), VMStatus>;

    fn end_user_session_without_publish_request(
        &mut self,
        module_storage: &impl AptosModuleStorage,
    ) -> Result<(), VMStatus>;

    fn end_user_session_with_publish_request(
        &mut self,
        module_storage: &impl AptosModuleStorage,
        gas_meter: &mut impl AptosGasMeter,
        traversal_context: &mut TraversalContext,
        destination: AccountAddress,
        bundle: ModuleBundle,
        modules: &[CompiledModule],
        compatability_checks: Compatibility,
    ) -> Result<(), VMStatus>;

    fn charge_change_set_and_start_success_epilogue(
        &mut self,
        vm: &AptosVM,
        gas_meter: &mut impl AptosGasMeter,
        module_storage: &impl AptosModuleStorage,
    ) -> Result<Fee, VMStatus>;

    fn end_success_epilogue(
        &mut self,
        fee_statement: FeeStatement,
        module_storage: &impl AptosModuleStorage,
    ) -> Result<(VMStatus, VMOutput), VMStatus>;

    fn mark_multisig_payload_execution_failure_and_start_success_epilogue(&mut self, vm: &AptosVM);

    fn start_failure_epilogue_with_abort_hook(
        &mut self,
        vm: &AptosVM,
        gas_meter: &mut impl AptosGasMeter,
        module_storage: &impl AptosModuleStorage,
        log_context: &AdapterLogSchema,
        traversal_context: &mut TraversalContext,
    ) -> Result<FeeStatement, VMStatus>;

    fn end_failure_epilogue(
        &mut self,
        fee_statement: FeeStatement,
        status: ExecutionStatus,
        module_storage: &impl AptosModuleStorage,
    ) -> Result<VMOutput, VMStatus>;
}

impl<'a, DataView> TransactionalSession<'a, DataView> for LegacySession<'a, DataView>
where
    DataView: AptosMoveResolver,
{
    fn data_view(&self) -> &DataView {
        self.data_view
    }

    fn end_prologue_and_start_user_session(
        &mut self,
        vm: &AptosVM,
        module_storage: &impl AptosModuleStorage,
    ) -> Result<(), VMStatus> {
        match self.take_state() {
            LegacySessionState::PrologueSession { session } => {
                let (prologue_change_set, session) = session.into_user_session(
                    vm,
                    self.txn_metadata,
                    self.data_view,
                    self.change_set_configs,
                    module_storage,
                )?;
                self.set_state(LegacySessionState::UserSession {
                    session,
                    prologue_change_set,
                });
                Ok(())
            },
            _ => unreachable!("Only prologue session can be ended"),
        }
    }

    fn end_user_session_without_publish_request(
        &mut self,
        module_storage: &impl AptosModuleStorage,
    ) -> Result<(), VMStatus> {
        match self.take_state() {
            LegacySessionState::UserSession {
                session,
                prologue_change_set,
            } => {
                let change_set = session.finish(self.change_set_configs, module_storage)?;
                let change_set = UserSessionChangeSet::new(
                    change_set,
                    ModuleWriteSet::empty(),
                    self.change_set_configs,
                )?;
                self.set_state(LegacySessionState::GasCharging {
                    change_set,
                    prologue_change_set,
                });
                Ok(())
            },
            _ => unreachable!("Only user session can be ended"),
        }
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
        match self.take_state() {
            LegacySessionState::UserSession {
                session,
                prologue_change_set,
            } => {
                let change_set = session.finish_with_module_publishing_and_initialization(
                    self.data_view,
                    module_storage,
                    gas_meter,
                    traversal_context,
                    self.features,
                    self.gas_feature_version,
                    self.change_set_configs,
                    destination,
                    bundle,
                    modules,
                    compatability_checks,
                )?;
                self.set_state(LegacySessionState::GasCharging {
                    change_set,
                    prologue_change_set,
                });
                Ok(())
            },
            _ => unreachable!("Only user session can be ended"),
        }
    }

    fn charge_change_set_and_start_success_epilogue(
        &mut self,
        vm: &AptosVM,
        gas_meter: &mut impl AptosGasMeter,
        module_storage: &impl AptosModuleStorage,
    ) -> Result<Fee, VMStatus> {
        match self.take_state() {
            LegacySessionState::GasCharging {
                mut change_set,
                prologue_change_set,
            } => {
                let storage_refund = vm.charge_change_set(
                    &mut change_set,
                    gas_meter,
                    self.txn_metadata,
                    self.data_view,
                    module_storage,
                )?;

                let session = EpilogueSession::on_user_session_success(
                    vm,
                    self.txn_metadata,
                    self.data_view,
                    change_set,
                );
                self.set_state(LegacySessionState::SuccessEpilogue {
                    session,
                    prologue_change_set,
                });
                Ok(storage_refund)
            },
            _ => unreachable!("Change set can only be charged in the corresponding state"),
        }
    }

    fn end_success_epilogue(
        &mut self,
        fee_statement: FeeStatement,
        module_storage: &impl AptosModuleStorage,
    ) -> Result<(VMStatus, VMOutput), VMStatus> {
        use LegacySessionState::*;
        match self.take_state() {
            SuccessEpilogue {
                session,
                prologue_change_set,
                ..
            } => {
                match session.finish(
                    fee_statement,
                    ExecutionStatus::Success,
                    self.change_set_configs,
                    module_storage,
                ) {
                    Ok(output) => Ok((VMStatus::Executed, output)),
                    Err(status) => {
                        self.set_state(SuccessEpilogueFailed {
                            prologue_change_set,
                        });
                        Err(status)
                    },
                }
            },
            _ => unreachable!("Can only end success epilogue"),
        }
    }

    fn mark_multisig_payload_execution_failure_and_start_success_epilogue(&mut self, vm: &AptosVM) {
        use LegacySessionState::*;
        let prologue_change_set = match self.take_state() {
            UserSession {
                prologue_change_set,
                ..
            } => prologue_change_set,
            PrologueSession { .. }
            | SuccessEpilogueFailed { .. }
            | FailureEpilogue { .. }
            | SuccessEpilogue { .. }
            | GasCharging { .. } => {
                unreachable!("Should only be called when in user session")
            },
        };

        let session = EpilogueSession::on_user_session_failure(
            vm,
            self.txn_metadata,
            self.data_view,
            prologue_change_set.clone(),
        );
        self.set_state(SuccessEpilogue {
            session,
            prologue_change_set,
        });
    }

    fn start_failure_epilogue_with_abort_hook(
        &mut self,
        vm: &AptosVM,
        gas_meter: &mut impl AptosGasMeter,
        module_storage: &impl AptosModuleStorage,
        log_context: &AdapterLogSchema,
        traversal_context: &mut TraversalContext,
    ) -> Result<FeeStatement, VMStatus> {
        use LegacySessionState::*;

        let prologue_change_set = match self.take_state() {
            UserSession {
                prologue_change_set,
                ..
            }
            | GasCharging {
                prologue_change_set,
                ..
            }
            | SuccessEpilogue {
                prologue_change_set,
                ..
            }
            | SuccessEpilogueFailed {
                prologue_change_set,
            } => prologue_change_set,
            PrologueSession { .. } | FailureEpilogue { .. } => {
                unreachable!("Failure epilogue can only be started after user session")
            },
        };

        // Storage refund is zero since no slots are deleted in aborted transactions.
        const ZERO_STORAGE_REFUND: u64 = 0;

        let should_create_account_resource = should_create_account_resource(
            self.txn_metadata,
            self.features,
            self.data_view,
            module_storage,
        )?;

        let (previous_session_change_set, fee_statement) = if should_create_account_resource {
            let mut abort_hook_session =
                AbortHookSession::new(vm, self.txn_metadata, self.data_view, prologue_change_set);

            let sender = self.txn_metadata.sender();
            abort_hook_session.execute(|session| {
                abort_hook_try_create_account(
                    session,
                    sender,
                    gas_meter,
                    traversal_context,
                    module_storage,
                    log_context,
                )
            })?;

            let mut abort_hook_session_change_set =
                abort_hook_session.finish(self.change_set_configs, module_storage)?;
            if let Err(err) = vm.charge_change_set(
                &mut abort_hook_session_change_set,
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

            (abort_hook_session_change_set, fee_statement)
        } else {
            let fee_statement = AptosVM::fee_statement_from_gas_meter(
                self.txn_metadata,
                gas_meter,
                ZERO_STORAGE_REFUND,
            );
            (prologue_change_set, fee_statement)
        };

        let session = EpilogueSession::on_user_session_failure(
            vm,
            self.txn_metadata,
            self.data_view,
            previous_session_change_set,
        );
        self.set_state(FailureEpilogue { session });
        Ok(fee_statement)
    }

    fn end_failure_epilogue(
        &mut self,
        fee_statement: FeeStatement,
        status: ExecutionStatus,
        module_storage: &impl AptosModuleStorage,
    ) -> Result<VMOutput, VMStatus> {
        match self.take_state() {
            LegacySessionState::FailureEpilogue { session } => session.finish(
                fee_statement,
                status,
                self.change_set_configs,
                module_storage,
            ),
            _ => unreachable!("Can only end failure epilogue for failure epilogue state"),
        }
    }
}

impl<'a, DataView> LegacySession<'a, DataView>
where
    DataView: AptosMoveResolver,
{
    /// Creates a new session in prologue state.
    pub(crate) fn start_prologue_session(
        vm: &'a AptosVM,
        data_view: &'a DataView,
        txn_metadata: &'a TransactionMetadata,
        change_set_configs: &'a ChangeSetConfigs,
    ) -> Self {
        let session = PrologueSession::new(vm, txn_metadata, data_view);
        Self {
            state: Some(LegacySessionState::PrologueSession { session }),
            data_view,
            txn_metadata,
            gas_feature_version: vm.gas_feature_version(),
            features: vm.features(),
            change_set_configs,
        }
    }
}

// Private interfaces.
impl<'a, DataView> LegacySession<'a, DataView>
where
    DataView: AptosMoveResolver,
{
    fn state_mut(&mut self) -> &mut LegacySessionState<'a> {
        self.state.as_mut().expect("Session state is always set")
    }

    fn take_state(&mut self) -> LegacySessionState<'a> {
        self.state.take().expect("Session state is always set")
    }

    fn set_state(&mut self, state: LegacySessionState<'a>) {
        self.state = Some(state)
    }

    fn respawned(&mut self) -> Option<&mut RespawnedSession<'a>> {
        use LegacySessionState::*;
        Some(match self.state_mut() {
            PrologueSession { session } => session,
            UserSession { session, .. } => session,
            SuccessEpilogue { session, .. } | FailureEpilogue { session } => session,
            GasCharging { .. } | SuccessEpilogueFailed { .. } => {
                return None;
            },
        })
    }
}

pub struct ContinuousSession<'a, DataView> {
    /// Scratchpad for changes made by this session to the state (but not yet committed).
    data_cache: TransactionDataCache,
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
            self.data_view,
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
            self.data_view,
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
        // TODO:
        //  1. Save data cache prologue.
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
        // TODO: restore data cache to prologue state.
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

        // TODO: Restore data cache to prologue.
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

struct ChangeSetView<'a, 'b> {
    #[allow(dead_code)]
    data_cache: &'a TransactionDataCache,
    extensions: &'a NativeContextExtensions<'b>,
    module_writes: &'a mut BTreeMap<StateKey, ModuleWrite<WriteOp>>,
    resource_write_set: &'a mut BTreeMap<StateKey, AbstractResourceWriteOp>,
    #[allow(dead_code)]
    delayed_field_change_set: &'a mut BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>>,
    aggregator_v1_write_set: &'a mut BTreeMap<StateKey, WriteOp>,
    #[allow(dead_code)]
    aggregator_v1_delta_set: &'a mut BTreeMap<StateKey, DeltaOp>,
}

impl<'a, 'b> ChangeSetView<'a, 'b> {
    fn new(
        data_cache: &'a TransactionDataCache,
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

impl<'a, 'b> ChangeSetInterface for ChangeSetView<'a, 'b> {
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
                .fetch_module_size_in_bytes(write.module_address(), write.module_name())
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
            data_cache: TransactionDataCache::empty(),
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

fn abort_hook_try_create_account(
    session: &mut impl Session,
    sender: AccountAddress,
    gas_meter: &mut impl AptosGasMeter,
    traversal_context: &mut TraversalContext,
    module_storage: &impl AptosModuleStorage,
    log_context: &AdapterLogSchema,
) -> Result<(), VMStatus> {
    create_account_if_does_not_exist(
        session,
        module_storage,
        gas_meter,
        sender,
        traversal_context,
    )
    .or_else(|_| {
        // If this fails, it is likely due to out of gas, so we try again without
        // metering and then validate below that we charged sufficiently.
        create_account_if_does_not_exist(
            session,
            module_storage,
            &mut UnmeteredGasMeter,
            sender,
            traversal_context,
        )
    })
    .map_err(expect_no_verification_errors)
    .or_else(|err| {
        expect_only_successful_execution(
            err,
            &format!("{:?}::{}", ACCOUNT_MODULE, CREATE_ACCOUNT_IF_DOES_NOT_EXIST),
            log_context,
        )
    })?;
    Ok(())
}

fn abort_hook_verify_gas_charge_for_slot_creation(
    vm: &AptosVM,
    txn_metadata: &TransactionMetadata,
    log_context: &AdapterLogSchema,
    gas_meter: &mut impl AptosGasMeter,
    fee_statement: &FeeStatement,
) -> Result<(), VMStatus> {
    let gas_params = vm.gas_params(log_context)?;
    let gas_unit_price = u64::from(txn_metadata.gas_unit_price());
    if gas_unit_price != 0 || !vm.features().is_default_account_resource_enabled() {
        let gas_used = fee_statement.gas_used();
        let storage_fee = fee_statement.storage_fee_used();
        let storage_refund = fee_statement.storage_fee_refund();

        let actual = gas_used * gas_unit_price + storage_fee - storage_refund;
        let expected = u64::from(
            gas_meter
                .disk_space_pricing()
                .hack_account_creation_fee_lower_bound(&gas_params.vm.txn),
        );
        if actual < expected {
            expect_only_successful_execution(
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                    .with_message(
                        "Insufficient fee for storing account for lazy account creation"
                            .to_string(),
                    )
                    .finish(Location::Undefined),
                &format!("{:?}::{}", ACCOUNT_MODULE, CREATE_ACCOUNT_IF_DOES_NOT_EXIST),
                log_context,
            )?;
        }
    }

    Ok(())
}
