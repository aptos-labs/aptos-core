// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    move_vm_ext::{
        session::{
            respawned_session::RespawnedSession,
            user_transaction_sessions::session_change_sets::{
                SystemSessionChangeSet, UserSessionChangeSet,
            },
        },
        VelorMoveResolver, SessionId,
    },
    transaction_metadata::TransactionMetadata,
    VelorVM,
};
use velor_gas_algebra::Fee;
use velor_types::{
    fee_statement::FeeStatement,
    transaction::{ExecutionStatus, TransactionStatus},
};
use velor_vm_types::{
    change_set::VMChangeSet, module_and_script_storage::module_storage::VelorModuleStorage,
    module_write_set::ModuleWriteSet, output::VMOutput,
    storage::change_set_configs::ChangeSetConfigs,
};
use derive_more::{Deref, DerefMut};
use move_core_types::vm_status::VMStatus;

#[derive(Deref, DerefMut)]
pub struct EpilogueSession<'r> {
    #[deref]
    #[deref_mut]
    session: RespawnedSession<'r>,
    storage_refund: Fee,
    module_write_set: ModuleWriteSet,
}

impl<'r> EpilogueSession<'r> {
    pub fn on_user_session_success(
        vm: &VelorVM,
        txn_meta: &TransactionMetadata,
        resolver: &'r impl VelorMoveResolver,
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
        vm: &VelorVM,
        txn_meta: &TransactionMetadata,
        resolver: &'r impl VelorMoveResolver,
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
        vm: &VelorVM,
        txn_meta: &TransactionMetadata,
        resolver: &'r impl VelorMoveResolver,
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
        module_storage: &impl VelorModuleStorage,
    ) -> Result<VMOutput, VMStatus> {
        let Self {
            session,
            storage_refund: _,
            module_write_set,
        } = self;

        let change_set =
            session.finish_with_squashed_change_set(change_set_configs, module_storage, true)?;
        let epilogue_session_change_set =
            UserSessionChangeSet::new(change_set, module_write_set, change_set_configs)?;

        let (change_set, module_write_set) = epilogue_session_change_set.unpack();
        Ok(VMOutput::new(
            change_set,
            module_write_set,
            fee_statement,
            TransactionStatus::Keep(execution_status),
        ))
    }
}
