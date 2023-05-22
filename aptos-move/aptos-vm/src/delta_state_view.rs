// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_aggregator::transaction::ChangeSetExt;
use aptos_state_view::{StateViewId, TStateView};
use aptos_types::{
    state_store::{
        state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
    },
    write_set::TransactionWrite,
};

pub struct DeltaStateView<'a, 'b, S> {
    base: &'a S,
    change_set: &'b ChangeSetExt,
}

impl<'a, 'b, S> DeltaStateView<'a, 'b, S> {
    pub fn new(base: &'a S, change_set: &'b ChangeSetExt) -> Self {
        Self { base, change_set }
    }
}

impl<'a, 'b, S> TStateView for DeltaStateView<'a, 'b, S>
where
    S: TStateView<Key = StateKey>,
{
    type Key = StateKey;

    fn id(&self) -> StateViewId {
        self.base.id()
    }

    fn get_state_value(&self, state_key: &StateKey) -> Result<Option<StateValue>> {
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
        self.base.is_genesis()
    }

    fn get_usage(&self) -> Result<StateStorageUsage> {
        // TODO(Gas): Check if this is correct
        self.base.get_usage()
    }
}

#[cfg(test)]
mod test {
    use super::DeltaStateView;
    use aptos_aggregator::{
        delta_change_set::{delta_add, serialize, DeltaChangeSet},
        transaction::ChangeSetExt,
    };
    use aptos_language_e2e_tests::data_store::FakeDataStore;
    use aptos_state_view::TStateView;
    use aptos_types::{
        state_store::{state_key::StateKey, state_value::StateValue, table::TableHandle},
        transaction::{ChangeSet, NoOpChangeSetChecker},
        write_set::{WriteOp, WriteSetMut},
    };
    use move_core_types::account_address::AccountAddress;
    use std::sync::Arc;

    fn state_value_to_int(state_value: StateValue) -> u128 {
        u128::from_be_bytes(
            state_value
                .into_bytes()
                .iter()
                .rev()
                .cloned()
                .collect::<Vec<u8>>()
                .try_into()
                .unwrap(),
        )
    }

    #[test]
    fn test_delta_state_view() {
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
        let change_set = ChangeSet::new(write_set, vec![], &NoOpChangeSetChecker).unwrap();

        let change_set_ext =
            ChangeSetExt::new(delta_change_set, change_set, Arc::new(NoOpChangeSetChecker));
        let delta_state_view: DeltaStateView<FakeDataStore> =
            DeltaStateView::new(&base_view, &change_set_ext);

        assert_eq!(
            state_value_to_int(delta_state_view.get_state_value(&key1).unwrap().unwrap()),
            155
        );
        assert_eq!(
            state_value_to_int(delta_state_view.get_state_value(&key2).unwrap().unwrap()),
            400
        );
        assert_eq!(
            state_value_to_int(delta_state_view.get_state_value(&key3).unwrap().unwrap()),
            500
        );
    }
}
