// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    data_cache::StorageAdapter,
    move_vm_ext::{AptosMoveResolver, SessionExt, SessionId},
    storage_adapter::ExecutorViewWithChanges,
    AptosVM,
};
use aptos_gas_algebra::Fee;
use aptos_vm_types::{change_set::VMChangeSet, storage::ChangeSetConfigs};
use move_core_types::vm_status::{err_msg, StatusCode, VMStatus};

/// We finish the session after the user transaction is done running to get the change set and
/// charge gas and storage fee based on it before running storage refunds and the transaction
/// epilogue. The latter needs to see the state view as if the change set is applied on top of
/// the base state view, and this struct implements that.
#[ouroboros::self_referencing]
pub struct RespawnedSession<'r, 'l> {
    executor_view: ExecutorViewWithChanges<'r>,
    #[borrows(executor_view)]
    #[covariant]
    resolver: StorageAdapter<'this, ExecutorViewWithChanges<'r>>,
    #[borrows(resolver)]
    #[not_covariant]
    session: Option<SessionExt<'this, 'l>>,
    pub storage_refund: Fee,
}

impl<'r, 'l> RespawnedSession<'r, 'l> {
    pub fn spawn(
        vm: &'l AptosVM,
        session_id: SessionId,
        base: &'r dyn AptosMoveResolver,
        previous_session_change_set: VMChangeSet,
        storage_refund: Fee,
    ) -> Result<Self, VMStatus> {
        let executor_view =
            ExecutorViewWithChanges::new(base.as_executor_resolver(), previous_session_change_set);

        Ok(RespawnedSessionBuilder {
            executor_view,
            resolver_builder: |executor_view| vm.as_move_resolver(executor_view),
            session_builder: |resolver| Some(vm.0.new_session(resolver, session_id)),
            storage_refund,
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
        let mut change_set = self.into_heads().executor_view.change_set;
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

    pub fn get_storage_fee_refund(&self) -> Fee {
        *self.borrow_storage_refund()
    }
}

// #[cfg(test)]
// mod test {
//     use super::*;
//     use aptos_aggregator::delta_change_set::{delta_add, serialize};
//     use aptos_language_e2e_tests::data_store::FakeDataStore;
//     use aptos_types::write_set::WriteOp;
//     use aptos_vm_types::check_change_set::CheckChangeSet;
//     use std::collections::HashMap;
//
//     /// A mock for testing. Always succeeds on checking a change set.
//     struct NoOpChangeSetChecker;
//
//     impl CheckChangeSet for NoOpChangeSetChecker {
//         fn check_change_set(&self, _change_set: &VMChangeSet) -> anyhow::Result<(), VMStatus> {
//             Ok(())
//         }
//     }
//
//     fn key(s: impl ToString) -> StateKey {
//         StateKey::raw(s.to_string().into_bytes())
//     }
//
//     fn write(v: u128) -> WriteOp {
//         WriteOp::Modification(serialize(&v))
//     }
//
//     fn read(view: &ChangeSetStateView, s: impl ToString) -> u128 {
//         view.get_state_value_u128(&key(s)).unwrap().unwrap()
//     }
//
//     #[test]
//     fn test_change_set_state_view() {
//         let mut base_view = FakeDataStore::default();
//         base_view.set_legacy(key("module_base"), serialize(&10));
//         base_view.set_legacy(key("module_both"), serialize(&20));
//
//         base_view.set_legacy(key("resource_base"), serialize(&30));
//         base_view.set_legacy(key("resource_both"), serialize(&40));
//
//         base_view.set_legacy(key("aggregator_base"), serialize(&50));
//         base_view.set_legacy(key("aggregator_both"), serialize(&60));
//         base_view.set_legacy(key("aggregator_delta_set"), serialize(&70));
//
//         let resource_write_set = HashMap::from([
//             (key("resource_both"), write(80)),
//             (key("resource_write_set"), write(90)),
//         ]);
//
//         let module_write_set = HashMap::from([
//             (key("module_both"), write(100)),
//             (key("module_write_set"), write(110)),
//         ]);
//
//         let aggregator_write_set = HashMap::from([
//             (key("aggregator_both"), write(120)),
//             (key("aggregator_write_set"), write(130)),
//         ]);
//
//         let aggregator_delta_set =
//             HashMap::from([(key("aggregator_delta_set"), delta_add(1, 1000))]);
//
//         let change_set = VMChangeSet::new(
//             resource_write_set,
//             module_write_set,
//             aggregator_write_set,
//             aggregator_delta_set,
//             vec![],
//             &NoOpChangeSetChecker,
//         )
//         .unwrap();
//         let view = ChangeSetStateView::new(&base_view, change_set).unwrap();
//
//         assert_eq!(read(&view, "module_base"), 10);
//         assert_eq!(read(&view, "module_both"), 100);
//         assert_eq!(read(&view, "module_write_set"), 110);
//
//         assert_eq!(read(&view, "resource_base"), 30);
//         assert_eq!(read(&view, "resource_both"), 80);
//         assert_eq!(read(&view, "resource_write_set"), 90);
//
//         assert_eq!(read(&view, "aggregator_base"), 50);
//         assert_eq!(read(&view, "aggregator_both"), 120);
//         assert_eq!(read(&view, "aggregator_write_set"), 130);
//         assert_eq!(read(&view, "aggregator_delta_set"), 71);
//     }
// }
