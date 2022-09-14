// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_aggregator::{
    aggregator_extension::{AggregatorData, AggregatorID, AggregatorState},
    delta_change_set::{DeltaOp, DeltaUpdate},
};
use aptos_types::vm_status::VMStatus;
use better_any::{Tid, TidAble};
use move_deps::{move_binary_format::errors::Location, move_table_extension::TableResolver};
use std::{
    cell::RefCell,
    collections::{btree_map, BTreeMap},
};

/// Represents a single aggregator change.
#[derive(Copy, Clone, Debug)]
pub enum AggregatorChange {
    // A value should be written to storage.
    Write(u128),
    // A delta should be merged with the value from storage.
    Merge(DeltaOp),
    // A value should be deleted from the storage.
    Delete,
}

/// Represents changes made by all aggregators during this context. This change
/// set can be converted into appropriate `WriteSet` and `DeltaChangeSet` by the
/// user, e.g. VM session.
pub struct AggregatorChangeSet {
    pub changes: BTreeMap<AggregatorID, AggregatorChange>,
}

/// Native context that can be attached to VM `NativeContextExtensions`.
///
/// Note: table resolver is reused for fine-grained storage access.
#[derive(Tid)]
pub struct NativeAggregatorContext<'a> {
    txn_hash: [u8; 32],
    pub(crate) resolver: &'a dyn TableResolver,
    pub(crate) aggregator_data: RefCell<AggregatorData>,
}

impl<'a> NativeAggregatorContext<'a> {
    /// Creates a new instance of a native aggregator context. This must be
    /// passed into VM session.
    pub fn new(txn_hash: [u8; 32], resolver: &'a dyn TableResolver) -> Self {
        Self {
            txn_hash,
            resolver,
            aggregator_data: Default::default(),
        }
    }

    /// Returns the hash of transaction associated with this context.
    pub fn txn_hash(&self) -> [u8; 32] {
        self.txn_hash
    }

    /// Returns all changes made within this context (i.e. by a single
    /// transaction).
    pub fn into_change_set(self) -> AggregatorChangeSet {
        let NativeAggregatorContext {
            aggregator_data, ..
        } = self;
        let (_, destroyed_aggregators, aggregators) = aggregator_data.into_inner().into();

        let mut changes = BTreeMap::new();

        // First, process all writes and deltas.
        for (id, aggregator) in aggregators {
            let (value, state, limit, history) = aggregator.into();

            let change = match state {
                AggregatorState::Data => AggregatorChange::Write(value),
                AggregatorState::PositiveDelta => {
                    let history = history.unwrap();
                    let plus = DeltaUpdate::Plus(value);
                    let delta_op =
                        DeltaOp::new(plus, limit, history.max_positive, history.min_negative);
                    AggregatorChange::Merge(delta_op)
                }
                AggregatorState::NegativeDelta => {
                    let history = history.unwrap();
                    let minus = DeltaUpdate::Minus(value);
                    let delta_op =
                        DeltaOp::new(minus, limit, history.max_positive, history.min_negative);
                    AggregatorChange::Merge(delta_op)
                }
            };
            changes.insert(id, change);
        }

        // Additionally, do not forget to delete destroyed values from storage.
        for id in destroyed_aggregators {
            changes.insert(id, AggregatorChange::Delete);
        }

        AggregatorChangeSet { changes }
    }
}

impl AggregatorChangeSet {
    pub fn squash(&mut self, other: Self) -> Result<(), VMStatus> {
        for (other_id, other_change) in other.changes {
            match self.changes.entry(other_id) {
                // If something was changed only in `other` session, add it.
                btree_map::Entry::Vacant(entry) => {
                    entry.insert(other_change);
                }
                // Otherwise, we might need to aggregate deltas.
                btree_map::Entry::Occupied(mut entry) => {
                    use AggregatorChange::*;

                    let entry_mut = entry.get_mut();
                    match (*entry_mut, other_change) {
                        (Write(_) | Merge(_), Write(data)) => *entry_mut = Write(data),
                        (Write(_) | Merge(_), Delete) => *entry_mut = Delete,
                        (Write(data), Merge(delta)) => {
                            let new_data = delta
                                .apply_to(data)
                                .map_err(|e| e.finish(Location::Undefined).into_vm_status())?;
                            *entry_mut = Write(new_data);
                        }
                        (Merge(mut delta1), Merge(delta2)) => {
                            delta1
                                .merge_with(delta2)
                                .map_err(|e| e.finish(Location::Undefined).into_vm_status())?;
                            *entry_mut = Merge(delta1)
                        }
                        // Hashing properties guarantee that aggregator keys should
                        // not collide, making this case impossible.
                        (Delete, _) => unreachable!("resource cannot be accessed after deletion"),
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use aptos_aggregator::aggregator_extension::AggregatorHandle;
    use aptos_types::account_address::AccountAddress;
    use claims::assert_matches;
    use move_deps::move_table_extension::TableHandle;

    struct EmptyStorage;

    impl TableResolver for EmptyStorage {
        fn resolve_table_entry(
            &self,
            _handle: &TableHandle,
            _key: &[u8],
        ) -> Result<Option<Vec<u8>>, anyhow::Error> {
            Ok(None)
        }
    }

    fn test_id(key: u128) -> AggregatorID {
        let bytes: Vec<u8> = [key.to_le_bytes(), key.to_le_bytes()]
            .iter()
            .flat_map(|b| b.to_vec())
            .collect();
        let key = AggregatorHandle(AccountAddress::from_bytes(&bytes).unwrap());
        AggregatorID::new(TableHandle(AccountAddress::ZERO), key)
    }

    // All aggregators are initialized deterministically based on their ID,
    // with the follwing spec.
    //
    //     +-------+---------------+----------+-----+---------+
    //     |  key  | storage value |  create  | get | remove  |
    //     +-------+---------------+----------+-----+---------+
    //     |  100  |               |   yes    | yes |   yes   |
    //     |  200  |               |   yes    | yes |         |
    //     |  300  |               |   yes    |     |   yes   |
    //     |  400  |               |   yes    |     |         |
    //     |  500  |               |          | yes |   yes   |
    //     |  600  |      300      |          | yes |         |
    //     |  700  |               |          | yes |         |
    //     |  800  |               |          |     |   yes   |
    //     +-------+---------------+----------+-----+---------+
    fn test_set_up(context: &NativeAggregatorContext) {
        let mut aggregator_data = context.aggregator_data.borrow_mut();

        aggregator_data.create_new_aggregator(test_id(100), 100);
        aggregator_data.create_new_aggregator(test_id(200), 200);
        aggregator_data.create_new_aggregator(test_id(300), 300);
        aggregator_data.create_new_aggregator(test_id(400), 400);

        aggregator_data.get_aggregator(test_id(100), 100);
        aggregator_data.get_aggregator(test_id(200), 200);
        aggregator_data.get_aggregator(test_id(500), 500);
        aggregator_data.get_aggregator(test_id(600), 600);
        aggregator_data.get_aggregator(test_id(700), 700);

        aggregator_data.remove_aggregator(test_id(100));
        aggregator_data.remove_aggregator(test_id(300));
        aggregator_data.remove_aggregator(test_id(500));
        aggregator_data.remove_aggregator(test_id(800));
    }

    #[test]
    fn test_into_change_set() {
        let context = NativeAggregatorContext::new([0; 32], &EmptyStorage);
        use AggregatorChange::*;

        test_set_up(&context);
        let AggregatorChangeSet { changes } = context.into_change_set();

        assert!(!changes.contains_key(&test_id(100)));
        assert_matches!(changes.get(&test_id(200)).unwrap(), Write(0));
        assert!(!changes.contains_key(&test_id(300)));
        assert_matches!(changes.get(&test_id(400)).unwrap(), Write(0));
        assert_matches!(changes.get(&test_id(500)).unwrap(), Delete);
        assert!(changes.contains_key(&test_id(600)));
        assert!(changes.contains_key(&test_id(700)));
        assert_matches!(changes.get(&test_id(800)).unwrap(), Delete);
    }
}
