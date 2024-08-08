// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    move_vm_ext::{
        session::{
            respawned_session::RespawnedSession,
            user_transaction_sessions::session_change_sets::UserSessionChangeSet,
        },
        AptosMoveResolver, SessionId,
    },
    transaction_metadata::TransactionMetadata,
    AptosVM,
};
use aptos_vm_types::{
    change_set::VMChangeSet, module_and_script_storage::module_storage::AptosModuleStorage,
    module_write_set::ModuleWriteSet, storage::change_set_configs::ChangeSetConfigs,
};
use derive_more::{Deref, DerefMut};
use move_binary_format::errors::Location;
use move_core_types::vm_status::VMStatus;

#[derive(Deref, DerefMut)]
pub struct UserSession<'r, 'l> {
    #[deref]
    #[deref_mut]
    pub session: RespawnedSession<'r, 'l>,
}

impl<'r, 'l> UserSession<'r, 'l> {
    pub fn new(
        vm: &'l AptosVM,
        txn_meta: &'l TransactionMetadata,
        resolver: &'r impl AptosMoveResolver,
        prologue_change_set: VMChangeSet,
    ) -> Self {
        let session_id = SessionId::txn_meta(txn_meta);

        let session = RespawnedSession::spawn(
            vm,
            session_id,
            resolver,
            prologue_change_set,
            Some(txn_meta.as_user_transaction_context()),
        );

        Self { session }
    }

    pub fn legacy_inherit_prologue_session(prologue_session: RespawnedSession<'r, 'l>) -> Self {
        Self {
            session: prologue_session,
        }
    }

    pub fn finish(
        self,
        change_set_configs: &ChangeSetConfigs,
        maybe_module_write_set: Option<ModuleWriteSet>,
        module_storage: &impl AptosModuleStorage,
    ) -> Result<UserSessionChangeSet, VMStatus> {
        let Self { session } = self;
        let (change_set, module_write_set_if_use_loader_v1) =
            session.finish_with_squashed_change_set(change_set_configs, module_storage, false)?;

        let module_write_set = if let Some(module_write_set) = maybe_module_write_set {
            // This means we are using V2 flow, which does not store modules inside the MoveVM.
            module_write_set_if_use_loader_v1
                .is_empty_or_invariant_violation()
                .map_err(|e| e.finish(Location::Undefined).into_vm_status())?;
            module_write_set
        } else {
            module_write_set_if_use_loader_v1
        };

        UserSessionChangeSet::new(change_set, module_write_set, change_set_configs)
    }
}
