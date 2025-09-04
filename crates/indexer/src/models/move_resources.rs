// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::extra_unused_lifetimes)]
use crate::{models::transactions::Transaction, schema::move_resources, util::standardize_address};
use anyhow::{Context, Result};
use velor_api_types::{DeleteResource, MoveStructTag as APIMoveStructTag, WriteResource};
use field_count::FieldCount;
use serde::{Deserialize, Serialize};
#[derive(
    Associations, Clone, Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize,
)]
#[diesel(belongs_to(Transaction, foreign_key = transaction_version))]
#[diesel(primary_key(transaction_version, write_set_change_index))]
#[diesel(table_name = move_resources)]
pub struct MoveResource {
    pub transaction_version: i64,
    pub write_set_change_index: i64,
    pub transaction_block_height: i64,
    pub name: String,
    pub type_: String,
    pub address: String,
    pub module: String,
    pub generic_type_params: Option<serde_json::Value>,
    pub data: Option<serde_json::Value>,
    pub is_deleted: bool,
    pub state_key_hash: String,
}

pub struct MoveStructTag {
    module: String,
    name: String,
    generic_type_params: Option<serde_json::Value>,
}

impl MoveResource {
    pub fn from_write_resource(
        write_resource: &WriteResource,
        write_set_change_index: i64,
        transaction_version: i64,
        transaction_block_height: i64,
    ) -> Self {
        let parsed_data = Self::convert_move_struct_tag(&write_resource.data.typ);
        Self {
            transaction_version,
            transaction_block_height,
            write_set_change_index,
            type_: write_resource.data.typ.to_string(),
            name: parsed_data.name.clone(),
            address: standardize_address(&write_resource.address.to_string()),
            module: parsed_data.module.clone(),
            generic_type_params: parsed_data.generic_type_params,
            data: Some(serde_json::to_value(&write_resource.data.data).unwrap()),
            is_deleted: false,
            state_key_hash: standardize_address(write_resource.state_key_hash.as_str()),
        }
    }

    pub fn from_delete_resource(
        delete_resource: &DeleteResource,
        write_set_change_index: i64,
        transaction_version: i64,
        transaction_block_height: i64,
    ) -> Self {
        let parsed_data = Self::convert_move_struct_tag(&delete_resource.resource);
        Self {
            transaction_version,
            transaction_block_height,
            write_set_change_index,
            type_: delete_resource.resource.to_string(),
            name: parsed_data.name.clone(),
            address: standardize_address(&delete_resource.address.to_string()),
            module: parsed_data.module.clone(),
            generic_type_params: parsed_data.generic_type_params,
            data: None,
            is_deleted: true,
            state_key_hash: standardize_address(delete_resource.state_key_hash.as_str()),
        }
    }

    pub fn convert_move_struct_tag(struct_tag: &APIMoveStructTag) -> MoveStructTag {
        MoveStructTag {
            module: struct_tag.module.to_string(),
            name: struct_tag.name.to_string(),
            generic_type_params: struct_tag
                .generic_type_params
                .iter()
                .map(|move_type| -> Result<Option<serde_json::Value>> {
                    Ok(Some(
                        serde_json::to_value(move_type).context("Failed to parse move type")?,
                    ))
                })
                .collect::<Result<Option<serde_json::Value>>>()
                .unwrap_or(None),
        }
    }
}
