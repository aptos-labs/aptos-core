// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]
#![allow(clippy::unused_unit)]

use crate::{
    schema::{collection_datas, current_collection_datas},
    util::{hash_str, truncate_str},
};
use anyhow::Context;
use aptos_api_types::WriteTableItem as APIWriteTableItem;
use bigdecimal::BigDecimal;
use field_count::FieldCount;
use serde::{Deserialize, Serialize};

use super::{
    token_utils::{CollectionDataIdType, TokenWriteSet},
    tokens::{TableHandleToOwner, TableMetadataForToken},
};

#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Queryable, Serialize)]
#[diesel(primary_key(collection_data_id_hash, transaction_version))]
#[diesel(table_name = collection_datas)]
pub struct CollectionData {
    pub collection_data_id_hash: String,
    pub transaction_version: i64,
    pub creator_address: String,
    pub collection_name: String,
    pub description: String,
    pub metadata_uri: String,
    pub supply: BigDecimal,
    pub maximum: BigDecimal,
    pub maximum_mutable: bool,
    pub uri_mutable: bool,
    pub description_mutable: bool,
    // Default time columns
    pub inserted_at: chrono::NaiveDateTime,
}

#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Queryable, Serialize)]
#[diesel(primary_key(collection_data_id_hash))]
#[diesel(table_name = current_collection_datas)]
pub struct CurrentCollectionData {
    pub collection_data_id_hash: String,
    pub creator_address: String,
    pub collection_name: String,
    pub description: String,
    pub metadata_uri: String,
    pub supply: BigDecimal,
    pub maximum: BigDecimal,
    pub maximum_mutable: bool,
    pub uri_mutable: bool,
    pub description_mutable: bool,
    pub last_transaction_version: i64,
    // Default time columns
    pub inserted_at: chrono::NaiveDateTime,
}

impl CollectionData {
    pub fn from_write_table_item(
        table_item: &APIWriteTableItem,
        txn_version: i64,
        table_handle_to_owner: &TableHandleToOwner,
    ) -> anyhow::Result<Option<(Self, CurrentCollectionData)>> {
        let table_item_data = table_item.data.as_ref().unwrap();

        let maybe_collection_data = match TokenWriteSet::from_table_item_type(
            table_item_data.value_type.as_str(),
            &table_item_data.value,
            txn_version,
        )? {
            Some(TokenWriteSet::CollectionData(inner)) => Some(inner),
            _ => None,
        };
        if let Some(collection_data) = maybe_collection_data {
            let table_handle = table_item.handle.to_string();
            let creator_address = table_handle_to_owner
                            .get(&TableMetadataForToken::standardize_handle(&table_handle))
                            .map(|table_metadata| table_metadata.owner_address.clone())
                            .context(format!(
                                "version {} failed! collection creator resource was missing, table handle {} not in map {:?}",
                                txn_version, TableMetadataForToken::standardize_handle(&table_handle), table_handle_to_owner,
                            ))?;
            let collection_data_id = CollectionDataIdType {
                creator: creator_address,
                name: collection_data.name,
            };
            let collection_data_id_hash = hash_str(&collection_data_id.to_string());
            let collection_name = truncate_str(&collection_data_id.name, 128);
            let metadata_uri = truncate_str(&collection_data.uri, 512);

            Ok(Some((
                Self {
                    collection_data_id_hash: collection_data_id_hash.clone(),
                    collection_name: collection_name.clone(),
                    creator_address: collection_data_id.creator.clone(),
                    description: collection_data.description.clone(),
                    transaction_version: txn_version,
                    metadata_uri: metadata_uri.clone(),
                    supply: collection_data.supply.clone(),
                    maximum: collection_data.maximum.clone(),
                    maximum_mutable: collection_data.mutability_config.maximum,
                    uri_mutable: collection_data.mutability_config.uri,
                    description_mutable: collection_data.mutability_config.description,
                    inserted_at: chrono::Utc::now().naive_utc(),
                },
                CurrentCollectionData {
                    collection_data_id_hash,
                    collection_name,
                    creator_address: collection_data_id.creator,
                    description: collection_data.description,
                    metadata_uri,
                    supply: collection_data.supply,
                    maximum: collection_data.maximum,
                    maximum_mutable: collection_data.mutability_config.maximum,
                    uri_mutable: collection_data.mutability_config.uri,
                    description_mutable: collection_data.mutability_config.description,
                    last_transaction_version: txn_version,
                    inserted_at: chrono::Utc::now().naive_utc(),
                },
            )))
        } else {
            Ok(None)
        }
    }
}
