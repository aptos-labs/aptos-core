// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    move_vm_ext::{
        session::{
            respawned_session::RespawnedSession,
            user_transaction_sessions::{epilogue::EpilogueSession, user::UserSession, Context},
        },
        AptosMoveResolver, SessionId,
    },
    transaction_metadata::TransactionMetadata,
    AptosVM,
};
use aptos_gas_algebra::Fee;
use aptos_vm_types::storage::change_set_configs::ChangeSetConfigs;
use derive_more::{Deref, DerefMut};
use move_core_types::vm_status::VMStatus;

#[derive(Deref, DerefMut)]
pub struct PrologueSession<'r, 'l> {
    context: Context<'l>,
    #[deref]
    #[deref_mut]
    session: RespawnedSession<'r, 'l>,
}

impl<'r, 'l> PrologueSession<'r, 'l> {
    pub fn new(
        vm: &'l AptosVM,
        txn_meta: &'l TransactionMetadata,
        resolver: &'r impl AptosMoveResolver,
    ) -> Self {
        let context = Context { txn_meta };
        let session_id = SessionId::prologue_meta(context.txn_meta);
        let session = RespawnedSession::new_with_resolver(vm, session_id, resolver);

        Self { context, session }
    }

    pub fn into_succeeding_sessions(
        self,
        vm: &'l AptosVM,
        gas_feature_version: u64,
        change_set_configs: &ChangeSetConfigs,
    ) -> Result<(EpilogueSession<'r, 'l>, UserSession<'r, 'l>), VMStatus> {
        let Self {
            context,
            session: prologue_session,
        } = self;

        let epilogue_session_id = SessionId::epilogue_meta(context.txn_meta);

        let (e, u) = if gas_feature_version >= 1 {
            // Create a new session so that the data cache is flushed.
            // This is to ensure we correctly charge for loading certain resources, even if they
            // have been previously cached in the prologue.
            //
            // TODO(Gas): Do this in a better way in the future, perhaps without forcing the data cache to be flushed.
            // By releasing resource group cache, we start with a fresh slate for resource group
            // cost accounting.

            let (change_set, prologue_base_view) = prologue_session.finish(change_set_configs)?;

            (
                EpilogueSession::new(
                    context,
                    RespawnedSession::new_with_view(
                        vm,
                        epilogue_session_id,
                        prologue_base_view.plus_change_set(change_set.clone()),
                    ),
                    Fee::zero(),
                ),
                UserSession::new(
                    context,
                    RespawnedSession::new_with_view(
                        vm,
                        SessionId::txn_meta(context.txn_meta),
                        prologue_base_view.plus_change_set(change_set),
                    ),
                ),
            )
        } else {
            (
                EpilogueSession::new(
                    context,
                    RespawnedSession::new_with_view(
                        vm,
                        epilogue_session_id,
                        prologue_session
                            .borrow_executor_view()
                            .assert_no_change_set()
                            .clone(),
                    ),
                    Fee::zero(),
                ),
                UserSession::new(context, prologue_session),
            )
        };
        Ok((e, u))
    }
}
