// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    delta_change_set::{serialize, DeltaOp},
    module::AGGREGATOR_MODULE,
};
use aptos_types::{
    aggregator::AggregatorID,
    state_store::{
        state_key::StateKey,
        state_value::{StateValue, StateValueMetadataKind},
    },
    write_set::WriteOp,
};
use move_binary_format::errors::Location;
use move_core_types::vm_status::{StatusCode, VMStatus};

/// Defines different ways `AggregatorResolver` can be used to read its value
/// from the state.
pub enum AggregatorReadMode {
    /// The returned value is guaranteed to be correct.
    Precise,
    /// The returned value is based on speculation or an approximation. For
    /// example, while reading and accumulating deltas only some of them can be
    /// taken into account.
    Speculative,
}

/// Allows to query aggregator values from the state storage.
/// Because there are two types of aggregators in the system, V1 and V2, we use
/// different code paths for each.
pub trait TAggregatorView {
    // We differentiate between two possible ways to identify an aggregator in
    // storage for now (V1 or V2) so that the APIs are completely separate and
    // we can delete all V1 code when necessary.
    type IdentifierV1;
    type IdentifierV2;

    /// Aggregator V1 is implemented as a state item, and therefore the API has
    /// the same pattern as for modules or resources:
    ///   -  Ok(None)         if aggregator value is not in storage,
    ///   -  Ok(Some(...))    if aggregator value exists in storage,
    ///   -  Err(...)         otherwise (e.g. storage error or failed delta
    ///                       application).
    fn get_aggregator_v1_state_value(
        &self,
        id: &Self::IdentifierV1,
        mode: AggregatorReadMode,
    ) -> anyhow::Result<Option<StateValue>>;

    fn get_aggregator_v1_value(
        &self,
        id: &Self::IdentifierV1,
        mode: AggregatorReadMode,
    ) -> anyhow::Result<Option<u128>> {
        let maybe_state_value = self.get_aggregator_v1_state_value(id, mode)?;
        match maybe_state_value {
            Some(state_value) => Ok(Some(bcs::from_bytes(state_value.bytes())?)),
            None => Ok(None),
        }
    }

    /// Because aggregator V1 is a state item, it also can have metadata (for
    /// example used to calculate storage refunds).
    fn get_aggregator_v1_state_value_metadata(
        &self,
        id: &Self::IdentifierV1,
    ) -> anyhow::Result<Option<StateValueMetadataKind>> {
        // When getting state value metadata for aggregator V1, we need to do a
        // precise read.
        let maybe_state_value =
            self.get_aggregator_v1_state_value(id, AggregatorReadMode::Precise)?;
        Ok(maybe_state_value.map(StateValue::into_metadata))
    }

    fn get_aggregator_v2_value(
        &self,
        _id: &Self::IdentifierV2,
        _mode: AggregatorReadMode,
    ) -> anyhow::Result<u128> {
        unimplemented!("Aggregator V2 is not yet supported")
    }

    /// Returns a unique per-block identifier that can be used when creating a
    /// new aggregator V2.
    fn generate_aggregator_v2_id(&self) -> Self::IdentifierV2 {
        unimplemented!("ID generation for Aggregator V2 is not yet supported")
    }

    /// Consumes a single delta of aggregator V1, and tries to materialize it
    /// with a given identifier (state key). If materialization succeeds, a
    /// write op is produced.
    fn try_convert_aggregator_v1_delta_into_write_op(
        &self,
        id: &Self::IdentifierV1,
        delta_op: &DeltaOp,
        mode: AggregatorReadMode,
    ) -> anyhow::Result<WriteOp, VMStatus> {
        let base = self
            .get_aggregator_v1_value(id, mode)
            .map_err(|e| VMStatus::error(StatusCode::STORAGE_ERROR, Some(e.to_string())))?
            .ok_or_else(|| {
                VMStatus::error(
                    StatusCode::STORAGE_ERROR,
                    Some("Cannot convert delta for deleted aggregator".to_string()),
                )
            })?;
        delta_op
            .apply_to(base)
            .map_err(|partial_error| {
                partial_error
                    .finish(Location::Module(AGGREGATOR_MODULE.clone()))
                    .into_vm_status()
            })
            .map(|result| WriteOp::Modification(serialize(&result).into()))
    }
}

pub trait AggregatorResolver:
    TAggregatorView<IdentifierV1 = StateKey, IdentifierV2 = AggregatorID>
{
}

impl<T: TAggregatorView<IdentifierV1 = StateKey, IdentifierV2 = AggregatorID>> AggregatorResolver
    for T
{
}

// Utils to store aggregator values in data store. Here, we
// only care about aggregators which are state items (V1).
#[cfg(any(test, feature = "testing"))]
pub mod test_utils {
    use super::*;
    use crate::delta_change_set::serialize;
    use aptos_types::{
        aggregator::AggregatorHandle,
        state_store::{state_key::StateKey, state_value::StateValue, table::TableHandle},
    };
    use move_core_types::account_address::AccountAddress;
    use std::collections::HashMap;

    /// Generates a dummy identifier for aggregator V1 based on the given key.
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
            self.set_from_state_key(id.into_state_key(), value);
        }

        pub fn set_from_state_key(&mut self, state_key: StateKey, value: u128) {
            self.0
                .insert(state_key, StateValue::new_legacy(serialize(&value).into()));
        }
    }

    impl TAggregatorView for AggregatorStore {
        type IdentifierV1 = StateKey;
        type IdentifierV2 = AggregatorID;

        fn get_aggregator_v1_state_value(
            &self,
            state_key: &Self::IdentifierV1,
            _mode: AggregatorReadMode,
        ) -> anyhow::Result<Option<StateValue>> {
            Ok(self.0.get(state_key).cloned())
        }
    }
}
