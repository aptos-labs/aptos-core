// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::aggregator_extension::AggregatorID;

/// Defines different ways a value of an aggregator can be resolved in
/// `AggregatorResolver`. The implementation of the trait can use custom
///  logic for different reading modes.
pub enum AggregatorReadMode {
    /// The returned value is guaranteed to be correct.
    Aggregated,
    /// The returned value is based on last committed value, ignoring
    /// any pending changes.
    LastCommitted,
}

/// Returns a value of an aggregator from cache or global storage.
///   - Ok(..)       if aggregator value exists
///   - Err(..)      otherwise.
pub trait AggregatorResolver {
    /// Returns a value of an aggregator.
    fn resolve_aggregator_value(
        &self,
        id: &AggregatorID,
        mode: AggregatorReadMode,
    ) -> Result<u128, anyhow::Error>;

    /// Returns a unique per-block identifier that can be used when creating a
    /// new aggregator.
    fn generate_aggregator_id(&self) -> AggregatorID;
}

// Utils to store aggregator values in data store. Here, we
// only care about aggregators which are state items.
#[cfg(any(test, feature = "testing"))]
pub mod test_utils {
    use super::*;
    use crate::{aggregator_extension::AggregatorHandle, delta_change_set::serialize};
    use aptos_state_view::TStateView;
    use aptos_types::state_store::{
        state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
        table::TableHandle,
    };
    use move_core_types::account_address::AccountAddress;
    use std::collections::HashMap;

    /// Generates a dummy id for aggregator based on the given key. Only used for testing.
    pub fn aggregator_v1_id_for_test(key: u128) -> AggregatorID {
        let bytes: Vec<u8> = [key.to_le_bytes(), key.to_le_bytes()]
            .iter()
            .flat_map(|b| b.to_vec())
            .collect();
        let key = AggregatorHandle(AccountAddress::from_bytes(bytes).unwrap());
        AggregatorID::legacy(TableHandle(AccountAddress::ZERO), key)
    }

    #[derive(Default)]
    pub struct AggregatorStore {
        v1_store: HashMap<StateKey, StateValue>,
        v2_store: HashMap<AggregatorID, StateValue>,
    }

    impl AggregatorStore {
        pub fn set_from_id(&mut self, id: AggregatorID, value: u128) {
            match id {
                AggregatorID::Legacy { .. } => {
                    let state_key = id
                        .as_state_key()
                        .expect("Should be able to extract state key for aggregator v1");
                    self.set_from_state_key(state_key, value);
                },
                AggregatorID::Ephemeral(_) => self.set_from_ephemeral_id(id, value),
            }
        }

        pub fn set_from_state_key(&mut self, state_key: StateKey, value: u128) {
            self.v1_store
                .insert(state_key, StateValue::new_legacy(serialize(&value)));
        }

        pub fn set_from_ephemeral_id(&mut self, aggregator_id: AggregatorID, value: u128) {
            self.v2_store
                .insert(aggregator_id, StateValue::new_legacy(serialize(&value)));
        }
    }

    impl AggregatorResolver for AggregatorStore {
        fn resolve_aggregator_value(
            &self,
            id: &AggregatorID,
            _mode: AggregatorReadMode,
        ) -> Result<u128, anyhow::Error> {
            match id {
                AggregatorID::Legacy { .. } => {
                    let state_key = id
                        .as_state_key()
                        .expect("Should be able to extract state key for aggregator v1");
                    match self.get_state_value_u128(&state_key)? {
                        Some(value) => Ok(value),
                        None => {
                            anyhow::bail!("Could not find the value of the aggregator")
                        },
                    }
                },
                AggregatorID::Ephemeral(_) => {
                    match self.v2_store.get(id).map(|val| val.bytes()) {
                        Some(bytes) => Ok(bcs::from_bytes(bytes)?),
                        None => {
                            anyhow::bail!("Could not find the value of the aggregator")
                        },
                    }
                },
            }
        }

        fn generate_aggregator_id(&self) -> AggregatorID {
            unimplemented!("Aggregator id generation will be implemented for V2 aggregators.")
        }
    }

    impl TStateView for AggregatorStore {
        type Key = StateKey;

        fn get_state_value(&self, state_key: &Self::Key) -> anyhow::Result<Option<StateValue>> {
            Ok(self.v1_store.get(state_key).cloned())
        }

        fn get_usage(&self) -> anyhow::Result<StateStorageUsage> {
            let mut usage = StateStorageUsage::new_untracked();
            for (k, v) in self.v1_store.iter() {
                usage.add_item(k.size() + v.size())
            }
            Ok(usage)
        }
    }
}
