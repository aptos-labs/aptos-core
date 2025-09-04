// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::extra_unused_lifetimes)]
use crate::{
    models::transactions::Transaction,
    schema::{current_table_items, table_items, table_metadatas},
    util::{hash_str, standardize_address},
};
use velor_api_types::{DeleteTableItem, WriteTableItem};
use field_count::FieldCount;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(table_handle, key_hash))]
#[diesel(table_name = current_table_items)]
pub struct CurrentTableItem {
    pub table_handle: String,
    pub key_hash: String,
    pub key: String,
    pub decoded_key: serde_json::Value,
    pub decoded_value: Option<serde_json::Value>,
    pub last_transaction_version: i64,
    pub is_deleted: bool,
}

#[derive(
    Associations, Clone, Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize,
)]
#[diesel(belongs_to(Transaction, foreign_key = transaction_version))]
#[diesel(primary_key(transaction_version, write_set_change_index))]
#[diesel(table_name = table_items)]
pub struct TableItem {
    pub transaction_version: i64,
    pub write_set_change_index: i64,
    pub transaction_block_height: i64,
    pub key: String,
    pub table_handle: String,
    pub decoded_key: serde_json::Value,
    pub decoded_value: Option<serde_json::Value>,
    pub is_deleted: bool,
}

#[derive(Clone, Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(handle))]
#[diesel(table_name = table_metadatas)]
pub struct TableMetadata {
    pub handle: String,
    pub key_type: String,
    pub value_type: String,
}

impl TableItem {
    pub fn from_write_table_item(
        write_table_item: &WriteTableItem,
        write_set_change_index: i64,
        transaction_version: i64,
        transaction_block_height: i64,
    ) -> (Self, CurrentTableItem) {
        (
            Self {
                transaction_version,
                write_set_change_index,
                transaction_block_height,
                key: write_table_item.key.to_string(),
                table_handle: standardize_address(&write_table_item.handle.to_string()),
                decoded_key: write_table_item.data.as_ref().unwrap().key.clone(),
                decoded_value: Some(write_table_item.data.as_ref().unwrap().value.clone()),
                is_deleted: false,
            },
            CurrentTableItem {
                table_handle: standardize_address(&write_table_item.handle.to_string()),
                key_hash: hash_str(&write_table_item.key.to_string()),
                key: write_table_item.key.to_string(),
                decoded_key: write_table_item.data.as_ref().unwrap().key.clone(),
                decoded_value: Some(write_table_item.data.as_ref().unwrap().value.clone()),
                last_transaction_version: transaction_version,
                is_deleted: false,
            },
        )
    }

    pub fn from_delete_table_item(
        delete_table_item: &DeleteTableItem,
        write_set_change_index: i64,
        transaction_version: i64,
        transaction_block_height: i64,
    ) -> (Self, CurrentTableItem) {
        let decoded_key = delete_table_item
            .data
            .as_ref()
            .unwrap_or_else(|| {
                panic!(
                    "Could not extract data from DeletedTableItem '{:?}'",
                    delete_table_item
                )
            })
            .key
            .clone();
        (
            Self {
                transaction_version,
                write_set_change_index,
                transaction_block_height,
                key: delete_table_item.key.to_string(),
                table_handle: standardize_address(&delete_table_item.handle.to_string()),
                decoded_key: decoded_key.clone(),
                decoded_value: None,
                is_deleted: true,
            },
            CurrentTableItem {
                table_handle: standardize_address(&delete_table_item.handle.to_string()),
                key_hash: hash_str(&delete_table_item.key.to_string()),
                key: delete_table_item.key.to_string(),
                decoded_key,
                decoded_value: None,
                last_transaction_version: transaction_version,
                is_deleted: true,
            },
        )
    }
}

impl TableMetadata {
    pub fn from_write_table_item(table_item: &WriteTableItem) -> Self {
        Self {
            handle: table_item.handle.to_string(),
            key_type: table_item.data.as_ref().unwrap().key_type.clone(),
            value_type: table_item.data.as_ref().unwrap().value_type.clone(),
        }
    }
}
