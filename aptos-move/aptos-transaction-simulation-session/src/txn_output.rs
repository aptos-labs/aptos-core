// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::HumanReadable;
use anyhow::{bail, Result};
use aptos_resource_viewer::AptosValueAnnotator;
use aptos_types::{
    access_path::Path as AccessPath,
    contract_event::ContractEvent,
    state_store::{state_key::inner::StateKeyInner, StateView},
    write_set::{PersistedWriteOp, WriteSet},
};
use move_core_types::language_storage::{StructTag, TypeTag};
use serde_json::json;
use std::{collections::BTreeMap, path::Path};

/// Writes a write set to a file in a human-readable format.
///
/// Specifically, state values are decoded and annotated with field names and structure
/// for easier inspection.
///
/// This format is intended for debugging and inspection only, and is not meant to be
/// reversible.
pub fn save_write_set(
    state_view: &impl StateView,
    write_set_path: &Path,
    write_set: &WriteSet,
) -> Result<()> {
    let mut entries = BTreeMap::new();

    let annotator = AptosValueAnnotator::new(state_view);

    for (k, v) in write_set.write_op_iter() {
        let key = HumanReadable(k).to_string();

        let encode_data = |data: &[u8]| -> Result<serde_json::Value> {
            let val = match k.inner() {
                StateKeyInner::AccessPath(access_path) => match access_path.get_path() {
                    AccessPath::Resource(struct_tag) => {
                        json!(annotator.view_resource(&struct_tag, data)?)
                    },
                    AccessPath::ResourceGroup(_struct_tag) => {
                        let group: BTreeMap<StructTag, Vec<u8>> = bcs::from_bytes(data)?;

                        let mut group_pretty = BTreeMap::new();

                        for (k, v) in group {
                            group_pretty
                                .insert(k.to_canonical_string(), annotator.view_resource(&k, &v)?);
                        }

                        json!(group_pretty)
                    },
                    _ => json!(hex::encode(&data)),
                },
                _ => json!(hex::encode(&data)),
            };
            Ok(val)
        };

        let val = match v.to_persistable() {
            PersistedWriteOp::Creation(data) => {
                json!({
                    "create": {
                        "data": encode_data(&data)?,
                    }
                })
            },
            PersistedWriteOp::Modification(data) => {
                json!({
                    "modify": {
                        "data": encode_data(&data)?,
                    }
                })
            },
            PersistedWriteOp::Deletion => {
                json!({
                    "delete": {}
                })
            },
            PersistedWriteOp::CreationWithMetadata { data, metadata } => {
                json!({
                    "create": {
                        "data": encode_data(&data)?,
                        "metadata": metadata,
                    }
                })
            },
            PersistedWriteOp::ModificationWithMetadata { data, metadata } => {
                json!({
                    "modify": {
                        "data": encode_data(&data)?,
                        "metadata": metadata,
                    }
                })
            },
            PersistedWriteOp::DeletionWithMetadata { metadata } => {
                json!({
                    "delete": {
                        "metadata": metadata,
                    }
                })
            },
        };

        entries.insert(key, val);
    }

    std::fs::write(write_set_path, serde_json::to_string_pretty(&entries)?)?;

    Ok(())
}

/// Saves events to a file, in a human readable format.
///
/// Specifically, event data is decoded and annotated with field names and structure
/// for easier inspection.
///
/// This format is intended for debugging and inspection only, and is not meant to be
/// reversible.
pub fn save_events(
    events_path: &Path,
    state_view: &impl StateView,
    events: &[ContractEvent],
) -> Result<()> {
    let mut entries = vec![];

    let annotator = AptosValueAnnotator::new(state_view);

    fn struct_tag_from_type_tag(type_tag: &TypeTag) -> Result<&StructTag> {
        match type_tag {
            TypeTag::Struct(struct_tag) => Ok(struct_tag),
            _ => bail!("Expected struct type tag, got: {:?}", type_tag),
        }
    }

    for event in events {
        let val = match event {
            ContractEvent::V1(event) => json!({
                "V1": {
                    "key": event.key().clone(),
                    "sequence_number": event.sequence_number(),
                    "type_tag": event.type_tag().to_canonical_string(),
                    "event_data": annotator.view_resource(struct_tag_from_type_tag(event.type_tag())?, event.event_data())?
                }
            }),
            ContractEvent::V2(event) => json!({
                "V2": {
                    "type_tag": event.type_tag().to_canonical_string(),
                    "event_data": annotator.view_resource(struct_tag_from_type_tag(event.type_tag())?, event.event_data())?
                }
            }),
        };

        entries.push(val);
    }

    std::fs::write(events_path, serde_json::to_string_pretty(&entries)?)?;

    Ok(())
}
