// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    aptos_vm::{create_account_if_does_not_exist, should_create_account_resource},
    errors::expect_only_successful_execution,
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
    system_module_names::{ACCOUNT_MODULE, CREATE_ACCOUNT_IF_DOES_NOT_EXIST},
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
    change_set::ChangeSetInterface, module_and_script_storage::module_storage::AptosModuleStorage,
    module_write_set::ModuleWriteSet, output::VMOutput,
    storage::change_set_configs::ChangeSetConfigs,
};
use move_binary_format::{
    compatibility::Compatibility,
    errors::{Location, PartialVMError, VMResult},
    CompiledModule,
};
use move_core_types::{
    account_address::AccountAddress,
    identifier::IdentStr,
    language_storage::{ModuleId, TypeTag},
    vm_status::{StatusCode, VMStatus},
};
use move_vm_runtime::{
    data_cache::TransactionDataCache, logging::expect_no_verification_errors,
    module_traversal::TraversalContext, move_vm::SerializedReturnValues,
    native_extensions::NativeContextExtensions, LoadedFunction, ModuleStorage,
};
use move_vm_types::gas::{GasMeter, UnmeteredGasMeter};
use std::borrow::Borrow;

pub trait Session {
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

    fn execute_loaded_function(
        &mut self,
        func: LoadedFunction,
        args: Vec<impl Borrow<[u8]>>,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        module_storage: &impl ModuleStorage,
    ) -> VMResult<SerializedReturnValues>;

    fn extract_publish_request(&mut self) -> Option<PublishRequest>;

    fn mark_unbiasable(&mut self);
}

/// Represents different states of user transaction execution.
enum LegacySessionState<'a> {
    /// Prologue session: used for validation of the transaction. Any failures here result in
    /// transaction being discarded. State changes to [LegacySessionState::UserSession] once the
    /// prologue successfully terminates.
    PrologueSession {
        session: PrologueSession<'a>,
    },
    UserSession {
        session: UserSession<'a>,
        prologue_change_set: SystemSessionChangeSet,
    },
    UserSessionComplete {
        change_set: UserSessionChangeSet,
        prologue_change_set: SystemSessionChangeSet,
    },
    SuccessEpilogue {
        session: EpilogueSession<'a>,
        prologue_change_set: SystemSessionChangeSet,
    },
    SuccessEpilogueFailedMultisig {
        session: EpilogueSession<'a>,
        prologue_change_set: SystemSessionChangeSet,
    },
    Failed {
        prologue_change_set: SystemSessionChangeSet,
    },
    FailureEpilogue {
        session: EpilogueSession<'a>,
    },
}

pub struct LegacySession<'a> {
    state: Option<LegacySessionState<'a>>,
    change_set_configs: &'a ChangeSetConfigs,
}

pub trait TransactionalSession<'a> {
    fn start_prologue_session(
        vm: &AptosVM,
        txn_metadata: &TransactionMetadata,
        resolver: &'a impl AptosMoveResolver,
        change_set_configs: &'a ChangeSetConfigs,
    ) -> Self;

    fn end_prologue_and_start_user_session(
        &mut self,
        vm: &AptosVM,
        txn_meta: &TransactionMetadata,
        resolver: &'a impl AptosMoveResolver,
        module_storage: &impl AptosModuleStorage,
    ) -> Result<(), VMStatus>;
}

impl<'a> TransactionalSession<'a> for LegacySession<'a> {
    fn start_prologue_session(
        vm: &AptosVM,
        txn_metadata: &TransactionMetadata,
        resolver: &'a impl AptosMoveResolver,
        change_set_configs: &'a ChangeSetConfigs,
    ) -> Self {
        let session = PrologueSession::new(vm, txn_metadata, resolver);
        Self {
            state: Some(LegacySessionState::PrologueSession { session }),
            change_set_configs,
        }
    }

    fn end_prologue_and_start_user_session(
        &mut self,
        vm: &AptosVM,
        txn_meta: &TransactionMetadata,
        resolver: &'a impl AptosMoveResolver,
        module_storage: &impl AptosModuleStorage,
    ) -> Result<(), VMStatus> {
        match self.take_state() {
            LegacySessionState::PrologueSession { session } => {
                let (prologue_change_set, session) = session.into_user_session(
                    vm,
                    txn_meta,
                    resolver,
                    self.change_set_configs,
                    module_storage,
                )?;
                self.set_state(LegacySessionState::UserSession {
                    session,
                    prologue_change_set,
                });
                Ok(())
            },
            _ => unreachable!("todo: error"),
        }
    }
}

impl<'a> LegacySession<'a> {
    pub fn end_user_session_without_publish_request(
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
                self.set_state(LegacySessionState::UserSessionComplete {
                    change_set,
                    prologue_change_set,
                });
                Ok(())
            },
            _ => unreachable!("todo: error"),
        }
    }

    pub(crate) fn end_user_session_with_publish_request(
        &mut self,
        resolver: &impl AptosMoveResolver,
        module_storage: &impl AptosModuleStorage,
        gas_meter: &mut impl AptosGasMeter,
        traversal_context: &mut TraversalContext,
        features: &Features,
        gas_feature_version: u64,
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
                    resolver,
                    module_storage,
                    gas_meter,
                    traversal_context,
                    features,
                    gas_feature_version,
                    self.change_set_configs,
                    destination,
                    bundle,
                    modules,
                    compatability_checks,
                )?;
                self.set_state(LegacySessionState::UserSessionComplete {
                    change_set,
                    prologue_change_set,
                });
                Ok(())
            },
            _ => unreachable!("todo: error"),
        }
    }

    pub fn view_change_set(&mut self) -> &mut impl ChangeSetInterface {
        match self.state_mut() {
            LegacySessionState::UserSessionComplete { change_set, .. } => change_set,
            _ => unreachable!("todo: error"),
        }
    }

    pub fn on_user_session_success(
        &mut self,
        vm: &AptosVM,
        txn_meta: &TransactionMetadata,
        resolver: &'a impl AptosMoveResolver,
        storage_refund: Fee,
    ) {
        match self.take_state() {
            LegacySessionState::UserSessionComplete {
                change_set,
                prologue_change_set,
            } => {
                let session = EpilogueSession::on_user_session_success(
                    vm,
                    txn_meta,
                    resolver,
                    change_set,
                    storage_refund,
                );
                self.set_state(LegacySessionState::SuccessEpilogue {
                    session,
                    prologue_change_set,
                });
            },
            _ => unreachable!("todo: error"),
        }
    }

    pub fn get_storage_fee_refund(&self) -> Fee {
        match self.state() {
            LegacySessionState::SuccessEpilogue { session, .. }
            | LegacySessionState::SuccessEpilogueFailedMultisig { session, .. } => {
                session.get_storage_fee_refund()
            },
            LegacySessionState::PrologueSession { .. }
            | LegacySessionState::UserSession { .. }
            | LegacySessionState::UserSessionComplete { .. }
            | LegacySessionState::Failed { .. }
            | LegacySessionState::FailureEpilogue { .. } => unreachable!("todo: error"),
        }
    }

    pub fn end_success_epilogue(
        &mut self,
        fee_statement: FeeStatement,
        module_storage: &impl AptosModuleStorage,
    ) -> Result<(VMStatus, VMOutput), VMStatus> {
        match self.take_state() {
            LegacySessionState::SuccessEpilogue {
                session,
                prologue_change_set,
            }
            | LegacySessionState::SuccessEpilogueFailedMultisig {
                session,
                prologue_change_set,
            } => {
                match session.finish(
                    fee_statement,
                    ExecutionStatus::Success,
                    self.change_set_configs,
                    module_storage,
                ) {
                    Ok(output) => Ok((VMStatus::Executed, output)),
                    Err(status) => {
                        self.state = Some(LegacySessionState::Failed {
                            prologue_change_set,
                        });
                        Err(status)
                    },
                }
            },
            _ => unreachable!("todo: error"),
        }
    }

    pub fn on_user_payload_failure_for_multisig(
        &mut self,
        vm: &AptosVM,
        txn_meta: &TransactionMetadata,
        resolver: &'a impl AptosMoveResolver,
    ) {
        use LegacySessionState::*;
        let prologue_change_set = match self.take_state() {
            UserSession {
                prologue_change_set,
                ..
            } => prologue_change_set,
            PrologueSession { .. }
            | Failed { .. }
            | FailureEpilogue { .. }
            | SuccessEpilogueFailedMultisig { .. }
            | SuccessEpilogue { .. }
            | UserSessionComplete { .. } => {
                unreachable!("todo")
            },
        };

        let session = EpilogueSession::on_user_session_failure(
            vm,
            txn_meta,
            resolver,
            prologue_change_set.clone(),
        );
        self.set_state(SuccessEpilogueFailedMultisig {
            session,
            prologue_change_set,
        })
    }

    pub fn start_failure_epilogue_with_abort_hook(
        &mut self,
        vm: &AptosVM,
        gas_meter: &mut impl AptosGasMeter,
        txn_data: &TransactionMetadata,
        resolver: &'a impl AptosMoveResolver,
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
            | Failed {
                prologue_change_set,
            }
            | SuccessEpilogue {
                prologue_change_set,
                ..
            }
            | SuccessEpilogueFailedMultisig {
                prologue_change_set,
                ..
            } => prologue_change_set,
            _ => unreachable!("todo"),
        };

        // Storage refund is zero since no slots are deleted in aborted transactions.
        const ZERO_STORAGE_REFUND: u64 = 0;

        let should_create_account_resource =
            should_create_account_resource(txn_data, vm.features(), resolver, module_storage)?;

        let (previous_session_change_set, fee_statement) = if should_create_account_resource {
            let mut abort_hook_session =
                AbortHookSession::new(vm, txn_data, resolver, prologue_change_set);

            abort_hook_session.execute(|session| {
                create_account_if_does_not_exist(
                    session,
                    module_storage,
                    gas_meter,
                    txn_data.sender(),
                    traversal_context,
                )
                // If this fails, it is likely due to out of gas, so we try again without metering
                // and then validate below that we charged sufficiently.
                .or_else(|_err| {
                    create_account_if_does_not_exist(
                        session,
                        module_storage,
                        &mut UnmeteredGasMeter,
                        txn_data.sender(),
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
                })
            })?;

            let mut abort_hook_session_change_set =
                abort_hook_session.finish(self.change_set_configs, module_storage)?;
            if let Err(err) = vm.charge_change_set(
                &mut abort_hook_session_change_set,
                gas_meter,
                txn_data,
                resolver,
                module_storage,
            ) {
                info!(
                    *log_context,
                    "Failed during charge_change_set: {:?}. Most likely exceeded gas limited.", err,
                );
            };

            let fee_statement =
                AptosVM::fee_statement_from_gas_meter(txn_data, gas_meter, ZERO_STORAGE_REFUND);

            // Verify we charged sufficiently for creating an account slot
            let gas_params = vm.gas_params(log_context)?;
            let gas_unit_price = u64::from(txn_data.gas_unit_price());
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
            (abort_hook_session_change_set, fee_statement)
        } else {
            let fee_statement =
                AptosVM::fee_statement_from_gas_meter(txn_data, gas_meter, ZERO_STORAGE_REFUND);
            (prologue_change_set, fee_statement)
        };

        let session = EpilogueSession::on_user_session_failure(
            vm,
            txn_data,
            resolver,
            previous_session_change_set,
        );
        self.set_state(FailureEpilogue { session });
        Ok(fee_statement)
    }

    pub fn end_failure_epilogue(
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
            _ => unreachable!("todo: error"),
        }
    }
}

impl<'a> Session for LegacySession<'a> {
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
        self.respawned().expect("todo").execute(|session| {
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
        self.respawned().expect("todo").execute(|session| {
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
            .expect("todo")
            .execute(|session| session.extract_publish_request())
    }

    fn mark_unbiasable(&mut self) {
        self.respawned()
            .expect("todo")
            .execute(|session| session.mark_unbiasable())
    }
}

// Private interfaces.
impl<'a> LegacySession<'a> {
    fn state(&self) -> &LegacySessionState<'a> {
        self.state.as_ref().expect("Session state is always set")
    }

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
            SuccessEpilogue { session, .. }
            | SuccessEpilogueFailedMultisig { session, .. }
            | FailureEpilogue { session } => session,
            UserSessionComplete { .. } | Failed { .. } => {
                return None;
            },
        })
    }
}

#[allow(dead_code)]
pub struct ContinuousSession<'a, DataView> {
    data_cache: TransactionDataCache,
    data_view: &'a DataView,
    extensions: NativeContextExtensions<'a>,
    change_set_configs: &'a ChangeSetConfigs,
}
