// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    move_vm_ext::{
        session::{
            respawned_session::RespawnedSession,
            user_transaction_sessions::{abort_hook::AbortHookSession, Context},
        },
        SessionId,
    },
    AptosVM,
};
use aptos_gas_algebra::Fee;
use aptos_vm_types::{change_set::VMChangeSet, storage::change_set_configs::ChangeSetConfigs};
use derive_more::{Deref, DerefMut};
use move_core_types::vm_status::{err_msg, StatusCode, VMStatus};

#[derive(Deref, DerefMut)]
pub struct EpilogueSession<'r, 'l> {
    context: Context<'l>,
    #[deref]
    #[deref_mut]
    session: RespawnedSession<'r, 'l>,
    storage_refund: Fee,
}

impl<'r, 'l> EpilogueSession<'r, 'l> {
    pub fn new(
        context: Context<'l>,
        session: RespawnedSession<'r, 'l>,
        storage_refund: Fee,
    ) -> Self {
        Self {
            context,
            session,
            storage_refund,
        }
    }

    pub fn finish(self, change_set_configs: &ChangeSetConfigs) -> Result<VMChangeSet, VMStatus> {
        let session = self.session;
        let (additional_change_set, user_txn_result_view) = session.finish(change_set_configs)?;

        if additional_change_set.has_creation() {
            // In the epilogue there shouldn't be new slots created, otherwise there's a potential
            // vulnerability like this:
            // 1. slot created by the user
            // 2. another user transaction deletes the slot and claims the refund
            // 3. in the epilogue the same slot gets recreated, and the final write set will have
            //    a ModifyWithMetadata carrying the original metadata
            // 4. user keeps doing the same and repeatedly claim refund out of the slot.
            return Err(VMStatus::error(
                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                err_msg("Unexpected storage allocation after respawning session."),
            ));
        }

        let change_set = if let Some(mut change_set) = user_txn_result_view.change_set {
            change_set
                .squash_additional_change_set(additional_change_set, change_set_configs)
                .map_err(|_err| {
                    VMStatus::error(
                        StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                        err_msg("Failed to squash VMChangeSet"),
                    )
                })?;
            change_set
        } else {
            additional_change_set
        };

        Ok(change_set)
    }

    pub fn get_storage_fee_refund(&self) -> Fee {
        self.storage_refund
    }

    pub fn duplicate(&self, vm: &'l AptosVM) -> Result<Self, VMStatus> {
        Ok(Self {
            context: self.context,
            session: self.session.respawn_at_base(vm)?,
            storage_refund: self.storage_refund,
        })
    }

    /// when transaction aborts, sometimes we run the run_on_abort hook before the epilogue
    /// FIXME(aldenhu): needs better design, maybe PrologueSession returns a PrologueSideEffect instead of EpilogueSession
    pub fn into_abort_hook_session(
        self,
        vm: &'l AptosVM,
    ) -> Result<AbortHookSession<'r, 'l>, VMStatus> {
        let Self {
            context,
            session,
            storage_refund: _,
        } = self;

        Ok(AbortHookSession::new(
            context,
            RespawnedSession::new_with_view(
                vm,
                SessionId::run_on_abort(context.txn_meta),
                session.cancel(),
            ),
        ))
    }
}
