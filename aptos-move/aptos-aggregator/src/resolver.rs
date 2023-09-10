// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    aggregator_extension::AggregatorID,
    delta_change_set::{serialize, DeltaOp},
    module::AGGREGATOR_MODULE,
};
use aptos_types::{
    state_store::state_value::{StateValue, StateValueMetadata},
    write_set::WriteOp,
};
use move_binary_format::errors::Location;
use move_core_types::vm_status::{StatusCode, VMStatus};

/// Defines different ways a value of an aggregator can be resolved in
/// `AggregatorResolver`. The implementation of the trait can use custom
///  logic for different reading modes.
pub enum AggregatorReadMode {
    /// The returned value is guaranteed to be correct.
    Precise,
    /// The returned value is based on speculation or approximation. For
    /// example, while reading and accumulating deltas only some of them
    /// can be taken into account.
    Speculative,
}

/// Returns a value of an aggregator from cache or global storage.
///   - Ok(..)       if aggregator value exists
///   - Err(..)      otherwise.
pub trait TAggregatorView {
    type Key;

    fn get_aggregator_v1_state_value(&self, id: &Self::Key) -> anyhow::Result<Option<StateValue>>;

    fn get_aggregator_v1_value(&self, id: &Self::Key) -> anyhow::Result<u128> {
        let maybe_state_value = self.get_aggregator_v1_state_value(id)?;
        // TODO: consider reviving Option<u128>?
        let u128_bytes = maybe_state_value
            .expect("Aggregator V1 cannot be deleted")
            .into_bytes();
        bcs::from_bytes(&u128_bytes)
            .map_err(|_| anyhow::Error::msg("Failed to deserialize aggregator value to u128"))
    }

    // TODO: Nested options are used due to cyclic dependency. Aggregator crate needs refactoring.
    fn get_aggregator_v1_state_value_metadata(
        &self,
        id: &Self::Key,
    ) -> anyhow::Result<Option<Option<StateValueMetadata>>> {
        let maybe_state_value = self.get_aggregator_v1_state_value(id)?;
        Ok(maybe_state_value.map(StateValue::into_metadata))
    }

    /// Returns a value of an aggregator.
    fn get_aggregator_v2_value(
        &self,
        _id: &Self::Key,
        _mode: AggregatorReadMode,
    ) -> anyhow::Result<u128> {
        unimplemented!("Aggregator V2 is not yet supported")
    }

    /// Returns a unique per-block identifier that can be used when creating a
    /// new aggregator.
    fn generate_aggregator_v2_id(&self) -> Self::Key {
        unimplemented!("ID generation for Aggregator V2 is not yet supported")
    }

    /// Consumes a single delta and tries to materialize it with a given state
    /// key. If materialization succeeds, a write op is produced. Otherwise, an
    /// error VM status is returned.
    // TODO: remove from the trait!
    fn try_convert_aggregator_v2_delta_into_write_op(
        &self,
        id: &Self::Key,
        delta_op: &DeltaOp,
    ) -> anyhow::Result<WriteOp, VMStatus> {
        // In case storage fails to fetch the value, return immediately.
        let base = self
            .get_aggregator_v1_value(id)
            .map_err(|e| VMStatus::error(StatusCode::STORAGE_ERROR, Some(e.to_string())))?;

        // Otherwise we have to apply delta to the storage value.
        delta_op
            .apply_to(base)
            .map_err(|partial_error| {
                // If delta application fails, transform partial VM
                // error into an appropriate VM status.
                partial_error
                    .finish(Location::Module(AGGREGATOR_MODULE.clone()))
                    .into_vm_status()
            })
            .map(|result| WriteOp::Modification(serialize(&result)))
    }
}

pub trait AggregatorResolver: TAggregatorView<Key = AggregatorID> {}

impl<T: TAggregatorView<Key = AggregatorID>> AggregatorResolver for T {}

// Utils to store aggregator values in data store. Here, we
// only care about aggregators which are state items.
#[cfg(any(test, feature = "testing"))]
pub mod test_utils {
    use super::*;
    use crate::{aggregator_extension::AggregatorHandle, delta_change_set::serialize};
    use aptos_types::state_store::{
        state_key::StateKey, state_value::StateValue, table::TableHandle,
    };
    use move_core_types::account_address::AccountAddress;
    use std::collections::HashMap;

    /// Generates a dummy id for aggregator based on the given key. Only used for testing.
    pub fn aggregator_id_for_test(key: u128) -> AggregatorID {
        let bytes: Vec<u8> = [key.to_le_bytes(), key.to_le_bytes()]
            .iter()
            .flat_map(|b| b.to_vec())
            .collect();
        let key = AggregatorHandle(AccountAddress::from_bytes(bytes).unwrap());
        AggregatorID::new(TableHandle(AccountAddress::ZERO), key)
    }

    #[derive(Default)]
    pub struct AggregatorStore(HashMap<StateKey, StateValue>);

    impl AggregatorStore {
        pub fn set_from_id(&mut self, id: AggregatorID, value: u128) {
            self.set_from_state_key(id.into(), value);
        }

        pub fn set_from_state_key(&mut self, state_key: StateKey, value: u128) {
            self.0
                .insert(state_key, StateValue::new_legacy(serialize(&value)));
        }
    }

    impl TAggregatorView for AggregatorStore {
        type Key = AggregatorID;

        fn get_aggregator_v1_state_value(
            &self,
            id: &Self::Key,
        ) -> anyhow::Result<Option<StateValue>> {
            Ok(self.0.get(id.as_state_key()).cloned())
        }
    }
}
