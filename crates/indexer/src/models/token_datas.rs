// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]
#![allow(clippy::unused_unit)]

use super::token_utils::TokenWriteSet;
use crate::{
    schema::{current_token_datas, token_datas},
    util::{hash_str, truncate_str},
};
use anyhow::Context;
use aptos_api_types::WriteTableItem as APIWriteTableItem;
use bigdecimal::BigDecimal;
use field_count::FieldCount;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Queryable, Serialize)]
#[diesel(primary_key(token_data_id_hash, transaction_version))]
#[diesel(table_name = token_datas)]
pub struct TokenData {
    pub token_data_id_hash: String,
    pub transaction_version: i64,
    pub creator_address: String,
    pub collection_name: String,
    pub name: String,
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

#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Queryable, Serialize)]
#[diesel(primary_key(token_data_id_hash))]
#[diesel(table_name = current_token_datas)]
pub struct CurrentTokenData {
    pub token_data_id_hash: String,
    pub creator_address: String,
    pub collection_name: String,
    pub name: String,
    pub maximum: bigdecimal::BigDecimal,
    pub supply: bigdecimal::BigDecimal,
    pub largest_property_version: bigdecimal::BigDecimal,
    pub metadata_uri: String,
    pub payee_address: String,
    pub royalty_points_numerator: bigdecimal::BigDecimal,
    pub royalty_points_denominator: bigdecimal::BigDecimal,
    pub maximum_mutable: bool,
    pub uri_mutable: bool,
    pub description_mutable: bool,
    pub properties_mutable: bool,
    pub royalty_mutable: bool,
    pub default_properties: serde_json::Value,
    pub last_transaction_version: i64,
    // Default time columns
    pub inserted_at: chrono::NaiveDateTime,
}

impl TokenData {
    pub fn from_write_table_item(
        table_item: &APIWriteTableItem,
        txn_version: i64,
    ) -> anyhow::Result<Option<(Self, CurrentTokenData)>> {
        let table_item_data = table_item.data.as_ref().unwrap();

        let maybe_token_data = match TokenWriteSet::from_table_item_type(
            table_item_data.value_type.as_str(),
            &table_item_data.value,
            txn_version,
        )? {
            Some(TokenWriteSet::TokenData(inner)) => Some(inner),
            _ => None,
        };

        if let Some(token_data) = maybe_token_data {
            let token_data_id = match TokenWriteSet::from_table_item_type(
                table_item_data.key_type.as_str(),
                &table_item_data.key,
                txn_version,
            )? {
                Some(TokenWriteSet::TokenDataId(inner)) => Some(inner),
                _ => None,
            }
            .context(format!(
                "Could not get token data id from table item key_type: {}, key: {:?} version: {}",
                table_item_data.key_type, table_item_data.key, txn_version
            ))?;
            let token_data_id_hash = hash_str(&token_data_id.to_string());
            let collection_name = truncate_str(&token_data_id.collection, 128);
            let name = truncate_str(&token_data_id.name, 128);
            let metadata_uri = truncate_str(&token_data.uri, 512);

            Ok(Some((
                Self {
                    token_data_id_hash: token_data_id_hash.clone(),
                    creator_address: token_data_id.creator.clone(),
                    collection_name: collection_name.clone(),
                    name: name.clone(),
                    transaction_version: txn_version,
                    maximum: token_data.maximum.clone(),
                    supply: token_data.supply.clone(),
                    largest_property_version: token_data.largest_property_version.clone(),
                    metadata_uri: metadata_uri.clone(),
                    payee_address: token_data.royalty.payee_address.clone(),
                    royalty_points_numerator: token_data.royalty.royalty_points_numerator.clone(),
                    royalty_points_denominator: token_data
                        .royalty
                        .royalty_points_denominator
                        .clone(),
                    maximum_mutable: token_data.mutability_config.maximum,
                    uri_mutable: token_data.mutability_config.uri,
                    description_mutable: token_data.mutability_config.description,
                    properties_mutable: token_data.mutability_config.properties,
                    royalty_mutable: token_data.mutability_config.royalty,
                    default_properties: token_data.default_properties.clone(),
                    inserted_at: chrono::Utc::now().naive_utc(),
                },
                CurrentTokenData {
                    token_data_id_hash,
                    creator_address: token_data_id.creator,
                    collection_name,
                    name,
                    maximum: token_data.maximum,
                    supply: token_data.supply,
                    largest_property_version: token_data.largest_property_version,
                    metadata_uri,
                    payee_address: token_data.royalty.payee_address,
                    royalty_points_numerator: token_data.royalty.royalty_points_numerator,
                    royalty_points_denominator: token_data.royalty.royalty_points_denominator,
                    maximum_mutable: token_data.mutability_config.maximum,
                    uri_mutable: token_data.mutability_config.uri,
                    description_mutable: token_data.mutability_config.description,
                    properties_mutable: token_data.mutability_config.properties,
                    royalty_mutable: token_data.mutability_config.royalty,
                    default_properties: token_data.default_properties,
                    last_transaction_version: txn_version,
                    inserted_at: chrono::Utc::now().naive_utc(),
                },
            )))
        } else {
            Ok(None)
        }
    }
}
