// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    move_vm_ext::{
        session::{
            respawned_session::RespawnedSession,
            user_transaction_sessions::session_change_sets::SystemSessionChangeSet,
        },
        AptosMoveResolver, SessionId,
    },
    transaction_metadata::TransactionMetadata,
    AptosVM,
};
use aptos_vm_types::{
    module_and_script_storage::module_storage::AptosModuleStorage,
    storage::change_set_configs::ChangeSetConfigs,
};
use derive_more::{Deref, DerefMut};
use move_core_types::vm_status::VMStatus;
use move_vm_runtime::module_traversal::TraversalContext;

#[derive(Deref, DerefMut)]
pub struct AbortHookSession<'r> {
    #[deref]
    #[deref_mut]
    session: RespawnedSession<'r>,
}

impl<'r> AbortHookSession<'r> {
    pub fn new(
        vm: &AptosVM,
        txn_meta: &TransactionMetadata,
        resolver: &'r impl AptosMoveResolver,
        prologue_session_change_set: SystemSessionChangeSet,
    ) -> Self {
        let session_id = SessionId::run_on_abort(txn_meta);

        let session = RespawnedSession::spawn(
            vm,
            session_id,
            resolver,
            prologue_session_change_set.unpack(),
            Some(txn_meta.as_user_transaction_context()),
        );

        Self { session }
    }

    pub fn finish(
        self,
        change_set_configs: &ChangeSetConfigs,
        module_storage: &impl AptosModuleStorage,
        traversal_context: &TraversalContext,
    ) -> Result<SystemSessionChangeSet, VMStatus> {
        let Self { session } = self;
        let change_set = session.finish_with_squashed_change_set(
            change_set_configs,
            module_storage,
            traversal_context,
            false,
        )?;
        let abort_hook_session_change_set =
            SystemSessionChangeSet::new(change_set, change_set_configs)?;

        Ok(abort_hook_session_change_set)
    }
}
