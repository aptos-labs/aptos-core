// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    aptos_vm_impl::AptosVMImpl,
    data_cache::{AsMoveResolver, StorageAdapter},
    move_vm_ext::{SessionExt, SessionId},
};
use anyhow::{bail, Result};
use aptos_aggregator::transaction::ChangeSetExt;
use aptos_gas::ChangeSetConfigs;
use aptos_state_view::{StateView, StateViewId, TStateView};
use aptos_types::{
    state_store::{
        state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
    },
    write_set::{TransactionWrite, WriteSet},
};
use move_core_types::vm_status::{err_msg, StatusCode, VMStatus};

/// We finish the session after the user transaction is done running to get the change set and
/// charge gas and storage fee based on it before running storage refunds and the transaction
/// epilogue. The latter needs to see the state view as if the change set is applied on top of
/// the base state view, and this struct implements that.
#[ouroboros::self_referencing]
pub struct RespawnedSession<'r, 'l> {
    state_view: ChangeSetStateView<'r>,
    #[borrows(state_view)]
    #[covariant]
    resolver: StorageAdapter<'this, ChangeSetStateView<'r>>,
    #[borrows(resolver)]
    #[not_covariant]
    session: Option<SessionExt<'this, 'l>>,
}

impl<'r, 'l> RespawnedSession<'r, 'l> {
    pub fn spawn(
        vm: &'l AptosVMImpl,
        session_id: SessionId,
        base_state_view: &'r dyn StateView,
        previous_session_change_set: ChangeSetExt,
    ) -> Result<Self, VMStatus> {
        let state_view = ChangeSetStateView::new(base_state_view, previous_session_change_set)?;

        Ok(RespawnedSessionBuilder {
            state_view,
            resolver_builder: |state_view| state_view.as_move_resolver(),
            session_builder: |resolver| Some(vm.new_session(resolver, session_id, true)),
        }
        .build())
    }

    pub fn execute<R>(&mut self, fun: impl FnOnce(&mut SessionExt) -> R) -> R {
        self.with_session_mut(|session| fun(session.as_mut().unwrap()))
    }

    pub fn finish(
        mut self,
        change_set_configs: &ChangeSetConfigs,
    ) -> Result<ChangeSetExt, VMStatus> {
        let new_change_set = self.with_session_mut(|session| {
            session.take().unwrap().finish(&mut (), change_set_configs)
        })?;
        let change_set = self.into_heads().state_view.change_set;
        change_set.squash(new_change_set).map_err(|_err| {
            VMStatus::Error(
                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                err_msg("Failed to squash ChangeSetExt"),
            )
        })
    }
}

/// A state view as if a change set is applied on top of the base state view.
struct ChangeSetStateView<'r> {
    base: &'r dyn StateView,
    change_set: ChangeSetExt,
    materialized_delta_change_set: WriteSet,
}

impl<'r> ChangeSetStateView<'r> {
    pub fn new(base: &'r dyn StateView, change_set: ChangeSetExt) -> Result<Self, VMStatus> {
        // TODO: at this point we know that delta application failed
        // (and it should have occurred in user transaction in general).
        // We need to rerun the epilogue and charge gas. Currently, the use
        // case of an aggregator is for gas fees (which are computed in
        // the epilogue), and therefore this should never happen.
        // Also, it is worth mentioning that current VM error handling is
        // rather ugly and has a lot of legacy code. This makes proper error
        // handling quite challenging.
        let materialized_delta_change_set = change_set
            .delta_change_set
            .clone()
            .try_into_write_set(base)?;
        Ok(Self {
            base,
            change_set,
            materialized_delta_change_set,
        })
    }
}

impl<'r> TStateView for ChangeSetStateView<'r> {
    type Key = StateKey;

    fn id(&self) -> StateViewId {
        self.base.id()
    }

    fn get_state_value(&self, state_key: &Self::Key) -> Result<Option<StateValue>> {
        if let Some(write_op) = self.materialized_delta_change_set.get(state_key) {
            Ok(write_op.as_state_value())
        } else if let Some(write_op) = self.change_set.write_set().get(state_key) {
            Ok(write_op.as_state_value())
        } else {
            self.base.get_state_value(state_key)
        }
    }

    fn is_genesis(&self) -> bool {
        unreachable!("Unexpected access to is_genesis()")
    }

    fn get_usage(&self) -> Result<StateStorageUsage> {
        bail!("Unexpected access to get_usage()")
    }
}
