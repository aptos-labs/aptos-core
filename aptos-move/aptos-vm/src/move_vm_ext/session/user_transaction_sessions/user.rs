// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    move_vm_ext::{session::respawned_session::RespawnedSession, AptosMoveResolver, SessionId},
    transaction_metadata::TransactionMetadata,
    AptosVM,
};
use aptos_vm_types::{change_set::VMChangeSet, storage::change_set_configs::ChangeSetConfigs};
use derive_more::{Deref, DerefMut};
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
    ) -> Result<Self, VMStatus> {
        let session_id = SessionId::txn_meta(txn_meta);

        let session = RespawnedSession::spawn(
            vm,
            session_id,
            resolver,
            prologue_change_set,
            Some(txn_meta.as_user_transaction_context()),
        )?;

        Ok(Self { session })
    }

    pub fn legacy_inherit_prologue_session(prologue_session: RespawnedSession<'r, 'l>) -> Self {
        Self {
            session: prologue_session,
        }
    }

    pub fn finish(self, change_set_configs: &ChangeSetConfigs) -> Result<VMChangeSet, VMStatus> {
        let Self { session } = self;
        session.finish_with_squashed_change_set(change_set_configs, false)
    }
}
