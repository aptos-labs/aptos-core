// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]
#![allow(clippy::unused_unit)]

use super::token_models::v2_token_utils::ObjectWithMetadata;
use crate::{
    models::move_resources::MoveResource,
    schema::{current_objects, objects},
};
use aptos_api_types::{DeleteResource, Transaction, WriteResource, WriteSetChange};
use bigdecimal::BigDecimal;
use field_count::FieldCount;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// PK of current_objects, i.e. object_address
pub type CurrentObjectPK = String;

#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(transaction_version, write_set_change_index))]
#[diesel(table_name = objects)]
pub struct Object {
    pub transaction_version: i64,
    pub write_set_change_index: i64,
    pub object_address: String,
    pub owner_address: Option<String>,
    pub state_key_hash: String,
    pub guid_creation_num: Option<BigDecimal>,
    pub allow_ungated_transfer: Option<bool>,
    pub is_deleted: bool,
}

#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(object_address))]
#[diesel(table_name = current_objects)]
pub struct CurrentObject {
    pub object_address: String,
    pub owner_address: Option<String>,
    pub state_key_hash: String,
    pub allow_ungated_transfer: Option<bool>,
    pub last_guid_creation_num: Option<BigDecimal>,
    pub last_transaction_version: i64,
    pub is_deleted: bool,
}

impl Object {
    /// Only parsing 0x1 ObjectCore from transactions
    pub fn from_transaction(
        transaction: &Transaction,
    ) -> (Vec<Self>, HashMap<CurrentObjectPK, CurrentObject>) {
        if let Transaction::UserTransaction(user_txn) = transaction {
            let mut objects = vec![];
            let mut current_objects: HashMap<String, CurrentObject> = HashMap::new();
            let txn_version = user_txn.info.version.0 as i64;

            for (index, wsc) in user_txn.info.changes.iter().enumerate() {
                let index = index as i64;
                let maybe_object_combo = match wsc {
                    WriteSetChange::DeleteResource(inner) => {
                        Self::from_delete_resource(inner, txn_version, index).unwrap()
                    },
                    WriteSetChange::WriteResource(inner) => {
                        Self::from_write_resource(inner, txn_version, index).unwrap()
                    },
                    _ => None,
                };
                if let Some((object, current_object)) = maybe_object_combo {
                    objects.push(object);
                    current_objects.insert(current_object.object_address.clone(), current_object);
                }
            }
            (objects, current_objects)
        } else {
            Default::default()
        }
    }

    fn from_write_resource(
        write_resource: &WriteResource,
        txn_version: i64,
        write_set_change_index: i64,
    ) -> anyhow::Result<Option<(Self, CurrentObject)>> {
        if let Some(inner) = ObjectWithMetadata::from_write_resource(write_resource, txn_version)? {
            let resource = MoveResource::from_write_resource(
                write_resource,
                0, // Placeholder, this isn't used anyway
                txn_version,
                0, // Placeholder, this isn't used anyway
            );
            let object_core = &inner.object_core;
            Ok(Some((
                Self {
                    transaction_version: txn_version,
                    write_set_change_index,
                    object_address: resource.address.clone(),
                    owner_address: Some(object_core.get_owner_address()),
                    state_key_hash: resource.state_key_hash.clone(),
                    guid_creation_num: Some(object_core.guid_creation_num.clone()),
                    allow_ungated_transfer: Some(object_core.allow_ungated_transfer),
                    is_deleted: false,
                },
                CurrentObject {
                    object_address: resource.address,
                    owner_address: Some(object_core.get_owner_address()),
                    state_key_hash: resource.state_key_hash,
                    allow_ungated_transfer: Some(object_core.allow_ungated_transfer),
                    last_guid_creation_num: Some(object_core.guid_creation_num.clone()),
                    last_transaction_version: txn_version,
                    is_deleted: false,
                },
            )))
        } else {
            Ok(None)
        }
    }

    /// This should never really happen since it's very difficult to delete the entire resource group
    /// currently. We actually need a better way of detecting whether an object is deleted since there
    /// is likely no delete resource write set change.
    fn from_delete_resource(
        delete_resource: &DeleteResource,
        txn_version: i64,
        write_set_change_index: i64,
    ) -> anyhow::Result<Option<(Self, CurrentObject)>> {
        if delete_resource.resource.to_string() == "0x1::object::ObjectCore" {
            let resource = MoveResource::from_delete_resource(
                delete_resource,
                0, // Placeholder, this isn't used anyway
                txn_version,
                0, // Placeholder, this isn't used anyway
            );
            Ok(Some((
                Self {
                    transaction_version: txn_version,
                    write_set_change_index,
                    object_address: resource.address.clone(),
                    owner_address: None,
                    state_key_hash: resource.state_key_hash.clone(),
                    guid_creation_num: None,
                    allow_ungated_transfer: None,
                    is_deleted: true,
                },
                CurrentObject {
                    object_address: resource.address,
                    owner_address: None,
                    state_key_hash: resource.state_key_hash,
                    allow_ungated_transfer: None,
                    last_guid_creation_num: None,
                    last_transaction_version: txn_version,
                    is_deleted: true,
                },
            )))
        } else {
            Ok(None)
        }
    }
}
