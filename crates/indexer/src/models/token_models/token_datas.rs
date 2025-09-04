// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]
#![allow(clippy::unused_unit)]

use super::token_utils::TokenWriteSet;
use crate::{
    schema::{current_token_datas, token_datas},
    util::standardize_address,
};
use velor_api_types::WriteTableItem as APIWriteTableItem;
use bigdecimal::BigDecimal;
use field_count::FieldCount;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
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
    pub collection_data_id_hash: String,
    pub transaction_timestamp: chrono::NaiveDateTime,
    pub description: String,
}

#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
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
    pub collection_data_id_hash: String,
    pub last_transaction_timestamp: chrono::NaiveDateTime,
    pub description: String,
}

impl TokenData {
    pub fn from_write_table_item(
        table_item: &APIWriteTableItem,
        txn_version: i64,
        txn_timestamp: chrono::NaiveDateTime,
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
            let maybe_token_data_id = match TokenWriteSet::from_table_item_type(
                table_item_data.key_type.as_str(),
                &table_item_data.key,
                txn_version,
            )? {
                Some(TokenWriteSet::TokenDataId(inner)) => Some(inner),
                _ => None,
            };
            if let Some(token_data_id) = maybe_token_data_id {
                let collection_data_id_hash = token_data_id.get_collection_data_id_hash();
                let token_data_id_hash = token_data_id.to_hash();
                let collection_name = token_data_id.get_collection_trunc();
                let name = token_data_id.get_name_trunc();
                let metadata_uri = token_data.get_uri_trunc();

                return Ok(Some((
                    Self {
                        collection_data_id_hash: collection_data_id_hash.clone(),
                        token_data_id_hash: token_data_id_hash.clone(),
                        creator_address: standardize_address(&token_data_id.creator),
                        collection_name: collection_name.clone(),
                        name: name.clone(),
                        transaction_version: txn_version,
                        maximum: token_data.maximum.clone(),
                        supply: token_data.supply.clone(),
                        largest_property_version: token_data.largest_property_version.clone(),
                        metadata_uri: metadata_uri.clone(),
                        payee_address: standardize_address(&token_data.royalty.payee_address),
                        royalty_points_numerator: token_data
                            .royalty
                            .royalty_points_numerator
                            .clone(),
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
                        transaction_timestamp: txn_timestamp,
                        description: token_data.description.clone(),
                    },
                    CurrentTokenData {
                        collection_data_id_hash,
                        token_data_id_hash,
                        creator_address: standardize_address(&token_data_id.creator),
                        collection_name,
                        name,
                        maximum: token_data.maximum,
                        supply: token_data.supply,
                        largest_property_version: token_data.largest_property_version,
                        metadata_uri,
                        payee_address: standardize_address(&token_data.royalty.payee_address),
                        royalty_points_numerator: token_data.royalty.royalty_points_numerator,
                        royalty_points_denominator: token_data.royalty.royalty_points_denominator,
                        maximum_mutable: token_data.mutability_config.maximum,
                        uri_mutable: token_data.mutability_config.uri,
                        description_mutable: token_data.mutability_config.description,
                        properties_mutable: token_data.mutability_config.properties,
                        royalty_mutable: token_data.mutability_config.royalty,
                        default_properties: token_data.default_properties,
                        last_transaction_version: txn_version,
                        last_transaction_timestamp: txn_timestamp,
                        description: token_data.description,
                    },
                )));
            } else {
                velor_logger::warn!(
                    transaction_version = txn_version,
                    key_type = table_item_data.key_type,
                    key = table_item_data.key,
                    "Expecting token_data_id as key for value = token_data"
                );
            }
        }
        Ok(None)
    }
}
