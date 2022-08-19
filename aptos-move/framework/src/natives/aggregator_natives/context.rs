// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_aggregator::{
    aggregator_extension::{extension_error, AggregatorData, AggregatorID, AggregatorState},
    delta_change_set::DeltaOp,
};
use better_any::{Tid, TidAble};
use move_deps::{
    move_binary_format::errors::PartialVMResult,
    move_table_extension::{TableHandle, TableResolver},
};
use std::{cell::RefCell, collections::BTreeMap};

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
    txn_hash: u128,
    pub(crate) resolver: &'a dyn TableResolver,
    pub(crate) aggregator_data: RefCell<AggregatorData>,
}

impl<'a> NativeAggregatorContext<'a> {
    /// Creates a new instance of a native aggregator context. This must be
    /// passed into VM session.
    pub fn new(txn_hash: u128, resolver: &'a dyn TableResolver) -> Self {
        Self {
            txn_hash,
            resolver,
            aggregator_data: Default::default(),
        }
    }

    /// Returns the hash of transaction associated with this context.
    pub fn txn_hash(&self) -> u128 {
        self.txn_hash
    }

    /// Resolves the value as a table item and returns its bytes.
    pub fn resolve_to_bytes(&self, handle: &TableHandle, key: &[u8]) -> PartialVMResult<Vec<u8>> {
        self.resolver
            .resolve_table_entry(handle, key)
            .map_err(|_| extension_error("value to found"))?
            .map_or(Err(extension_error("value to found")), Ok)
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
            let (value, state, limit) = aggregator.into();

            let change = match state {
                AggregatorState::Data => AggregatorChange::Write(value),
                AggregatorState::PositiveDelta => {
                    let delta_op = DeltaOp::Addition { value, limit };
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

#[cfg(test)]
mod test {
    use super::*;
    use claim::assert_matches;
    use move_deps::{
        move_core_types::gas_algebra::InternalGas, move_table_extension::TableOperation,
    };

    fn test_id(key: u128) -> AggregatorID {
        AggregatorID::new(0, key)
    }

    fn test_set_up(context: &NativeAggregatorContext) {
        let mut aggregator_data = context.aggregator_data.borrow_mut();

        // Aggregators with data.
        aggregator_data.create_new_aggregator(test_id(0), 1000);
        aggregator_data.create_new_aggregator(test_id(1), 1000);
        aggregator_data.create_new_aggregator(test_id(2), 1000);

        // Aggregators with delta.
        aggregator_data.get_aggregator(test_id(3), 1000);
        aggregator_data.get_aggregator(test_id(4), 1000);
        aggregator_data.get_aggregator(test_id(5), 10);

        // Different cases of aggregator removal.
        aggregator_data.remove_aggregator(test_id(0));
        aggregator_data.remove_aggregator(test_id(3));
        aggregator_data.remove_aggregator(test_id(6));
    }

    struct EmptyStorage;

    impl TableResolver for EmptyStorage {
        fn resolve_table_entry(
            &self,
            _handle: &TableHandle,
            _key: &[u8],
        ) -> Result<Option<Vec<u8>>, anyhow::Error> {
            Ok(None)
        }

        fn operation_cost(
            &self,
            _op: TableOperation,
            _key_size: usize,
            _val_size: usize,
        ) -> InternalGas {
            1.into()
        }
    }

    #[test]
    fn test_into_change_set() {
        let context = NativeAggregatorContext::new(0, &EmptyStorage);
        test_set_up(&context);

        let AggregatorChangeSet { changes } = context.into_change_set();

        assert!(!changes.contains_key(&test_id(0)));

        assert_matches!(
            changes.get(&test_id(1)).unwrap(),
            AggregatorChange::Write(0)
        );
        assert_matches!(
            changes.get(&test_id(2)).unwrap(),
            AggregatorChange::Write(0)
        );

        assert_matches!(changes.get(&test_id(3)).unwrap(), AggregatorChange::Delete);

        assert_matches!(
            changes.get(&test_id(4)).unwrap(),
            AggregatorChange::Merge(DeltaOp::Addition {
                value: 0,
                limit: 1000
            })
        );
        assert_matches!(
            changes.get(&test_id(5)).unwrap(),
            AggregatorChange::Merge(DeltaOp::Addition {
                value: 0,
                limit: 10
            })
        );

        assert_matches!(changes.get(&test_id(6)).unwrap(), AggregatorChange::Delete);
    }
}
