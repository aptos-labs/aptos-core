// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_aggregator::types::{
    DelayedFieldID, DelayedFieldValue, TryFromMoveValue, TryIntoMoveValue,
};
use aptos_table_natives::{TableHandle, TableResolver};
use aptos_types::{access_path::AccessPath, state_store::state_key::StateKey};
use bytes::Bytes;
use claims::assert_some;
use move_binary_format::errors::PartialVMError;
use move_core_types::{
    account_address::AccountAddress,
    language_storage::StructTag,
    metadata::Metadata,
    resolver::ResourceResolver,
    value::{IdentifierMappingKind, MoveTypeLayout},
    vm_status::StatusCode,
};
use move_vm_types::{
    value_transformation::{
        deserialize_and_replace_values_with_ids, TransformationResult, ValueToIdentifierMapping,
    },
    values::Value,
};
use std::{cell::RefCell, collections::BTreeMap};

/// Models a storage backend which only stores bytes of data.
#[derive(Debug, Default)]
pub struct MockDB {
    db: BTreeMap<StateKey, Bytes>,
}

impl MockDB {
    pub(crate) fn get_bytes(&self, state_key: &StateKey) -> Option<Bytes> {
        self.db.get(state_key).cloned()
    }

    pub(crate) fn store_bytes(&mut self, state_key: StateKey, bytes: Bytes) {
        self.db.insert(state_key, bytes);
    }
}

/// Models a state view which has:
///   1. A map of extracted aggregator / snapshot values.
///   2. A cache layer which models per-block data.
/// . 3. Actual storage backend.
#[derive(Debug, Default)]
pub(crate) struct MockStateView {
    mapping: RefCell<BTreeMap<DelayedFieldID, Value>>,
    in_memory_cache: BTreeMap<StateKey, Bytes>,
    db: MockDB,
}

impl MockStateView {
    pub(crate) fn add_to_db(&mut self, state_key: StateKey, value: Value, layout: MoveTypeLayout) {
        // INVARIANT: All data in storage (base) is stored as is.
        let blob = value
            .simple_serialize(&layout)
            .expect("Deserialization when storing a value always succeeds");
        self.db.store_bytes(state_key, blob.into());
    }

    pub(crate) fn add_mapping(&self, unique_index: u32, width: u32, v: Value) {
        let mut mapping = self.mapping.borrow_mut();
        mapping.insert(DelayedFieldID::new_with_width(unique_index, width), v);
    }

    pub(crate) fn add_to_in_memory_cache(
        &mut self,
        state_key: StateKey,
        value: Value,
        layout: MoveTypeLayout,
    ) {
        // INVARIANT: All data in cache must be lifted.
        // As a result, one should call `add_mapping` before this method.
        let blob = value
            .simple_serialize(&layout)
            .expect("Deserialization when caching a value always succeeds");
        self.in_memory_cache.insert(state_key, blob.into());
    }

    pub(crate) fn assert_mapping_equal_at(
        &self,
        unique_index: u32,
        width: u32,
        expected_value: Value,
    ) {
        let mapping = self.mapping.borrow();
        let actual_value =
            assert_some!(mapping.get(&DelayedFieldID::new_with_width(unique_index, width)));

        assert!(
            actual_value.equals(&expected_value).unwrap(),
            "actual_value: {:?}, expected_value: {:?}",
            actual_value,
            expected_value
        );
    }
}

impl ValueToIdentifierMapping for MockStateView {
    fn value_to_identifier(
        &self,
        kind: &IdentifierMappingKind,
        layout: &MoveTypeLayout,
        value: Value,
    ) -> TransformationResult<Value> {
        let (_base_value, width) =
            DelayedFieldValue::try_from_move_value(layout, value.copy_value()?, kind)?;

        let mut mapping = self.mapping.borrow_mut();
        let unique_index = mapping.len() as u32;
        let identifier = DelayedFieldID::new_with_width(unique_index, width);
        let identifier_value = identifier
            .try_into_move_value(layout)
            .map_err(PartialVMError::from)?;

        mapping.insert(identifier, value);
        Ok(identifier_value)
    }

    fn identifier_to_value(
        &self,
        layout: &MoveTypeLayout,
        identifier: Value,
    ) -> TransformationResult<Value> {
        let mapping = self.mapping.borrow();
        let (identifier, width) = DelayedFieldID::try_from_move_value(layout, identifier, &())
            .map_err(PartialVMError::from)?;
        assert_eq!(identifier.extract_width(), width);

        Ok(mapping
            .get(&identifier)
            .expect("Identifiers must always exist in the mapping")
            .copy_value()
            .expect("Copying mapped values should never fail"))
    }
}

// Performs a serialization round-trip, exchanging values which are supposed
// to be mapped to identifiers.
macro_rules! patch_blob_from_db {
    ($blob:ident, $layout:ident, $exchange:ident) => {
        deserialize_and_replace_values_with_ids(&$blob, $layout, $exchange)
            .map(|value| value.simple_serialize($layout))
            .flatten()
            .ok_or_else(|| {
                PartialVMError::new(StatusCode::VALUE_DESERIALIZATION_ERROR)
                    .with_message("Failed to deserialize and replace with identifiers".to_string())
            })
    };
}

impl ResourceResolver for MockStateView {
    type Error = PartialVMError;

    fn get_resource_bytes_with_metadata_and_layout(
        &self,
        address: &AccountAddress,
        typ: &StructTag,
        _metadata: &[Metadata],
        maybe_layout: Option<&MoveTypeLayout>,
    ) -> Result<(Option<Bytes>, usize), Self::Error> {
        let ap = AccessPath::resource_access_path(*address, typ.clone())
            .expect("Access path for resource have to be valid");
        let state_key = StateKey::access_path(ap);

        Ok(match self.in_memory_cache.get(&state_key) {
            Some(blob) => (Some(blob.clone()), blob.len()),
            None => {
                // If a resource is not cached, we must exchange lifted values.
                match self.db.get_bytes(&state_key) {
                    Some(blob) => {
                        if let Some(layout) = maybe_layout {
                            let patched_blob = patch_blob_from_db!(blob, layout, self)?;
                            let resource_size = patched_blob.len();
                            (Some(patched_blob.into()), resource_size)
                        } else {
                            let resource_size = blob.len();
                            (Some(blob), resource_size)
                        }
                    },
                    None => (None, 0),
                }
            },
        })
    }
}

impl TableResolver for MockStateView {
    fn resolve_table_entry_bytes_with_layout(
        &self,
        handle: &TableHandle,
        key: &[u8],
        maybe_layout: Option<&MoveTypeLayout>,
    ) -> Result<Option<Bytes>, PartialVMError> {
        let state_key = StateKey::table_item((*handle).into(), key.to_vec());
        Ok(match self.in_memory_cache.get(&state_key) {
            Some(blob) => Some(blob.clone()),
            None => {
                // Otherwise the table entry is not cached and we fetch from storage.
                // Since we have a layout passed, we can need to do the value exchange
                // here by serialization round-trip.
                match self.db.get_bytes(&state_key) {
                    Some(blob) => Some(
                        if let Some(layout) = maybe_layout {
                            patch_blob_from_db!(blob, layout, self)?.into()
                        } else {
                            blob
                        },
                    ),
                    None => None,
                }
            },
        })
    }
}
