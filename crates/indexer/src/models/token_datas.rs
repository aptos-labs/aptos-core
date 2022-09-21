// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]
#![allow(clippy::unused_unit)]

use crate::{
    schema::token_datas,
    util::{hash_str, u64_to_bigdecimal},
};
use anyhow::Context;
use aptos_api_types::WriteTableItem as APIWriteTableItem;
use bigdecimal::BigDecimal;
use field_count::FieldCount;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Queryable, Serialize)]
#[diesel(primary_key(creator_address, collection_name_hash, name_hash, transaction_version))]
#[diesel(table_name = token_datas)]
pub struct TokenData {
    pub creator_address: String,
    pub collection_name_hash: String,
    pub name_hash: String,
    pub collection_name: String,
    pub name: String,
    pub transaction_version: i64,
    pub maximum: BigDecimal,
    pub supply: BigDecimal,
    pub largest_property_version: BigDecimal,
    pub metadata_uri: String,
    pub payee_address: String,
    pub royalty_points_numerator: BigDecimal,
    pub royalty_points_denominator: BigDecimal,
    pub maximum_mutable: bool,
    pub uri_mutable: bool,
    pub description_mutable: bool,
    pub properties_mutable: bool,
    pub royalty_mutable: bool,
    pub default_properties: serde_json::Value,
    // Default time columns
    pub inserted_at: chrono::NaiveDateTime,
}

pub struct TokenDataId {
    pub creator_address: String,
    pub collection_name: String,
    pub name: String,
}

impl TokenData {
    pub fn from_write_table_item(
        table_item: &APIWriteTableItem,
        txn_version: i64,
    ) -> anyhow::Result<Option<Self>> {
        let table_item_data = table_item.data.as_ref().unwrap();
        if table_item_data.value_type != "0x3::token::TokenData" {
            return Ok(None);
        }
        let key = &table_item_data.key;
        let value = &table_item_data.value;
        let token_data_id = Self::get_token_data_id_from_table_item_key(key, txn_version)?;

        let collection_name_hash = hash_str(&token_data_id.collection_name);
        let name_hash = hash_str(&token_data_id.name);

        Ok(Some(Self {
            creator_address: token_data_id.creator_address,
            collection_name_hash,
            name_hash,
            collection_name: token_data_id.collection_name,
            name: token_data_id.name,
            transaction_version: txn_version,
            maximum: value["maximum"]
                .as_str()
                .map(|s| -> anyhow::Result<BigDecimal> { Ok(u64_to_bigdecimal(s.parse::<u64>()?)) })
                .context(format!(
                    "version {} failed! maximum missing from token data {:?}",
                    txn_version, value
                ))?
                .context(format!(
                    "version {} failed! failed to parse maximum {:?}",
                    txn_version, value["maximum"]
                ))?,
            supply: value["supply"]
                .as_str()
                .map(|s| -> anyhow::Result<BigDecimal> { Ok(u64_to_bigdecimal(s.parse::<u64>()?)) })
                .context(format!(
                    "version {} failed! supply missing from token data {:?}",
                    txn_version, value
                ))?
                .context(format!(
                    "version {} failed! failed to parse supply {:?}",
                    txn_version, value["maximum"]
                ))?,
            largest_property_version: value["largest_property_version"]
                .as_str()
                .map(|s| -> anyhow::Result<BigDecimal> { Ok(u64_to_bigdecimal(s.parse::<u64>()?)) })
                .context(format!(
                    "version {} failed! largest_property_version missing from token data {:?}",
                    txn_version, value
                ))?
                .context(format!(
                    "version {} failed! failed to parse largest_property_version {:?}",
                    txn_version, value["maximum"]
                ))?,
            metadata_uri: value["uri"]
                .as_str()
                .map(|s| s.to_string())
                .context(format!(
                    "version {} failed! uri missing from token data {:?}",
                    txn_version, value
                ))?,
            payee_address: value["royalty"]["payee_address"]
                .as_str()
                .map(|s| s.to_string())
                .context(format!(
                    "version {} failed! royalty.payee_address missing {:?}",
                    txn_version, value
                ))?,
            royalty_points_numerator: value["royalty"]["royalty_points_numerator"]
                .as_str()
                .map(|s| -> anyhow::Result<BigDecimal> { Ok(u64_to_bigdecimal(s.parse::<u64>()?)) })
                .context(format!(
                    "version {} failed! royalty.royalty_points_numerator missing {:?}",
                    txn_version, value
                ))?
                .context(format!(
                    "version {} failed! failed to parse royalty_points_numerator {:?}",
                    txn_version, value["royalty"]["royalty_points_numerator"]
                ))?,
            royalty_points_denominator: value["royalty"]["royalty_points_denominator"]
                .as_str()
                .map(|s| -> anyhow::Result<BigDecimal> { Ok(u64_to_bigdecimal(s.parse::<u64>()?)) })
                .context(format!(
                    "version {} failed! royalty.royalty_points_denominator missing {:?}",
                    txn_version, value
                ))?
                .context(format!(
                    "version {} failed! failed to parse royalty_points_denominator {:?}",
                    txn_version, value["royalty"]["royalty_points_denominator"]
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
            properties_mutable: value["mutability_config"]["properties"].as_bool().context(
                format!(
                    "version {} failed! mutability_config.properties missing {:?}",
                    txn_version, value
                ),
            )?,
            royalty_mutable: value["mutability_config"]["royalty"]
                .as_bool()
                .context(format!(
                    "version {} failed! mutability_config.royalty missing {:?}",
                    txn_version, value
                ))?,
            default_properties: value["default_properties"].clone(),
            inserted_at: chrono::Utc::now().naive_utc(),
        }))
    }

    fn get_token_data_id_from_table_item_key(
        key: &serde_json::Value,
        txn_version: i64,
    ) -> anyhow::Result<TokenDataId> {
        Ok(TokenDataId {
            creator_address: key["creator"]
                .as_str()
                .map(|s| s.to_string())
                .context(format!(
                    "version {} failed! creator missing from key {:?}",
                    txn_version, key
                ))?,
            collection_name: key["collection"]
                .as_str()
                .map(|s| s.to_string())
                .context(format!(
                    "version {} failed! collection missing from key {:?}",
                    txn_version, key
                ))?,
            name: key["name"]
                .as_str()
                .map(|s| s.to_string())
                .context(format!(
                    "version {} failed! name missing from key {:?}",
                    txn_version, key
                ))?,
        })
    }
}
