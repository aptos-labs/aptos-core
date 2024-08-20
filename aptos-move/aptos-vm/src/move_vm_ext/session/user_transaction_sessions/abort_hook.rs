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
use aptos_vm_types::storage::change_set_configs::ChangeSetConfigs;
use derive_more::{Deref, DerefMut};
use move_binary_format::errors::Location;
use move_core_types::vm_status::VMStatus;

#[derive(Deref, DerefMut)]
pub struct AbortHookSession<'r, 'l> {
    #[deref]
    #[deref_mut]
    session: RespawnedSession<'r, 'l>,
}

impl<'r, 'l> AbortHookSession<'r, 'l> {
    pub fn new(
        vm: &'l AptosVM,
        txn_meta: &'l TransactionMetadata,
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
    ) -> Result<SystemSessionChangeSet, VMStatus> {
        let Self { session } = self;
        let (change_set, empty_module_write_set) =
            session.finish_with_squashed_change_set(change_set_configs, false)?;
        let abort_hook_session_change_set =
            SystemSessionChangeSet::new(change_set, change_set_configs)?;

        // Abort hook can never publish modules (just like epilogue)! When we move publishing
        // outside MoveVM, we do not need to have a check here.
        empty_module_write_set
            .is_empty_or_invariant_violation()
            .map_err(|e| {
                e.with_message("Non-empty module write set in abort hook session".to_string())
                    .finish(Location::Undefined)
                    .into_vm_status()
            })?;

        Ok(abort_hook_session_change_set)
    }
}
