// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    aptos_vm_impl::AptosVMImpl,
    data_cache::StorageAdapter,
    move_vm_ext::{SessionExt, SessionId},
};
use anyhow::{bail, Result};
use aptos_gas::ChangeSetConfigs;
use aptos_state_view::{StateView, StateViewId, TStateView};
use aptos_types::{
    state_store::{
        state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
    },
    write_set::TransactionWrite,
};
use aptos_vm_types::change_set::VMChangeSet;
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
    ) -> Result<VMChangeSet, VMStatus> {
        let new_change_set = self.with_session_mut(|session| {
            session.take().unwrap().finish(&mut (), change_set_configs)
        })?;
        let change_set = self.into_heads().state_view.change_set;
        change_set
            .squash(new_change_set, change_set_configs)
            .map_err(|_err| {
                VMStatus::Error(
                    StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                    err_msg("Failed to squash VMChangeSet"),
                )
            })
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
        match self.change_set.delta_change_set().get(state_key) {
            Some(delta_op) => Ok(delta_op
                .try_into_write_op(self.base, state_key)?
                .as_state_value()),
            None => match self.change_set.write_set().get(state_key) {
                Some(write_op) => Ok(write_op.as_state_value()),
                None => self.base.get_state_value(state_key),
            },
        }
    }

    fn is_genesis(&self) -> bool {
        unreachable!("Unexpected access to is_genesis()")
    }

    fn get_usage(&self) -> Result<StateStorageUsage> {
        bail!("Unexpected access to get_usage()")
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use aptos_aggregator::delta_change_set::{delta_add, deserialize, serialize, DeltaChangeSet};
    use aptos_language_e2e_tests::data_store::FakeDataStore;
    use aptos_types::{
        state_store::table::TableHandle,
        write_set::{WriteOp, WriteSetMut},
    };
    use aptos_vm_types::check_change_set::CheckChangeSet;
    use move_core_types::account_address::AccountAddress;

    /// A mock for testing. Always succeeds on checking a change set.
    struct NoOpChangeSetChecker;

    impl CheckChangeSet for NoOpChangeSetChecker {
        fn check_change_set(&self, _change_set: &VMChangeSet) -> anyhow::Result<(), VMStatus> {
            Ok(())
        }
    }

    #[test]
    fn test_change_set_state_view() {
        let key1 = StateKey::table_item(
            TableHandle(AccountAddress::ZERO),
            String::from("test-key1").into_bytes(),
        );
        let key2 = StateKey::table_item(
            TableHandle(AccountAddress::ZERO),
            String::from("test-key2").into_bytes(),
        );
        let key3 = StateKey::raw(String::from("test-key3").into_bytes());

        let mut base_view = FakeDataStore::default();
        base_view.set_legacy(key1.clone(), serialize(&150));
        base_view.set_legacy(key2.clone(), serialize(&300));
        base_view.set_legacy(key3.clone(), serialize(&500));

        let delta_op = delta_add(5, 500);
        let mut delta_change_set = DeltaChangeSet::empty();
        delta_change_set.insert((key1.clone(), delta_op));

        let write_set_ops = [(key2.clone(), WriteOp::Modification(serialize(&400)))];
        let write_set = WriteSetMut::new(write_set_ops.into_iter())
            .freeze()
            .unwrap();
        let change_set =
            VMChangeSet::new(write_set, delta_change_set, vec![], &NoOpChangeSetChecker).unwrap();
        let change_set_state_view = ChangeSetStateView::new(&base_view, change_set).unwrap();

        assert_eq!(
            deserialize(
                change_set_state_view
                    .get_state_value(&key1)
                    .unwrap()
                    .unwrap()
                    .bytes()
            ),
            155
        );
        assert_eq!(
            deserialize(
                change_set_state_view
                    .get_state_value(&key2)
                    .unwrap()
                    .unwrap()
                    .bytes()
            ),
            400
        );
        assert_eq!(
            deserialize(
                change_set_state_view
                    .get_state_value(&key3)
                    .unwrap()
                    .unwrap()
                    .bytes()
            ),
            500
        );
    }
}
