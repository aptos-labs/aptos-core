// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    aptos_vm::should_create_account_resource,
    move_vm_ext::{
        session::{
            respawned_session::RespawnedSession,
            user_transaction_sessions::{
                abort_hook::AbortHookSession,
                epilogue::EpilogueSession,
                prologue::PrologueSession,
                session_change_sets::{SystemSessionChangeSet, UserSessionChangeSet},
                user::UserSession,
            },
        },
        AptosMoveResolver,
    },
    session::{
        common::{abort_hook_try_create_account, abort_hook_verify_gas_charge_for_slot_creation},
        Session, TransactionalSession,
    },
    transaction_metadata::TransactionMetadata,
    AptosVM,
};
use aptos_framework::natives::code::PublishRequest;
use aptos_gas_algebra::Fee;
use aptos_gas_meter::AptosGasMeter;
use aptos_logger::info;
use aptos_types::{
    fee_statement::FeeStatement,
    on_chain_config::Features,
    transaction::{ExecutionStatus, ModuleBundle},
};
use aptos_vm_logging::log_schema::AdapterLogSchema;
use aptos_vm_types::{
    module_and_script_storage::module_storage::AptosModuleStorage,
    module_write_set::ModuleWriteSet, output::VMOutput,
    storage::change_set_configs::ChangeSetConfigs,
};
use move_binary_format::{compatibility::Compatibility, errors::VMResult, CompiledModule};
use move_core_types::{
    account_address::AccountAddress,
    identifier::IdentStr,
    language_storage::{ModuleId, TypeTag},
    vm_status::VMStatus,
};
use move_vm_runtime::{
    module_traversal::TraversalContext, move_vm::SerializedReturnValues, LoadedFunction,
    ModuleStorage,
};
use move_vm_types::gas::GasMeter;
use std::borrow::Borrow;

/// Represents different states of user transaction execution. Only used for the legacy session.
enum LegacySessionState<'a> {
    /// Prologue session: used for validation of the transaction. Any failures here result in
    /// transaction being discarded. State changes to [LegacySessionState::UserSession] once the
    /// prologue successfully terminates.
    PrologueSession { session: PrologueSession<'a> },
    /// User session: used to execute transaction's payload. If successful, the state transitions
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
    /// Represents the state when user transaction payload is successfully executed. During this
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
