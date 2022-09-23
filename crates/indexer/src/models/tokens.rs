// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]
#![allow(clippy::unused_unit)]

use std::collections::HashMap;

use super::{
    collection_datas::{CollectionData, CurrentCollectionData},
    move_resources::MoveResource,
    token_datas::{CurrentTokenData, TokenData},
    token_utils::TokenWriteSet,
};
use crate::{
    schema::{current_token_ownerships, token_ownerships, tokens},
    util::{ensure_not_negative, hash_str, truncate_str},
};
use anyhow::Context;
use aptos_api_types::{
    DeleteTableItem as APIDeleteTableItem, Transaction as APITransaction,
    WriteResource as APIWriteResource, WriteSetChange as APIWriteSetChange,
    WriteTableItem as APIWriteTableItem,
};
use aptos_logger::warn;
use bigdecimal::BigDecimal;
use field_count::FieldCount;
use serde::{Deserialize, Serialize};

type TableHandle = String;
type OwnerAddress = String;
type TableType = String;
pub type TableHandleToOwner = HashMap<TableHandle, TableMetadataForToken>;
pub type TokenDataIdHash = String;
// PK of current_token_ownerships, i.e. token_data_id_hash + property_version + owner_address
pub type CurrentTokenOwnershipPK = (TokenDataIdHash, BigDecimal, OwnerAddress);

#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Queryable, Serialize)]
#[diesel(primary_key(token_data_id_hash, property_version, transaction_version))]
#[diesel(table_name = tokens)]
pub struct Token {
    pub token_data_id_hash: String,
    pub property_version: BigDecimal,
    pub transaction_version: i64,
    pub creator_address: String,
    pub collection_name: String,
    pub name: String,
    pub token_properties: serde_json::Value,
    // Default time columns
    pub inserted_at: chrono::NaiveDateTime,
}

#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Queryable, Serialize)]
#[diesel(primary_key(
    token_data_id_hash,
    property_version,
    transaction_version,
    table_handle
))]
#[diesel(table_name = token_ownerships)]
pub struct TokenOwnership {
    pub token_data_id_hash: String,
    pub property_version: BigDecimal,
    pub transaction_version: i64,
    pub table_handle: String,
    pub creator_address: String,
    pub collection_name: String,
    pub name: String,
    pub owner_address: Option<String>,
    pub amount: BigDecimal,
    pub table_type: Option<String>,
    // Default time columns
    pub inserted_at: chrono::NaiveDateTime,
}

#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Queryable, Serialize)]
#[diesel(primary_key(token_data_id_hash, property_version, owner_address))]
#[diesel(table_name = current_token_ownerships)]
pub struct CurrentTokenOwnership {
    pub token_data_id_hash: String,
    pub property_version: BigDecimal,
    pub owner_address: String,
    pub creator_address: String,
    pub collection_name: String,
    pub name: String,
    pub amount: BigDecimal,
    pub token_properties: serde_json::Value,
    pub last_transaction_version: i64,
    // Default time columns
    pub inserted_at: chrono::NaiveDateTime,
}

#[derive(Debug)]
pub struct TableMetadataForToken {
    pub owner_address: OwnerAddress,
    pub table_type: TableType,
}

impl Token {
    /// We can find token data from write sets in user transactions. Table items will contain metadata for collections
    /// and tokens. To find ownership, we have to look in write resource write sets for who owns those table handles
    ///
    /// We also will compute current versions of the token tables which are at a higher granularity than the transactional tables (only
    /// state at the last transaction will be tracked, hence using hashmap to dedupe)
    pub fn from_transaction(
        transaction: &APITransaction,
    ) -> (
        Vec<Self>,
        Vec<TokenOwnership>,
        Vec<TokenData>,
        Vec<CollectionData>,
        HashMap<CurrentTokenOwnershipPK, CurrentTokenOwnership>,
        HashMap<TokenDataIdHash, CurrentTokenData>,
        HashMap<TokenDataIdHash, CurrentCollectionData>,
    ) {
        if let APITransaction::UserTransaction(user_txn) = transaction {
            let mut tokens = vec![];
            let mut token_ownerships = vec![];
            let mut token_datas = vec![];
            let mut collection_datas = vec![];

            let mut current_token_ownerships: HashMap<
                CurrentTokenOwnershipPK,
                CurrentTokenOwnership,
            > = HashMap::new();
            let mut current_token_datas: HashMap<TokenDataIdHash, CurrentTokenData> =
                HashMap::new();
            let mut current_collection_datas: HashMap<TokenDataIdHash, CurrentCollectionData> =
                HashMap::new();

            let txn_version = user_txn.info.version.0 as i64;
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
                if let Some((token, token_ownership, maybe_current_token_ownership)) =
                    maybe_token_w_ownership
                {
                    tokens.push(token);
                    token_ownerships.push(token_ownership);
                    if let Some(current_token_ownership) = maybe_current_token_ownership {
                        current_token_ownerships.insert(
                            (
                                current_token_ownership.token_data_id_hash.clone(),
                                current_token_ownership.property_version.clone(),
                                current_token_ownership.owner_address.clone(),
                            ),
                            current_token_ownership,
                        );
                    }
                }
                if let Some((token_data, current_token_data)) = maybe_token_data {
                    token_datas.push(token_data);
                    current_token_datas.insert(
                        current_token_data.token_data_id_hash.clone(),
                        current_token_data,
                    );
                }
                if let Some((collection_data, current_collection_data)) = maybe_collection_data {
                    collection_datas.push(collection_data);
                    current_collection_datas.insert(
                        current_collection_data.collection_data_id_hash.clone(),
                        current_collection_data,
                    );
                }
            }
            return (
                tokens,
                token_ownerships,
                token_datas,
                collection_datas,
                current_token_ownerships,
                current_token_datas,
                current_collection_datas,
            );
        }
        Default::default()
    }

    /// Get token from write table item. Table items don't have address of the table so we need to look it up in the table_handle_to_owner mapping
    /// We get the mapping from resource.
    /// If the mapping is missing we'll just leave owner address as blank. This isn't great but at least helps us account for the token
    pub fn from_write_table_item(
        table_item: &APIWriteTableItem,
        txn_version: i64,
        table_handle_to_owner: &TableHandleToOwner,
    ) -> anyhow::Result<Option<(Self, TokenOwnership, Option<CurrentTokenOwnership>)>> {
        let table_item_data = table_item.data.as_ref().unwrap();

        let maybe_token = match TokenWriteSet::from_table_item_type(
            table_item_data.value_type.as_str(),
            &table_item_data.value,
            txn_version,
        )? {
            Some(TokenWriteSet::Token(inner)) => Some(inner),
            _ => None,
        };

        if let Some(token) = maybe_token {
            let table_handle =
                TableMetadataForToken::standardize_handle(&table_item.handle.to_string());
            let table_handle_metadata = table_handle_to_owner.get(&table_handle);
            let (owner_address, table_type) = match table_handle_metadata {
                Some(metadata) => (
                    Some(metadata.owner_address.clone()),
                    Some(metadata.table_type.clone()),
                ),
                None => {
                    warn!(
                        "Missing table handle metadata for token. Version: {}, table handle for TokenStore: {}, all metadata: {:?}",
                        txn_version, table_handle, table_handle_to_owner
                    );
                    (None, None)
                }
            };
            let token_id = token.id;
            let token_data_id = token_id.token_data_id;
            let token_data_id_hash = hash_str(&token_data_id.to_string());
            let collection_name = truncate_str(&token_data_id.collection, 128);
            let name = truncate_str(&token_data_id.name, 128);

            let curr_token_ownership = match &owner_address {
                Some(owner_address) => Some(CurrentTokenOwnership {
                    token_data_id_hash: token_data_id_hash.clone(),
                    property_version: token_id.property_version.clone(),
                    owner_address: owner_address.clone(),
                    creator_address: token_data_id.creator.clone(),
                    collection_name: collection_name.clone(),
                    name: name.clone(),
                    amount: ensure_not_negative(token.amount.clone()),
                    token_properties: token.token_properties.clone(),
                    last_transaction_version: txn_version,
                    inserted_at: chrono::Utc::now().naive_utc(),
                }),
                None => None,
            };

            Ok(Some((
                Self {
                    token_data_id_hash: token_data_id_hash.clone(),
                    creator_address: token_data_id.creator.clone(),
                    collection_name: collection_name.clone(),
                    name: name.clone(),
                    property_version: token_id.property_version.clone(),
                    transaction_version: txn_version,
                    token_properties: token.token_properties.clone(),
                    inserted_at: chrono::Utc::now().naive_utc(),
                },
                TokenOwnership {
                    token_data_id_hash,
                    creator_address: token_data_id.creator,
                    collection_name,
                    name,
                    property_version: token_id.property_version,
                    transaction_version: txn_version,
                    owner_address,
                    amount: ensure_not_negative(token.amount),
                    table_handle,
                    table_type,
                    inserted_at: chrono::Utc::now().naive_utc(),
                },
                curr_token_ownership,
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
    ) -> anyhow::Result<Option<(Self, TokenOwnership, Option<CurrentTokenOwnership>)>> {
        let table_item_data = table_item.data.as_ref().unwrap();

        let maybe_token_id = match TokenWriteSet::from_table_item_type(
            table_item_data.key_type.as_str(),
            &table_item_data.key,
            txn_version,
        )? {
            Some(TokenWriteSet::TokenId(inner)) => Some(inner),
            _ => None,
        };

        if let Some(token_id) = maybe_token_id {
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
            let token_data_id = token_id.token_data_id;
            let token_data_id_hash = hash_str(&token_data_id.to_string());
            let collection_name = truncate_str(&token_data_id.collection, 128);
            let name = truncate_str(&token_data_id.name, 128);

            let curr_token_ownership = match &owner_address {
                Some(owner_address) => Some(CurrentTokenOwnership {
                    token_data_id_hash: token_data_id_hash.clone(),
                    property_version: token_id.property_version.clone(),
                    owner_address: owner_address.clone(),
                    creator_address: token_data_id.creator.clone(),
                    collection_name: collection_name.clone(),
                    name: name.clone(),
                    amount: BigDecimal::default(),
                    token_properties: serde_json::Value::Null,
                    last_transaction_version: txn_version,
                    inserted_at: chrono::Utc::now().naive_utc(),
                }),
                None => None,
            };

            Ok(Some((
                Self {
                    token_data_id_hash: token_data_id_hash.clone(),
                    creator_address: token_data_id.creator.clone(),
                    collection_name: collection_name.clone(),
                    name: name.clone(),
                    property_version: token_id.property_version.clone(),
                    transaction_version: txn_version,
                    token_properties: serde_json::Value::Null,
                    inserted_at: chrono::Utc::now().naive_utc(),
                },
                TokenOwnership {
                    token_data_id_hash,
                    creator_address: token_data_id.creator,
                    collection_name,
                    name,
                    property_version: token_id.property_version,
                    transaction_version: txn_version,
                    owner_address,
                    amount: BigDecimal::default(),
                    table_handle,
                    table_type,
                    inserted_at: chrono::Utc::now().naive_utc(),
                },
                curr_token_ownership,
            )))
        } else {
            Ok(None)
        }
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
