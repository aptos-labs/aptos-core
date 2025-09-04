// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    move_vm_ext::{
        session::{
            respawned_session::RespawnedSession,
            user_transaction_sessions::{
                session_change_sets::SystemSessionChangeSet, user::UserSession,
            },
        },
        VelorMoveResolver, SessionId,
    },
    transaction_metadata::TransactionMetadata,
    VelorVM,
};
use velor_vm_types::{
    change_set::VMChangeSet, module_and_script_storage::module_storage::VelorModuleStorage,
    storage::change_set_configs::ChangeSetConfigs,
};
use derive_more::{Deref, DerefMut};
use move_core_types::vm_status::VMStatus;

#[derive(Deref, DerefMut)]
pub struct PrologueSession<'r> {
    #[deref]
    #[deref_mut]
    session: RespawnedSession<'r>,
}

impl<'r> PrologueSession<'r> {
    pub fn new(
        vm: &VelorVM,
        txn_meta: &TransactionMetadata,
        resolver: &'r impl VelorMoveResolver,
    ) -> Self {
        let session_id = SessionId::prologue_meta(txn_meta);
        let session = RespawnedSession::spawn(
            vm,
            session_id,
            resolver,
            VMChangeSet::empty(),
            Some(txn_meta.as_user_transaction_context()),
        );

        Self { session }
    }

    pub fn into_user_session(
        self,
        vm: &VelorVM,
        txn_meta: &TransactionMetadata,
        resolver: &'r impl VelorMoveResolver,
        change_set_configs: &ChangeSetConfigs,
        module_storage: &impl VelorModuleStorage,
    ) -> Result<(SystemSessionChangeSet, UserSession<'r>), VMStatus> {
        let Self { session } = self;

        if vm.gas_feature_version() >= 1 {
            // Create a new session so that the data cache is flushed.
            // This is to ensure we correctly charge for loading certain resources, even if they
            // have been previously cached in the prologue.
            //
            // TODO(Gas): Do this in a better way in the future, perhaps without forcing the data cache to be flushed.
            // By releasing resource group cache, we start with a fresh slate for resource group
            // cost accounting.

            let change_set = session.finish_with_squashed_change_set(
                change_set_configs,
                module_storage,
                false,
            )?;
            let prologue_session_change_set =
                SystemSessionChangeSet::new(change_set.clone(), change_set_configs)?;

            resolver.release_resource_group_cache();
            Ok((
                prologue_session_change_set,
                UserSession::new(vm, txn_meta, resolver, change_set),
            ))
        } else {
            Ok((
                SystemSessionChangeSet::empty(),
                UserSession::legacy_inherit_prologue_session(session),
            ))
        }
    }
}
