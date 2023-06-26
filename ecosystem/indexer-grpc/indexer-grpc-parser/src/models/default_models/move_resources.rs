// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::extra_unused_lifetimes)]
use super::transactions::Transaction;
use crate::{schema::move_resources, utils::util::standardize_address};
use anyhow::{Context, Result};
use aptos_protos::transaction::v1::{
    DeleteResource, MoveStructTag as MoveStructTagPB, WriteResource,
};
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
    address: String,
    pub module: String,
    pub name: String,
    pub generic_type_params: Option<serde_json::Value>,
}

impl MoveResource {
    pub fn from_write_resource(
        write_resource: &WriteResource,
        write_set_change_index: i64,
        transaction_version: i64,
        transaction_block_height: i64,
    ) -> Self {
        let parsed_data = Self::convert_move_struct_tag(
            write_resource
                .r#type
                .as_ref()
                .expect("MoveStructTag Not Exists."),
        );
        Self {
            transaction_version,
            transaction_block_height,
            write_set_change_index,
            type_: write_resource.type_str.clone(),
            name: parsed_data.name.clone(),
            address: standardize_address(&write_resource.address.to_string()),
            module: parsed_data.module.clone(),
            generic_type_params: parsed_data.generic_type_params,
            data: Some(serde_json::from_str(write_resource.data.as_str()).unwrap()),
            is_deleted: false,
            state_key_hash: standardize_address(
                hex::encode(write_resource.state_key_hash.as_slice()).as_str(),
            ),
        }
    }

    pub fn from_delete_resource(
        delete_resource: &DeleteResource,
        write_set_change_index: i64,
        transaction_version: i64,
        transaction_block_height: i64,
    ) -> Self {
        let parsed_data = Self::convert_move_struct_tag(
            delete_resource
                .r#type
                .as_ref()
                .expect("MoveStructTag Not Exists."),
        );
        Self {
            transaction_version,
            transaction_block_height,
            write_set_change_index,
            type_: delete_resource.type_str.clone(),
            name: parsed_data.name.clone(),
            address: standardize_address(&delete_resource.address.to_string()),
            module: parsed_data.module.clone(),
            generic_type_params: parsed_data.generic_type_params,
            data: None,
            is_deleted: true,
            state_key_hash: standardize_address(
                hex::encode(delete_resource.state_key_hash.as_slice()).as_str(),
            ),
        }
    }

    pub fn convert_move_struct_tag(struct_tag: &MoveStructTagPB) -> MoveStructTag {
        MoveStructTag {
            address: standardize_address(struct_tag.address.as_str()),
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

    pub fn get_outer_type_from_resource(write_resource: &WriteResource) -> String {
        let move_struct_tag =
            Self::convert_move_struct_tag(write_resource.r#type.as_ref().unwrap());

        format!(
            "{}::{}::{}",
            move_struct_tag.get_address(),
            move_struct_tag.module,
            move_struct_tag.name,
        )
    }
}

impl MoveStructTag {
    pub fn get_address(&self) -> String {
        standardize_address(self.address.as_str())
    }
}
