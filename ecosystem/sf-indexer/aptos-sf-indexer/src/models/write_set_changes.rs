// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::extra_unused_lifetimes)]
use crate::{
    models::transactions::Transaction,
    schema::{move_modules, move_resources, table_items, table_metadatas, write_set_changes},
};
use aptos_protos::block_output::v1::{
    write_set_change_output::Change, MoveModuleOutput, MoveResourceOutput, TableItemOutput,
    WriteSetChangeOutput,
};
use aptos_rest_client::aptos_api_types::HexEncodedBytes;
use field_count::FieldCount;
use serde::Serialize;

#[derive(Associations, Debug, FieldCount, Identifiable, Insertable, Queryable, Serialize)]
#[diesel(table_name = "write_set_changes")]
#[belongs_to(Transaction, foreign_key = "transaction_version")]
#[primary_key(transaction_version, hash)]
pub struct WriteSetChange {
    pub transaction_version: i64,
    pub hash: String,
    pub transaction_block_height: i64,
    #[diesel(column_name = type)]
    pub type_: String,
    pub address: String,
    pub index: i64,
    // Default time columns
    pub inserted_at: chrono::NaiveDateTime,
}

impl WriteSetChange {
    pub fn from_write_set_change(
        write_set_change: &WriteSetChangeOutput,
        index: usize,
        block_height: u64,
    ) -> (Self, WriteSetChangeDetail) {
        let version = write_set_change.version as i64;
        let block_height = block_height as i64;
        let hash = HexEncodedBytes::from(write_set_change.hash.clone()).to_string();
        let type_ = write_set_change.r#type.clone();
        match write_set_change.change.as_ref().unwrap() {
            Change::MoveModule(module) => (
                WriteSetChange {
                    transaction_version: version,
                    hash,
                    transaction_block_height: block_height,
                    type_,
                    address: module.address.clone(),
                    index: index as i64,
                    inserted_at: chrono::Utc::now().naive_utc(),
                },
                WriteSetChangeDetail::Module(MoveModule::from_module(
                    module,
                    index,
                    version,
                    block_height,
                )),
            ),
            Change::MoveResource(resource) => (
                WriteSetChange {
                    transaction_version: version,
                    hash,
                    transaction_block_height: block_height,
                    type_,
                    address: resource.address.clone(),
                    index: index as i64,
                    inserted_at: chrono::Utc::now().naive_utc(),
                },
                WriteSetChangeDetail::Resource(MoveResource::from_resource(
                    resource,
                    index,
                    version,
                    block_height,
                )),
            ),
            Change::TableItem(table_item) => (
                WriteSetChange {
                    transaction_version: version,
                    hash,
                    transaction_block_height: block_height,
                    type_,
                    address: String::default(),
                    index: index as i64,
                    inserted_at: chrono::Utc::now().naive_utc(),
                },
                WriteSetChangeDetail::Table(
                    TableItem::from_table_item(table_item, index, version, block_height),
                    TableMetadata::from_table_item(table_item),
                ),
            ),
        }
    }

    pub fn from_write_set_changes(
        write_set_changes: &[WriteSetChangeOutput],
        block_height: u64,
    ) -> (Vec<Self>, Vec<WriteSetChangeDetail>) {
        write_set_changes
            .iter()
            .enumerate()
            .map(|(index, write_set_change)| {
                Self::from_write_set_change(write_set_change, index, block_height)
            })
            .collect::<Vec<(Self, WriteSetChangeDetail)>>()
            .into_iter()
            .unzip()
    }
}

pub enum WriteSetChangeDetail {
    Module(MoveModule),
    Resource(MoveResource),
    Table(TableItem, TableMetadata),
}

#[derive(
    Associations, Clone, Debug, FieldCount, Identifiable, Insertable, Queryable, Serialize,
)]
#[belongs_to(Transaction, foreign_key = "transaction_version")]
#[primary_key(transaction_version, write_set_change_index)]
#[diesel(table_name = "move_modules")]
pub struct MoveModule {
    pub transaction_version: i64,
    pub write_set_change_index: i64,
    pub transaction_block_height: i64,
    pub name: String,
    pub address: String,
    pub bytecode: Option<Vec<u8>>,
    pub friends: Option<serde_json::Value>,
    pub structs: Option<serde_json::Value>,
    pub is_deleted: bool,
    // Default time columns
    pub inserted_at: chrono::NaiveDateTime,
}

impl MoveModule {
    pub fn from_module(
        move_module: &MoveModuleOutput,
        write_set_change_index: usize,
        version: i64,
        block_height: i64,
    ) -> Self {
        Self {
            transaction_version: version,
            write_set_change_index: write_set_change_index as i64,
            transaction_block_height: block_height,
            name: move_module.name.clone(),
            address: move_module.address.clone(),
            bytecode: (!move_module.is_deleted).then(|| move_module.bytecode.clone()),
            friends: (!move_module.is_deleted)
                .then(|| serde_json::to_value(move_module.friends.clone()).unwrap()),
            structs: (!move_module.is_deleted)
                .then(|| serde_json::to_value(move_module.structs.clone()).unwrap()),
            is_deleted: move_module.is_deleted,
            inserted_at: chrono::Utc::now().naive_utc(),
        }
    }
}

#[derive(
    Associations, Clone, Debug, FieldCount, Identifiable, Insertable, Queryable, Serialize,
)]
#[belongs_to(Transaction, foreign_key = "transaction_version")]
#[primary_key(transaction_version, write_set_change_index)]
#[diesel(table_name = "move_resources")]
pub struct MoveResource {
    pub transaction_version: i64,
    pub write_set_change_index: i64,
    pub transaction_block_height: i64,
    pub type_str: String,
    pub name: String,
    pub address: String,
    pub module: String,
    pub generic_type_params: Option<serde_json::Value>,
    pub data: Option<serde_json::Value>,
    pub is_deleted: bool,
    // Default time columns
    pub inserted_at: chrono::NaiveDateTime,
}

impl MoveResource {
    pub fn from_resource(
        move_resource: &MoveResourceOutput,
        write_set_change_index: usize,
        version: i64,
        block_height: i64,
    ) -> Self {
        Self {
            transaction_version: version,
            write_set_change_index: write_set_change_index as i64,
            transaction_block_height: block_height,
            type_str: move_resource.type_str.clone(),
            name: move_resource.name.clone(),
            address: move_resource.address.clone(),
            module: move_resource.module.clone(),
            generic_type_params: (!move_resource.is_deleted)
                .then(|| serde_json::to_value(move_resource.generic_type_params.clone()).unwrap()),
            data: (!move_resource.is_deleted)
                .then(|| serde_json::from_str(&move_resource.data).unwrap()),
            is_deleted: move_resource.is_deleted,
            inserted_at: chrono::Utc::now().naive_utc(),
        }
    }
}

#[derive(
    Associations, Clone, Debug, FieldCount, Identifiable, Insertable, Queryable, Serialize,
)]
#[belongs_to(Transaction, foreign_key = "transaction_version")]
#[primary_key(transaction_version, write_set_change_index)]
#[diesel(table_name = "table_items")]
pub struct TableItem {
    pub transaction_version: i64,
    pub write_set_change_index: i64,
    pub transaction_block_height: i64,
    pub key: String,
    pub table_handle: String,
    pub decoded_key: serde_json::Value,
    pub decoded_value: Option<serde_json::Value>,
    pub is_deleted: bool,
    // Default time columns
    pub inserted_at: chrono::NaiveDateTime,
}

impl TableItem {
    pub fn from_table_item(
        table_item: &TableItemOutput,
        write_set_change_index: usize,
        version: i64,
        block_height: i64,
    ) -> Self {
        Self {
            transaction_version: version,
            write_set_change_index: write_set_change_index as i64,
            transaction_block_height: block_height,
            key: table_item.key.clone(),
            table_handle: table_item.handle.clone(),
            decoded_key: serde_json::from_str(&table_item.decoded_key).unwrap(),
            decoded_value: (!table_item.is_deleted)
                .then(|| serde_json::from_str(&table_item.decoded_value).unwrap()),
            is_deleted: table_item.is_deleted,
            inserted_at: chrono::Utc::now().naive_utc(),
        }
    }
}

#[derive(
    Associations, Clone, Debug, FieldCount, Identifiable, Insertable, Queryable, Serialize,
)]
#[primary_key(handle)]
#[diesel(table_name = "table_metadatas")]
pub struct TableMetadata {
    pub handle: String,
    pub key_type: String,
    pub value_type: String,
    // Default time columns
    pub inserted_at: chrono::NaiveDateTime,
}

impl TableMetadata {
    pub fn from_table_item(table_item: &TableItemOutput) -> Self {
        Self {
            handle: table_item.handle.clone(),
            key_type: table_item.key_type.clone(),
            value_type: table_item.value_type.clone(),
            inserted_at: chrono::Utc::now().naive_utc(),
        }
    }
}

// Prevent conflicts with other things named `WriteSetChange`
pub type WriteSetChangeModel = WriteSetChange;
