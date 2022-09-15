// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]
#![allow(clippy::unused_unit)]

use crate::{
    schema::collection_datas,
    util::{hash_str, u64_to_bigdecimal},
};
use anyhow::Context;
use aptos_api_types::WriteTableItem as APIWriteTableItem;
use bigdecimal::BigDecimal;
use field_count::FieldCount;
use serde::Serialize;

use super::tokens::{TableHandleToOwner, TableMetadataForToken};

#[derive(Debug, FieldCount, Identifiable, Insertable, Queryable, Serialize)]
#[primary_key(creator_address, collection_name_hash, transaction_version)]
#[diesel(table_name = "collection_datas")]
pub struct CollectionData {
    pub creator_address: String,
    pub collection_name_hash: String,
    pub collection_name: String,
    pub description: String,
    pub transaction_version: i64,
    pub metadata_uri: String,
    pub supply: bigdecimal::BigDecimal,
    pub maximum: bigdecimal::BigDecimal,
    pub maximum_mutable: bool,
    pub uri_mutable: bool,
    pub description_mutable: bool,
    // Default time columns
    pub inserted_at: chrono::NaiveDateTime,
}

impl CollectionData {
    pub fn from_write_table_item(
        table_item: &APIWriteTableItem,
        txn_version: i64,
        table_handle_to_owner: &TableHandleToOwner,
    ) -> anyhow::Result<Option<Self>> {
        let table_item_data = table_item.data.as_ref().unwrap();
        if table_item_data.value_type != "0x3::token::CollectionData" {
            return Ok(None);
        }

        let value = &table_item_data.value;
        let table_handle = table_item.handle.to_string();
        let creator_address = table_handle_to_owner
                .get(&TableMetadataForToken::standardize_handle(&table_handle))
                .map(|table_metadata| table_metadata.owner_address.clone())
                .context(format!(
                    "version {} failed! collection creator resource was missing, table handle {} not in map {:?}",
                    txn_version, TableMetadataForToken::standardize_handle(&table_handle), table_handle_to_owner,
                ))?;
        let collection_name = value["name"]
            .as_str()
            .map(|s| s.to_string())
            .context(format!(
                "version {} failed! name missing from collection {:?}",
                txn_version, value
            ))?;
        let collection_name_hash = hash_str(&collection_name);

        Ok(Some(Self {
            collection_name,
            creator_address,
            collection_name_hash,
            description: value["description"]
                .as_str()
                .map(|s| s.to_string())
                .context(format!(
                    "version {} failed! description missing from collection {:?}",
                    txn_version, value
                ))?,
            transaction_version: txn_version,
            metadata_uri: value["uri"]
                .as_str()
                .map(|s| s.to_string())
                .context(format!(
                    "version {} failed! uri missing from collection {:?}",
                    txn_version, value
                ))?,
            supply: value["supply"]
                .as_str()
                .map(|s| -> anyhow::Result<BigDecimal> { Ok(u64_to_bigdecimal(s.parse::<u64>()?)) })
                .context(format!(
                    "version {} failed! supply missing from collection {:?}",
                    txn_version, value
                ))?
                .context(format!(
                    "version {} failed! failed to parse supply {:?}",
                    txn_version, value["supply"]
                ))?,
            maximum: value["maximum"]
                .as_str()
                .map(|s| -> anyhow::Result<BigDecimal> { Ok(u64_to_bigdecimal(s.parse::<u64>()?)) })
                .context(format!(
                    "version {} failed! maximum missing from collection {:?}",
                    txn_version, value
                ))?
                .context(format!(
                    "version {} failed! failed to parse maximum {:?}",
                    txn_version, value["maximum"]
                ))?,
            maximum_mutable: value["mutability_config"]["maximum"]
                .as_bool()
                .context(format!(
                    "version {} failed! mutability_config.maximum missing {:?}",
                    txn_version, value
                ))?,
            uri_mutable: value["mutability_config"]["uri"]
                .as_bool()
                .context(format!(
                    "version {} failed! mutability_config.uri missing {:?}",
                    txn_version, value
                ))?,
            description_mutable: value["mutability_config"]["description"]
                .as_bool()
                .context(format!(
                    "version {} failed! mutability_config.description missing {:?}",
                    txn_version, value
                ))?,
            inserted_at: chrono::Utc::now().naive_utc(),
        }))
    }
}
