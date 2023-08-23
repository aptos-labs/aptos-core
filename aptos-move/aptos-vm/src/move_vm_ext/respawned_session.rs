// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    aptos_vm_impl::AptosVMImpl,
    data_cache::StorageAdapter,
    move_vm_ext::{SessionExt, SessionId},
};
use anyhow::{bail, Result};
use aptos_state_view::{StateView, StateViewId, TStateView};
use aptos_types::{
    state_store::{
        state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
    },
    write_set::TransactionWrite,
};
use aptos_vm_types::{change_set::VMChangeSet, storage::ChangeSetConfigs};
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
        previous_session_change_set: VMChangeSet,
    ) -> Result<Self, VMStatus> {
        let state_view = ChangeSetStateView::new(base_state_view, previous_session_change_set)?;

        Ok(RespawnedSessionBuilder {
            state_view,
            resolver_builder: |state_view| {
                StorageAdapter::new_with_cached_config(
                    state_view,
                    vm.get_gas_feature_version(),
                    vm.get_features(),
                )
            },
            session_builder: |resolver| Some(vm.new_session(resolver, session_id)),
        }
        .build())
    }

    pub fn execute<R>(&mut self, fun: impl FnOnce(&mut SessionExt) -> R) -> R {
        self.with_session_mut(|session| fun(session.as_mut().unwrap()))
    }

    pub fn finish(
        mut self,
        change_set_configs: &ChangeSetConfigs,
    ) -> Result<VMChangeSet, VMStatus> {
        let additional_change_set = self.with_session_mut(|session| {
            session.take().unwrap().finish(&mut (), change_set_configs)
        })?;
        let mut change_set = self.into_heads().state_view.change_set;
        change_set
            .squash_additional_change_set(additional_change_set, change_set_configs)
            .map_err(|_err| {
                VMStatus::error(
                    StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                    err_msg("Failed to squash VMChangeSet"),
                )
            })?;
        Ok(change_set)
    }
}

/// A state view as if a change set is applied on top of the base state view.
struct ChangeSetStateView<'r> {
    base: &'r dyn StateView,
    change_set: VMChangeSet,
}

impl<'r> ChangeSetStateView<'r> {
    pub fn new(base: &'r dyn StateView, change_set: VMChangeSet) -> Result<Self, VMStatus> {
        Ok(Self { base, change_set })
    }
}

impl<'r> TStateView for ChangeSetStateView<'r> {
    type Key = StateKey;

    fn id(&self) -> StateViewId {
        self.base.id()
    }

    fn get_state_value(&self, state_key: &Self::Key) -> Result<Option<StateValue>> {
        // TODO: `get_state_value` should differentiate between different write types.
        match self.change_set.aggregator_v1_delta_set().get(state_key) {
            Some(delta_op) => Ok(delta_op
                .try_into_write_op(self.base, state_key)?
                .as_state_value()),
            None => {
                let cached_value = self
                    .change_set
                    .write_set_iter()
                    .find(|(k, _)| *k == state_key)
                    .map(|(_, v)| v);
                match cached_value {
                    Some(write_op) => Ok(write_op.as_state_value()),
                    None => self.base.get_state_value(state_key),
                }
            },
        }
    }

    fn get_usage(&self) -> Result<StateStorageUsage> {
        bail!("Unexpected access to get_usage()")
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use aptos_aggregator::delta_change_set::{delta_add, deserialize, serialize};
    use aptos_language_e2e_tests::data_store::FakeDataStore;
    use aptos_types::write_set::WriteOp;
    use aptos_vm_types::check_change_set::CheckChangeSet;
    use std::collections::HashMap;

    /// A mock for testing. Always succeeds on checking a change set.
    struct NoOpChangeSetChecker;

    impl CheckChangeSet for NoOpChangeSetChecker {
        fn check_change_set(&self, _change_set: &VMChangeSet) -> anyhow::Result<(), VMStatus> {
            Ok(())
        }
    }

    fn key(s: impl ToString) -> StateKey {
        StateKey::raw(s.to_string().into_bytes())
    }

    fn write(v: u128) -> WriteOp {
        WriteOp::Modification(serialize(&v))
    }

    fn read(view: &ChangeSetStateView, s: impl ToString) -> u128 {
        let bytes = view.get_state_value(&key(s)).unwrap().unwrap().into_bytes();
        deserialize(&bytes)
    }

    #[test]
    fn test_change_set_state_view() {
        let mut base_view = FakeDataStore::default();
        base_view.set_legacy(key("module_base"), serialize(&10));
        base_view.set_legacy(key("module_both"), serialize(&20));

        base_view.set_legacy(key("resource_base"), serialize(&30));
        base_view.set_legacy(key("resource_both"), serialize(&40));

        base_view.set_legacy(key("aggregator_base"), serialize(&50));
        base_view.set_legacy(key("aggregator_both"), serialize(&60));
        base_view.set_legacy(key("aggregator_delta_set"), serialize(&70));

        let resource_write_set = HashMap::from([
            (key("resource_both"), write(80)),
            (key("resource_write_set"), write(90)),
        ]);

        let module_write_set = HashMap::from([
            (key("module_both"), write(100)),
            (key("module_write_set"), write(110)),
        ]);

        let aggregator_write_set = HashMap::from([
            (key("aggregator_both"), write(120)),
            (key("aggregator_write_set"), write(130)),
        ]);

        let aggregator_delta_set =
            HashMap::from([(key("aggregator_delta_set"), delta_add(1, 1000))]);

        let change_set = VMChangeSet::new(
            resource_write_set,
            module_write_set,
            aggregator_write_set,
            aggregator_delta_set,
            vec![],
            &NoOpChangeSetChecker,
        )
        .unwrap();
        let view = ChangeSetStateView::new(&base_view, change_set).unwrap();

        assert_eq!(read(&view, "module_base"), 10);
        assert_eq!(read(&view, "module_both"), 100);
        assert_eq!(read(&view, "module_write_set"), 110);

        assert_eq!(read(&view, "resource_base"), 30);
        assert_eq!(read(&view, "resource_both"), 80);
        assert_eq!(read(&view, "resource_write_set"), 90);

        assert_eq!(read(&view, "aggregator_base"), 50);
        assert_eq!(read(&view, "aggregator_both"), 120);
        assert_eq!(read(&view, "aggregator_write_set"), 130);
        assert_eq!(read(&view, "aggregator_delta_set"), 71);
    }
}
