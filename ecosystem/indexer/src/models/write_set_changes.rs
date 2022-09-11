// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::extra_unused_lifetimes)]
use super::{
    move_modules::MoveModule,
    move_resources::MoveResource,
    move_tables::{TableItem, TableMetadata},
};
use crate::{models::transactions::Transaction, schema::write_set_changes};
use aptos_rest_client::aptos_api_types::WriteSetChange as APIWriteSetChange;
use field_count::FieldCount;
use serde::Serialize;

#[derive(Associations, Debug, FieldCount, Identifiable, Insertable, Queryable, Serialize)]
#[diesel(table_name = "write_set_changes")]
#[belongs_to(Transaction, foreign_key = "transaction_version")]
#[primary_key(transaction_version, index)]
pub struct WriteSetChange {
    pub transaction_version: i64,
    pub index: i64,
    pub hash: String,
    #[diesel(column_name = type)]
    pub type_: String,
    pub address: String,
    // Default time columns
    pub inserted_at: chrono::NaiveDateTime,
}

impl WriteSetChange {
    pub fn from_write_set_change(
        write_set_change: &APIWriteSetChange,
        index: i64,
        transaction_version: i64,
    ) -> (Self, WriteSetChangeDetail) {
        let type_ = Self::get_write_set_change_type(write_set_change);
        match write_set_change {
            APIWriteSetChange::WriteModule(module) => (
                Self {
                    transaction_version,
                    hash: module.state_key_hash.clone(),
                    type_,
                    address: module.address.to_string(),
                    index,
                    inserted_at: chrono::Utc::now().naive_utc(),
                },
                WriteSetChangeDetail::Module(MoveModule::from_write_module(
                    &module,
                    index,
                    transaction_version,
                )),
            ),
            APIWriteSetChange::DeleteModule(module) => (
                Self {
                    transaction_version,
                    hash: module.state_key_hash.clone(),
                    type_,
                    address: module.address.to_string(),
                    index,
                    inserted_at: chrono::Utc::now().naive_utc(),
                },
                WriteSetChangeDetail::Module(MoveModule::from_delete_module(
                    &module,
                    index,
                    transaction_version,
                )),
            ),
            APIWriteSetChange::WriteResource(resource) => (
                Self {
                    transaction_version,
                    hash: resource.state_key_hash.clone(),
                    type_,
                    address: resource.address.to_string(),
                    index,
                    inserted_at: chrono::Utc::now().naive_utc(),
                },
                WriteSetChangeDetail::Resource(MoveResource::from_write_resource(
                    &resource,
                    index,
                    transaction_version,
                )),
            ),
            APIWriteSetChange::DeleteResource(resource) => (
                Self {
                    transaction_version,
                    hash: resource.state_key_hash.clone(),
                    type_,
                    address: resource.address.to_string(),
                    index,
                    inserted_at: chrono::Utc::now().naive_utc(),
                },
                WriteSetChangeDetail::Resource(MoveResource::from_delete_resource(
                    &resource,
                    index,
                    transaction_version,
                )),
            ),
            APIWriteSetChange::WriteTableItem(table_item) => (
                Self {
                    transaction_version,
                    hash: table_item.state_key_hash.clone(),
                    type_,
                    address: String::default(),
                    index,
                    inserted_at: chrono::Utc::now().naive_utc(),
                },
                WriteSetChangeDetail::Table(
                    TableItem::from_write_table_item(&table_item, index, transaction_version),
                    Some(TableMetadata::from_write_table_item(&table_item)),
                ),
            ),
            APIWriteSetChange::DeleteTableItem(table_item) => (
                Self {
                    transaction_version,
                    hash: table_item.state_key_hash.clone(),
                    type_,
                    address: String::default(),
                    index,
                    inserted_at: chrono::Utc::now().naive_utc(),
                },
                WriteSetChangeDetail::Table(
                    TableItem::from_delete_table_item(table_item, index, transaction_version),
                    None,
                ),
            ),
        }
    }

    pub fn from_write_set_changes(
        write_set_changes: &[APIWriteSetChange],
        transaction_version: i64,
    ) -> (Vec<Self>, Vec<WriteSetChangeDetail>) {
        write_set_changes
            .iter()
            .enumerate()
            .map(|(index, write_set_change)| {
                Self::from_write_set_change(write_set_change, index as i64, transaction_version)
            })
            .collect::<Vec<(Self, WriteSetChangeDetail)>>()
            .into_iter()
            .unzip()
    }

    fn get_write_set_change_type(t: &APIWriteSetChange) -> String {
        match t {
            APIWriteSetChange::DeleteModule(_) => String::from("delete_module"),
            APIWriteSetChange::DeleteResource(_) => String::from("delete_resource"),
            APIWriteSetChange::DeleteTableItem(_) => String::from("delete_table_item"),
            APIWriteSetChange::WriteModule(_) => String::from("write_module"),
            APIWriteSetChange::WriteResource(_) => String::from("write_resource"),
            APIWriteSetChange::WriteTableItem(_) => String::from("write_table_item"),
        }
    }
}

pub enum WriteSetChangeDetail {
    Module(MoveModule),
    Resource(MoveResource),
    Table(TableItem, Option<TableMetadata>),
}

// Prevent conflicts with other things named `WriteSetChange`
pub type WriteSetChangeModel = WriteSetChange;
