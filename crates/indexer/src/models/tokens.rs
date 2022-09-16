// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]
#![allow(clippy::unused_unit)]

use std::collections::HashMap;

use super::{
    collection_datas::CollectionData,
    move_resources::MoveResource,
    token_datas::{TokenData, TokenDataId},
};
use crate::{
    schema::{token_ownerships, tokens},
    util::{ensure_not_negative, hash_str, u64_to_bigdecimal},
};
use anyhow::Context;
use aptos_api_types::{
    DeleteTableItem as APIDeleteTableItem, Transaction as APITransaction,
    WriteResource as APIWriteResource, WriteSetChange as APIWriteSetChange,
    WriteTableItem as APIWriteTableItem,
};
use bigdecimal::BigDecimal;
use field_count::FieldCount;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Queryable, Serialize)]
#[primary_key(
    creator_address,
    collection_name_hash,
    name_hash,
    property_version,
    transaction_version
)]
#[diesel(table_name = "tokens")]
pub struct Token {
    pub creator_address: String,
    pub collection_name_hash: String,
    pub name_hash: String,
    pub collection_name: String,
    pub name: String,
    pub property_version: bigdecimal::BigDecimal,
    pub transaction_version: i64,
    pub token_properties: serde_json::Value,
    // Default time columns
    pub inserted_at: chrono::NaiveDateTime,
}

#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Queryable, Serialize)]
#[primary_key(
    creator_address,
    collection_name_hash,
    name_hash,
    property_version,
    transaction_version,
    table_handle
)]
#[diesel(table_name = "token_ownership")]
pub struct TokenOwnership {
    pub creator_address: String,
    pub collection_name_hash: String,
    pub name_hash: String,
    pub collection_name: String,
    pub name: String,
    pub property_version: bigdecimal::BigDecimal,
    pub transaction_version: i64,
    pub owner_address: Option<String>,
    pub amount: bigdecimal::BigDecimal,
    pub table_handle: String,
    pub table_type: Option<String>,
    // Default time columns
    pub inserted_at: chrono::NaiveDateTime,
}

#[derive(Debug)]
pub struct TableMetadataForToken {
    pub owner_address: OwnerAddress,
    pub table_type: TableType,
}
type TableHandle = String;
type OwnerAddress = String;
type TableType = String;
pub type TableHandleToOwner = HashMap<TableHandle, TableMetadataForToken>;

impl Token {
    /// We can find token data from write sets in user transactions. Table items will contain metadata for collections
    /// and tokens. To find ownership, we have to look in write resource write sets for who owns those table handles
    pub fn from_transaction(
        transaction: &APITransaction,
    ) -> (
        Vec<Self>,
        Vec<TokenOwnership>,
        Vec<TokenData>,
        Vec<CollectionData>,
    ) {
        if let APITransaction::UserTransaction(user_txn) = transaction {
            let mut tokens = vec![];
            let mut token_ownerships = vec![];
            let mut token_datas = vec![];
            let mut collection_datas = vec![];

            let txn_version = *user_txn.info.version.inner() as i64;
            let mut table_handle_to_owner: TableHandleToOwner = HashMap::new();
            for wsc in &user_txn.info.changes {
                if let APIWriteSetChange::WriteResource(write_resource) = wsc {
                    let maybe_map = TableMetadataForToken::get_table_handle_to_owner(
                        write_resource,
                        txn_version,
                    )
                    .unwrap();
                    if let Some(map) = maybe_map {
                        table_handle_to_owner.extend(map);
                    }
                }
            }

            for wsc in &user_txn.info.changes {
                let (maybe_token_w_ownership, maybe_token_data, maybe_collection_data) = match wsc {
                    APIWriteSetChange::WriteTableItem(write_table_item) => (
                        Self::from_write_table_item(
                            write_table_item,
                            txn_version,
                            &table_handle_to_owner,
                        )
                        .unwrap(),
                        TokenData::from_write_table_item(write_table_item, txn_version).unwrap(),
                        CollectionData::from_write_table_item(
                            write_table_item,
                            txn_version,
                            &table_handle_to_owner,
                        )
                        .unwrap(),
                    ),
                    APIWriteSetChange::DeleteTableItem(delete_table_item) => (
                        Self::from_delete_table_item(
                            delete_table_item,
                            txn_version,
                            &table_handle_to_owner,
                        )
                        .unwrap(),
                        None,
                        None,
                    ),
                    _ => (None, None, None),
                };
                if let Some((token, token_ownership)) = maybe_token_w_ownership {
                    tokens.push(token);
                    token_ownerships.push(token_ownership);
                }
                if let Some(token_data) = maybe_token_data {
                    token_datas.push(token_data);
                }
                if let Some(collection_data) = maybe_collection_data {
                    collection_datas.push(collection_data);
                }
            }
            return (tokens, token_ownerships, token_datas, collection_datas);
        }
        (vec![], vec![], vec![], vec![])
    }

    /// Get token from write table item. Table items don't have address of the table so we need to look it up in the table_handle_to_owner mapping
    /// We get the mapping from resource.
    /// If the mapping is missing we'll just leave owner address as blank. This isn't great but at least helps us account for the token
    pub fn from_write_table_item(
        table_item: &APIWriteTableItem,
        txn_version: i64,
        table_handle_to_owner: &TableHandleToOwner,
    ) -> anyhow::Result<Option<(Self, TokenOwnership)>> {
        let table_item_data = table_item.data.as_ref().unwrap();
        if table_item_data.key_type != "0x3::token::TokenId" {
            return Ok(None);
        }
        let table_handle =
            TableMetadataForToken::standardize_handle(&table_item.handle.to_string());
        let (owner_address, table_type) = table_handle_to_owner
            .get(&table_handle)
            .map(|table_metadata| {
                (
                    Some(table_metadata.owner_address.clone()),
                    Some(table_metadata.table_type.clone()),
                )
            })
            .unwrap_or((None, None));
        let key = &table_item_data.key;
        let token_data_id = Self::get_token_data_id_from_table_item_key(key, txn_version)?;
        let property_version = Self::get_property_version_from_table_item_key(key, txn_version)?;

        let collection_name_hash = hash_str(&token_data_id.collection_name);
        let name_hash = hash_str(&token_data_id.name);

        if table_item_data.value_type == "0x3::token::Token" {
            let value = &table_item_data.value;
            Ok(Some((
                Self {
                    creator_address: token_data_id.creator_address.clone(),
                    collection_name_hash: collection_name_hash.clone(),
                    name_hash: name_hash.clone(),
                    collection_name: token_data_id.collection_name.clone(),
                    name: token_data_id.name.clone(),
                    property_version: property_version.clone(),
                    transaction_version: txn_version,
                    token_properties: value["token_properties"].clone(),
                    inserted_at: chrono::Utc::now().naive_utc(),
                },
                TokenOwnership {
                    creator_address: token_data_id.creator_address.clone(),
                    collection_name: token_data_id.collection_name.clone(),
                    collection_name_hash,
                    name_hash,
                    name: token_data_id.name,
                    property_version,
                    transaction_version: txn_version,
                    owner_address,
                    amount: value["amount"]
                        .as_str()
                        .map(|s| -> anyhow::Result<BigDecimal> {
                            Ok(ensure_not_negative(u64_to_bigdecimal(s.parse::<u64>()?)))
                        })
                        .context(format!(
                            "version {} failed! amount missing from token {:?}",
                            txn_version, value
                        ))?
                        .context(format!("failed to parse amount {:?}", value["amount"]))?,
                    table_handle,
                    table_type,
                    inserted_at: chrono::Utc::now().naive_utc(),
                },
            )))
        } else {
            Ok(None)
        }
    }

    /// Get token from delete table item. The difference from write table item is that value isn't there so
    /// we'll set amount to 0 and token property to blank.
    pub fn from_delete_table_item(
        table_item: &APIDeleteTableItem,
        txn_version: i64,
        table_handle_to_owner: &TableHandleToOwner,
    ) -> anyhow::Result<Option<(Self, TokenOwnership)>> {
        let table_item_data = table_item.data.as_ref().unwrap();
        if table_item_data.key_type != "0x3::token::TokenId" {
            return Ok(None);
        }
        let table_handle =
            TableMetadataForToken::standardize_handle(&table_item.handle.to_string());
        let (owner_address, table_type) = table_handle_to_owner
            .get(&table_handle)
            .map(|table_metadata| {
                (
                    Some(table_metadata.owner_address.clone()),
                    Some(table_metadata.table_type.clone()),
                )
            })
            .unwrap_or((None, None));
        let key = &table_item_data.key;
        let token_data_id = Self::get_token_data_id_from_table_item_key(key, txn_version)?;
        let property_version = Self::get_property_version_from_table_item_key(key, txn_version)?;

        let collection_name_hash = hash_str(&token_data_id.collection_name);
        let name_hash = hash_str(&token_data_id.name);

        Ok(Some((
            Self {
                creator_address: token_data_id.creator_address.clone(),
                collection_name: token_data_id.collection_name.clone(),
                name: token_data_id.name.clone(),
                collection_name_hash: collection_name_hash.clone(),
                name_hash: name_hash.clone(),
                property_version: property_version.clone(),
                transaction_version: txn_version,
                token_properties: serde_json::Value::Null,
                inserted_at: chrono::Utc::now().naive_utc(),
            },
            TokenOwnership {
                creator_address: token_data_id.creator_address.clone(),
                collection_name: token_data_id.collection_name.clone(),
                name: token_data_id.name,
                collection_name_hash,
                name_hash,
                property_version,
                transaction_version: txn_version,
                owner_address,
                amount: BigDecimal::default(),
                table_handle,
                table_type,
                inserted_at: chrono::Utc::now().naive_utc(),
            },
        )))
    }

    fn get_property_version_from_table_item_key(
        key: &serde_json::Value,
        txn_version: i64,
    ) -> anyhow::Result<BigDecimal> {
        key["property_version"]
            .as_str()
            .map(|s| -> anyhow::Result<BigDecimal> { Ok(u64_to_bigdecimal(s.parse::<u64>()?)) })
            .context(format!(
                "version {} failed! token_data_id.property_version missing from token id {:?}",
                txn_version, key
            ))?
            .context(format!(
                "version {} failed! failed to parse property_version {:?}",
                txn_version, key["property_version"]
            ))
    }

    fn get_token_data_id_from_table_item_key(
        key: &serde_json::Value,
        txn_version: i64,
    ) -> anyhow::Result<TokenDataId> {
        Ok(TokenDataId {
            creator_address: key["token_data_id"]["creator"]
                .as_str()
                .map(|s| s.to_string())
                .context(format!(
                    "version {} failed! token_data_id.creator missing from token_id {:?}",
                    txn_version, key
                ))?,
            collection_name: key["token_data_id"]["collection"]
                .as_str()
                .map(|s| s.to_string())
                .context(format!(
                    "version {} failed! token_data_id.collection missing from token_id {:?}",
                    txn_version, key
                ))?,
            name: key["token_data_id"]["name"]
                .as_str()
                .map(|s| s.to_string())
                .context(format!(
                    "version {} failed! name missing from token_id {:?}",
                    txn_version, key
                ))?,
        })
    }
}

impl TableMetadataForToken {
    /// Helper to get destructured data from table metadata by handle
    pub fn get_owner_and_type(
        mapping: &TableHandleToOwner,
        table_handle: &TableHandle,
    ) -> (Option<OwnerAddress>, Option<TableType>) {
        mapping
            .get(table_handle)
            .map(|table_metadata| {
                (
                    Some(table_metadata.owner_address.clone()),
                    Some(table_metadata.table_type.clone()),
                )
            })
            .unwrap_or((None, None))
    }

    /// Mapping from table handle to owner type, including type of the table (AKA resource type)
    fn get_table_handle_to_owner(
        write_resource: &APIWriteResource,
        txn_version: i64,
    ) -> anyhow::Result<Option<TableHandleToOwner>> {
        let type_str = write_resource.data.typ.to_string();
        if !matches!(
            type_str.as_str(),
            "0x3::token::Collections" | "0x3::token::TokenStore"
        ) {
            return Ok(None);
        }
        let resource = MoveResource::from_write_resource(
            write_resource,
            0,
            txn_version,
            0, // Placeholder, this isn't used anyway
        );

        let maybe_key_value: Option<(TableHandle, TableMetadataForToken)> = match type_str.as_str()
        {
            "0x3::token::Collections" => match &resource.data {
                Some(data) => {
                    let owner_address = resource.address.clone();
                    let collection_handle = data["collection_data"]["handle"]
                        .as_str()
                        .map(|s| s.to_string())
                        .context(format!(
                            "version {} failed! collection data handle must be present {:?}",
                            txn_version, data
                        ))?;
                    Some((
                        Self::standardize_handle(&collection_handle),
                        TableMetadataForToken {
                            owner_address,
                            table_type: resource.type_.clone(),
                        },
                    ))
                }
                None => None,
            },
            "0x3::token::TokenStore" => match &resource.data {
                Some(data) => {
                    let address = &resource.address;
                    let token_store_handle = data["tokens"]["handle"]
                        .as_str()
                        .map(|s| s.to_string())
                        .context(format!(
                            "version {} failed! token store data handle must be present {:?}",
                            txn_version, data
                        ))?;
                    Some((
                        Self::standardize_handle(&token_store_handle),
                        TableMetadataForToken {
                            owner_address: address.clone(),
                            table_type: resource.type_.clone(),
                        },
                    ))
                }
                None => None,
            },
            _ => None,
        };
        if let Some((key, value)) = maybe_key_value {
            Ok(Some(HashMap::from([(key, value)])))
        } else {
            Ok(None)
        }
    }

    /// Removes leading 0s after 0x in a table to standardize between resources and table items
    pub fn standardize_handle(handle: &str) -> String {
        format!("0x{}", &handle[2..].trim_start_matches('0'))
    }
}
