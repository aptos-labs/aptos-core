// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::move_vm_ext::session::{
    respawned_session::RespawnedSession, user_transaction_sessions::Context,
    view_with_change_set::ExecutorViewWithChangeSet,
};
use aptos_vm_types::{change_set::VMChangeSet, storage::change_set_configs::ChangeSetConfigs};
use derive_more::{Deref, DerefMut};
use move_core_types::vm_status::VMStatus;

#[derive(Deref, DerefMut)]
pub struct AbortHookSession<'r, 'l> {
    context: Context<'l>,
    #[deref]
    #[deref_mut]
    session: RespawnedSession<'r, 'l>,
}

impl<'r, 'l> AbortHookSession<'r, 'l> {
    pub fn new(context: Context<'l>, session: RespawnedSession<'r, 'l>) -> Self {
        Self { context, session }
    }

    pub fn finish(
        self,
        change_set_configs: &ChangeSetConfigs,
    ) -> Result<(VMChangeSet, ExecutorViewWithChangeSet<'r>, Context<'l>), VMStatus> {
        let Self { context, session } = self;

        let (change_set, executor_view) = session.finish(change_set_configs)?;
        Ok((change_set, executor_view, context))
    }
}
