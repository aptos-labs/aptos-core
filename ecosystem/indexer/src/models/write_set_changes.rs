// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{models::transactions::Transaction, schema::write_set_changes};
use aptos_rest_client::aptos_api_types::WriteSetChange as APIWriteSetChange;
use serde::Serialize;
use serde_json::json;

#[derive(AsChangeset, Associations, Debug, Identifiable, Insertable, Queryable, Serialize)]
#[diesel(table_name = "write_set_changes")]
#[belongs_to(Transaction, foreign_key = "transaction_hash")]
#[primary_key(transaction_hash, hash)]
pub struct WriteSetChange {
    pub transaction_hash: String,
    pub hash: String,
    #[diesel(column_name = type)]
    pub type_: String,
    pub address: String,
    pub module: serde_json::Value,
    pub resource: serde_json::Value,
    pub data: serde_json::Value,

    // Default time columns
    pub inserted_at: chrono::NaiveDateTime,
}

impl WriteSetChange {
    pub fn from_write_set_change(
        transaction_hash: String,
        write_set_change: &APIWriteSetChange,
    ) -> Self {
        match write_set_change {
            APIWriteSetChange::DeleteModule {
                address,
                state_key_hash,
                module,
            } => WriteSetChange {
                transaction_hash,
                hash: state_key_hash.clone(),
                type_: write_set_change.type_str().to_string(),
                address: address.to_string(),
                module: serde_json::to_value(module).unwrap(),
                resource: Default::default(),
                data: Default::default(),
                inserted_at: chrono::Utc::now().naive_utc(),
            },
            APIWriteSetChange::DeleteResource {
                address,
                state_key_hash,
                resource,
            } => WriteSetChange {
                transaction_hash,
                hash: state_key_hash.clone(),
                type_: write_set_change.type_str().to_string(),
                address: address.to_string(),
                module: Default::default(),
                resource: serde_json::to_value(resource).unwrap(),
                data: Default::default(),
                inserted_at: chrono::Utc::now().naive_utc(),
            },
            APIWriteSetChange::DeleteTableItem {
                state_key_hash,
                handle,
                key,
            } => WriteSetChange {
                transaction_hash,
                hash: state_key_hash.clone(),
                type_: write_set_change.type_str().to_string(),
                address: "".to_owned(),
                module: Default::default(),
                resource: Default::default(),
                data: json!({
                    "handle": handle,
                    "key": key,
                }),
                inserted_at: chrono::Utc::now().naive_utc(),
            },
            APIWriteSetChange::WriteModule {
                address,
                state_key_hash,
                data,
            } => WriteSetChange {
                transaction_hash,
                hash: state_key_hash.clone(),
                type_: write_set_change.type_str().to_string(),
                address: address.to_string(),
                module: Default::default(),
                resource: Default::default(),
                data: serde_json::to_value(data).unwrap(),
                inserted_at: chrono::Utc::now().naive_utc(),
            },
            APIWriteSetChange::WriteResource {
                address,
                state_key_hash,
                data,
            } => WriteSetChange {
                transaction_hash,
                hash: state_key_hash.clone(),
                type_: write_set_change.type_str().to_string(),
                address: address.to_string(),
                module: Default::default(),
                resource: Default::default(),
                data: serde_json::to_value(data).unwrap(),
                inserted_at: chrono::Utc::now().naive_utc(),
            },
            APIWriteSetChange::WriteTableItem {
                state_key_hash,
                handle,
                key,
                value,
            } => WriteSetChange {
                transaction_hash,
                hash: state_key_hash.clone(),
                type_: write_set_change.type_str().to_string(),
                address: "".to_owned(),
                module: Default::default(),
                resource: Default::default(),
                data: json!({
                    "handle": handle,
                    "key": key,
                    "value": value,
                }),
                inserted_at: chrono::Utc::now().naive_utc(),
            },
        }
    }

    pub fn from_write_set_changes(
        transaction_hash: String,
        write_set_changes: &[APIWriteSetChange],
    ) -> Option<Vec<Self>> {
        if write_set_changes.is_empty() {
            return None;
        }
        Some(
            write_set_changes
                .iter()
                .map(|write_set_change| {
                    Self::from_write_set_change(transaction_hash.clone(), write_set_change)
                })
                .collect::<Vec<WriteSetChangeModel>>(),
        )
    }
}

// Prevent conflicts with other things named `WriteSetChange`
pub type WriteSetChangeModel = WriteSetChange;
