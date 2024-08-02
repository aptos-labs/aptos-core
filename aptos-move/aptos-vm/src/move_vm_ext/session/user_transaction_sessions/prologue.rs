// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    move_vm_ext::{
        session::{
            respawned_session::RespawnedSession,
            user_transaction_sessions::{
                session_change_sets::SystemSessionChangeSet, user::UserSession,
            },
        },
        AptosMoveResolver, SessionId,
    },
    transaction_metadata::TransactionMetadata,
    AptosVM,
};
use aptos_vm_types::{change_set::VMChangeSet, storage::change_set_configs::ChangeSetConfigs};
use derive_more::{Deref, DerefMut};
use move_binary_format::errors::Location;
use move_core_types::vm_status::VMStatus;

#[derive(Deref, DerefMut)]
pub struct PrologueSession<'r, 'l> {
    #[deref]
    #[deref_mut]
    session: RespawnedSession<'r, 'l>,
}

impl<'r, 'l> PrologueSession<'r, 'l> {
    pub fn new<'m>(
        vm: &'l AptosVM,
        txn_meta: &'m TransactionMetadata,
        resolver: &'r impl AptosMoveResolver,
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
        vm: &'l AptosVM,
        txn_meta: &'l TransactionMetadata,
        resolver: &'r impl AptosMoveResolver,
        gas_feature_version: u64,
        change_set_configs: &ChangeSetConfigs,
    ) -> Result<(SystemSessionChangeSet, UserSession<'r, 'l>), VMStatus> {
        let Self { session } = self;

        if gas_feature_version >= 1 {
            // Create a new session so that the data cache is flushed.
            // This is to ensure we correctly charge for loading certain resources, even if they
            // have been previously cached in the prologue.
            //
            // TODO(Gas): Do this in a better way in the future, perhaps without forcing the data cache to be flushed.
            // By releasing resource group cache, we start with a fresh slate for resource group
            // cost accounting.

            let (change_set, empty_module_write_set) =
                session.finish_with_squashed_change_set(change_set_configs, false)?;
            let prologue_session_change_set =
                SystemSessionChangeSet::new(change_set.clone(), change_set_configs)?;

            // Prologue can never publish modules! When we move publishing outside MoveVM, we do not
            // need to have this check here, as modules will only be visible in user session.
            empty_module_write_set
                .is_empty_or_invariant_violation()
                .map_err(|e| {
                    e.with_message("Non-empty module write set in prologue session".to_string())
                        .finish(Location::Undefined)
                        .into_vm_status()
                })?;

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
