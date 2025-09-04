// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::extra_unused_lifetimes)]
use super::{
    move_modules::MoveModule,
    move_resources::MoveResource,
    move_tables::{CurrentTableItem, TableItem, TableMetadata},
    transactions::TransactionQuery,
};
use crate::{
    models::transactions::Transaction, schema::write_set_changes, util::standardize_address,
};
use velor_api_types::WriteSetChange as APIWriteSetChange;
use field_count::FieldCount;
use serde::{Deserialize, Serialize};

#[derive(Associations, Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(belongs_to(Transaction, foreign_key = transaction_version))]
#[diesel(primary_key(transaction_version, index))]
#[diesel(table_name = write_set_changes)]
pub struct WriteSetChange {
    pub transaction_version: i64,
    pub index: i64,
    pub hash: String,
    transaction_block_height: i64,
    pub type_: String,
    pub address: String,
}

/// Need a separate struct for queryable because we don't want to define the inserted_at column (letting DB fill)
#[derive(Associations, Debug, Deserialize, Identifiable, Queryable, Serialize)]
#[diesel(belongs_to(TransactionQuery, foreign_key = transaction_version))]
#[diesel(primary_key(transaction_version, index))]
#[diesel(table_name = write_set_changes)]
pub struct WriteSetChangeQuery {
    pub transaction_version: i64,
    pub index: i64,
    pub hash: String,
    transaction_block_height: i64,
    pub type_: String,
    pub address: String,
    pub inserted_at: chrono::NaiveDateTime,
}

impl WriteSetChange {
    pub fn from_write_set_change(
        write_set_change: &APIWriteSetChange,
        index: i64,
        transaction_version: i64,
        transaction_block_height: i64,
    ) -> (Self, WriteSetChangeDetail) {
        let type_ = Self::get_write_set_change_type(write_set_change);
        match write_set_change {
            APIWriteSetChange::WriteModule(module) => (
                Self {
                    transaction_version,
                    hash: module.state_key_hash.clone(),
                    transaction_block_height,
                    type_,
                    address: standardize_address(&module.address.to_string()),
                    index,
                },
                WriteSetChangeDetail::Module(MoveModule::from_write_module(
                    module,
                    index,
                    transaction_version,
                    transaction_block_height,
                )),
            ),
            APIWriteSetChange::DeleteModule(module) => (
                Self {
                    transaction_version,
                    hash: module.state_key_hash.clone(),
                    transaction_block_height,
                    type_,
                    address: standardize_address(&module.address.to_string()),
                    index,
                },
                WriteSetChangeDetail::Module(MoveModule::from_delete_module(
                    module,
                    index,
                    transaction_version,
                    transaction_block_height,
                )),
            ),
            APIWriteSetChange::WriteResource(resource) => (
                Self {
                    transaction_version,
                    hash: resource.state_key_hash.clone(),
                    transaction_block_height,
                    type_,
                    address: standardize_address(&resource.address.to_string()),
                    index,
                },
                WriteSetChangeDetail::Resource(MoveResource::from_write_resource(
                    resource,
                    index,
                    transaction_version,
                    transaction_block_height,
                )),
            ),
            APIWriteSetChange::DeleteResource(resource) => (
                Self {
                    transaction_version,
                    hash: resource.state_key_hash.clone(),
                    transaction_block_height,
                    type_,
                    address: standardize_address(&resource.address.to_string()),
                    index,
                },
                WriteSetChangeDetail::Resource(MoveResource::from_delete_resource(
                    resource,
                    index,
                    transaction_version,
                    transaction_block_height,
                )),
            ),
            APIWriteSetChange::WriteTableItem(table_item) => {
                let (ti, cti) = TableItem::from_write_table_item(
                    table_item,
                    index,
                    transaction_version,
                    transaction_block_height,
                );
                (
                    Self {
                        transaction_version,
                        hash: table_item.state_key_hash.clone(),
                        transaction_block_height,
                        type_,
                        address: String::default(),
                        index,
                    },
                    WriteSetChangeDetail::Table(
                        ti,
                        cti,
                        Some(TableMetadata::from_write_table_item(table_item)),
                    ),
                )
            },
            APIWriteSetChange::DeleteTableItem(table_item) => {
                let (ti, cti) = TableItem::from_delete_table_item(
                    table_item,
                    index,
                    transaction_version,
                    transaction_block_height,
                );
                (
                    Self {
                        transaction_version,
                        hash: table_item.state_key_hash.clone(),
                        transaction_block_height,
                        type_,
                        address: String::default(),
                        index,
                    },
                    WriteSetChangeDetail::Table(ti, cti, None),
                )
            },
        }
    }

    pub fn from_write_set_changes(
        write_set_changes: &[APIWriteSetChange],
        transaction_version: i64,
        transaction_block_height: i64,
    ) -> (Vec<Self>, Vec<WriteSetChangeDetail>) {
        write_set_changes
            .iter()
            .enumerate()
            .map(|(index, write_set_change)| {
                Self::from_write_set_change(
                    write_set_change,
                    index as i64,
                    transaction_version,
                    transaction_block_height,
                )
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

#[derive(Deserialize, Serialize)]
pub enum WriteSetChangeDetail {
    Module(MoveModule),
    Resource(MoveResource),
    Table(TableItem, CurrentTableItem, Option<TableMetadata>),
}

// Prevent conflicts with other things named `WriteSetChange`
pub type WriteSetChangeModel = WriteSetChange;
