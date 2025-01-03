// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    move_vm_ext::{
        session::{
            respawned_session::RespawnedSession,
            user_transaction_sessions::session_change_sets::{
                SystemSessionChangeSet, UserSessionChangeSet,
            },
        },
        AptosMoveResolver, SessionId,
    },
    transaction_metadata::TransactionMetadata,
    AptosVM,
};
use aptos_gas_algebra::Fee;
use aptos_types::{
    fee_statement::FeeStatement,
    transaction::{ExecutionStatus, TransactionStatus},
};
use aptos_vm_types::{
    change_set::VMChangeSet, module_and_script_storage::module_storage::AptosModuleStorage,
    module_write_set::ModuleWriteSet, output::VMOutput,
    storage::change_set_configs::ChangeSetConfigs,
};
use derive_more::{Deref, DerefMut};
use move_binary_format::errors::Location;
use move_core_types::vm_status::VMStatus;

#[derive(Deref, DerefMut)]
pub struct EpilogueSession<'r, 'l> {
    #[deref]
    #[deref_mut]
    session: RespawnedSession<'r, 'l>,
    storage_refund: Fee,
    module_write_set: ModuleWriteSet,
}

impl<'r, 'l> EpilogueSession<'r, 'l> {
    pub fn on_user_session_success(
        vm: &'l AptosVM,
        txn_meta: &'l TransactionMetadata,
        resolver: &'r impl AptosMoveResolver,
        user_session_change_set: UserSessionChangeSet,
        storage_refund: Fee,
    ) -> Self {
        let (change_set, module_write_set) = user_session_change_set.unpack();
        Self::new(
            vm,
            txn_meta,
            resolver,
            change_set,
            module_write_set,
            storage_refund,
        )
    }

    pub fn on_user_session_failure(
        vm: &'l AptosVM,
        txn_meta: &'l TransactionMetadata,
        resolver: &'r impl AptosMoveResolver,
        previous_session_change_set: SystemSessionChangeSet,
    ) -> Self {
        Self::new(
            vm,
            txn_meta,
            resolver,
            previous_session_change_set.unpack(),
            ModuleWriteSet::empty(),
            0.into(),
        )
    }

    fn new(
        vm: &'l AptosVM,
        txn_meta: &'l TransactionMetadata,
        resolver: &'r impl AptosMoveResolver,
        previous_session_change_set: VMChangeSet,
        module_write_set: ModuleWriteSet,
        storage_refund: Fee,
    ) -> Self {
        let session_id = SessionId::epilogue_meta(txn_meta);
        let session = RespawnedSession::spawn(
            vm,
            session_id,
            resolver,
            previous_session_change_set,
            Some(txn_meta.as_user_transaction_context()),
        );

        Self {
            session,
            storage_refund,
            module_write_set,
        }
    }

    pub fn get_storage_fee_refund(&self) -> Fee {
        self.storage_refund
    }

    pub fn finish(
        self,
        fee_statement: FeeStatement,
        execution_status: ExecutionStatus,
        change_set_configs: &ChangeSetConfigs,
        module_storage: &impl AptosModuleStorage,
    ) -> Result<VMOutput, VMStatus> {
        let Self {
            session,
            storage_refund: _,
            module_write_set,
        } = self;

        let (change_set, empty_module_write_set) =
            session.finish_with_squashed_change_set(change_set_configs, module_storage, true)?;
        let epilogue_session_change_set =
            UserSessionChangeSet::new(change_set, module_write_set, change_set_configs)?;

        // Epilogue can never publish modules! When we move publishing outside MoveVM, we do not need to have
        // this check here, as modules will only be visible in user session.
        empty_module_write_set
            .is_empty_or_invariant_violation()
            .map_err(|e| {
                e.with_message("Non-empty module write set in epilogue session".to_string())
                    .finish(Location::Undefined)
                    .into_vm_status()
            })?;

        let (change_set, module_write_set) = epilogue_session_change_set.unpack();
        Ok(VMOutput::new(
            change_set,
            module_write_set,
            fee_statement,
            TransactionStatus::Keep(execution_status),
        ))
    }
}
