// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::move_vm_ext::{
    session::{respawned_session::RespawnedSession, user_transaction_sessions::Context},
    SessionExt,
};
use derive_more::{Deref, DerefMut};
use move_core_types::vm_status::VMStatus;

#[derive(Deref, DerefMut)]
pub struct UserSession<'r, 'l> {
    pub context: Context<'l>,
    /// This carries the prologue change set.
    #[deref]
    #[deref_mut]
    pub session: RespawnedSession<'r, 'l>,
}

impl<'r, 'l> UserSession<'r, 'l> {
    pub fn new(context: Context<'l>, session: RespawnedSession<'r, 'l>) -> Self {
        Self { context, session }
    }

    pub fn execute<T, E: Into<VMStatus>>(
        &mut self,
        fun: impl FnOnce(&mut SessionExt) -> Result<T, E>,
    ) -> Result<T, VMStatus> {
        self.session.execute(fun)
    }
}
