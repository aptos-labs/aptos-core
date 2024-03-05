// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    data_cache::StorageAdapter,
    errors::unwrap_or_invariant_violation,
    move_vm_ext::{
        session::view_with_change_set::ExecutorViewWithChangeSet, AptosMoveResolver, SessionExt,
        SessionId,
    },
    AptosVM,
};
use aptos_vm_types::{change_set::VMChangeSet, storage::change_set_configs::ChangeSetConfigs};
use move_core_types::vm_status::VMStatus;

/// FIXME(aldenhu): update documentation
#[ouroboros::self_referencing]
pub struct RespawnedSession<'r, 'l> {
    pub executor_view: ExecutorViewWithChangeSet<'r>,
    #[borrows(executor_view)]
    #[covariant]
    resolver: StorageAdapter<'this, ExecutorViewWithChangeSet<'r>>,
    #[borrows(resolver)]
    #[not_covariant]
    /// This has to be an option because session needs to finish() (which consumes itself) before
    /// RespawnedSession destructs.
    session: Option<SessionExt<'this, 'l>>,
}

impl<'r, 'l> RespawnedSession<'r, 'l> {
    pub fn new_with_resolver(
        vm: &'l AptosVM,
        session_id: SessionId,
        resolver: &'r impl AptosMoveResolver,
    ) -> Self {
        let executor_view = ExecutorViewWithChangeSet::new(
            resolver.as_executor_view(),
            resolver.as_resource_group_view(),
            None,
        );

        Self::build(vm, executor_view, session_id)
    }

    pub fn new_with_view(
        vm: &'l AptosVM,
        session_id: SessionId,
        view: ExecutorViewWithChangeSet<'r>,
    ) -> Self {
        Self::build(vm, view, session_id)
    }

    pub fn respawn_at_base(&self, vm: &'l AptosVM) -> Result<Self, VMStatus> {
        let executor_view = self.borrow_executor_view().clone();
        let session_id = self.with_session(|s| {
            Ok::<_, VMStatus>(
                unwrap_or_invariant_violation(s.as_ref(), "Inner session has already finished.")?
                    .session_id()
                    .clone(),
            )
        })?;

        Ok(Self::build(vm, executor_view, session_id))
    }

    fn build(
        vm: &'l AptosVM,
        executor_view: ExecutorViewWithChangeSet<'r>,
        session_id: SessionId,
    ) -> Self {
        RespawnedSessionBuilder {
            executor_view,
            resolver_builder: |executor_view| vm.as_move_resolver_with_group_view(executor_view),
            session_builder: |resolver| Some(vm.new_session(resolver, session_id)),
        }
        .build()
    }

    pub fn execute<T, E: Into<VMStatus>>(
        &mut self,
        fun: impl FnOnce(&mut SessionExt) -> Result<T, E>,
    ) -> Result<T, VMStatus> {
        self.with_session_mut(|session| {
            fun(unwrap_or_invariant_violation(
                session.as_mut(),
                "VM respawned session has to be set for execution.",
            )?)
            .map_err(Into::into)
        })
    }

    /// Finishes the internal session and return the resulting additional change set on top of the
    /// base view passed in on construction.
    pub fn finish(
        mut self,
        change_set_configs: &ChangeSetConfigs,
    ) -> Result<(VMChangeSet, ExecutorViewWithChangeSet<'r>), VMStatus> {
        let additional_change_set = self.with_session_mut(|session| {
            unwrap_or_invariant_violation(
                session.take(),
                "VM session cannot be finished more than once.",
            )?
            .finish(change_set_configs)
            .map_err(|e| e.into_vm_status())
        })?;
        let executor_view = self.into_heads().executor_view;

        Ok((additional_change_set, executor_view))
    }

    pub fn cancel(self) -> ExecutorViewWithChangeSet<'r> {
        self.into_heads().executor_view
    }
}
