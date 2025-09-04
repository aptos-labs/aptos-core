// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]
#![allow(clippy::unused_unit)]

use super::{
    collection_datas::{QUERY_RETRIES, QUERY_RETRY_DELAY_MS},
    token_utils::TokenWriteSet,
    v2_token_utils::{TokenStandard, TokenV2, TokenV2AggregatedDataMapping},
};
use crate::{
    database::PgPoolConnection,
    schema::{current_token_datas_v2, token_datas_v2},
    util::standardize_address,
};
use anyhow::Context;
use velor_api_types::{WriteResource as APIWriteResource, WriteTableItem as APIWriteTableItem};
use bigdecimal::{BigDecimal, Zero};
use diesel::{prelude::*, sql_query, sql_types::Text};
use field_count::FieldCount;
use serde::{Deserialize, Serialize};

// PK of current_token_datas_v2, i.e. token_data_id
pub type CurrentTokenDataV2PK = String;

#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(transaction_version, write_set_change_index))]
#[diesel(table_name = token_datas_v2)]
pub struct TokenDataV2 {
    pub transaction_version: i64,
    pub write_set_change_index: i64,
    pub token_data_id: String,
    pub collection_id: String,
    pub token_name: String,
    pub maximum: Option<BigDecimal>,
    pub supply: BigDecimal,
    pub largest_property_version_v1: Option<BigDecimal>,
    pub token_uri: String,
    pub token_properties: serde_json::Value,
    pub description: String,
    pub token_standard: String,
    pub is_fungible_v2: Option<bool>,
    pub transaction_timestamp: chrono::NaiveDateTime,
    pub decimals: i64,
}

#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(token_data_id))]
#[diesel(table_name = current_token_datas_v2)]
pub struct CurrentTokenDataV2 {
    pub token_data_id: String,
    pub collection_id: String,
    pub token_name: String,
    pub maximum: Option<BigDecimal>,
    pub supply: BigDecimal,
    pub largest_property_version_v1: Option<BigDecimal>,
    pub token_uri: String,
    pub token_properties: serde_json::Value,
    pub description: String,
    pub token_standard: String,
    pub is_fungible_v2: Option<bool>,
    pub last_transaction_version: i64,
    pub last_transaction_timestamp: chrono::NaiveDateTime,
    pub decimals: i64,
}

#[derive(Debug, QueryableByName)]
pub struct TokenDataIdFromTable {
    #[diesel(sql_type = Text)]
    pub token_data_id: String,
}

impl TokenDataV2 {
    pub fn get_v2_from_write_resource(
        write_resource: &APIWriteResource,
        txn_version: i64,
        write_set_change_index: i64,
        txn_timestamp: chrono::NaiveDateTime,
        token_v2_metadata: &TokenV2AggregatedDataMapping,
    ) -> anyhow::Result<Option<(Self, CurrentTokenDataV2)>> {
        if let Some(inner) = &TokenV2::from_write_resource(write_resource, txn_version)? {
            let token_data_id = standardize_address(&write_resource.address.to_string());
            // Get maximum, supply, and is fungible from fungible asset if this is a fungible token
            let (mut maximum, mut supply, mut decimals, mut is_fungible_v2) =
                (None, BigDecimal::zero(), 0, Some(false));
            // Get token properties from 0x4::property_map::PropertyMap
            let mut token_properties = serde_json::Value::Null;
            if let Some(metadata) = token_v2_metadata.get(&token_data_id) {
                let fungible_asset_metadata = metadata.fungible_asset_metadata.as_ref();
                let fungible_asset_supply = metadata.fungible_asset_supply.as_ref();
                if let Some(metadata) = fungible_asset_metadata {
                    if let Some(fa_supply) = fungible_asset_supply {
                        maximum = fa_supply.get_maximum();
                        supply = fa_supply.current.clone();
                        decimals = metadata.decimals as i64;
                        is_fungible_v2 = Some(true);
                    }
                }
                token_properties = metadata
                    .property_map
                    .as_ref()
                    .map(|m| m.inner.clone())
                    .unwrap_or(token_properties);
            } else {
                // ObjectCore should not be missing, returning from entire function early
                return Ok(None);
            }

            let collection_id = inner.get_collection_address();
            let token_name = inner.get_name_trunc();
            let token_uri = inner.get_uri_trunc();

            Ok(Some((
                Self {
                    transaction_version: txn_version,
                    write_set_change_index,
                    token_data_id: token_data_id.clone(),
                    collection_id: collection_id.clone(),
                    token_name: token_name.clone(),
                    maximum: maximum.clone(),
                    supply: supply.clone(),
                    largest_property_version_v1: None,
                    token_uri: token_uri.clone(),
                    token_properties: token_properties.clone(),
                    description: inner.description.clone(),
                    token_standard: TokenStandard::V2.to_string(),
                    is_fungible_v2,
                    transaction_timestamp: txn_timestamp,
                    decimals,
                },
                CurrentTokenDataV2 {
                    token_data_id,
                    collection_id,
                    token_name,
                    maximum,
                    supply,
                    largest_property_version_v1: None,
                    token_uri,
                    token_properties,
                    description: inner.description.clone(),
                    token_standard: TokenStandard::V2.to_string(),
                    is_fungible_v2,
                    last_transaction_version: txn_version,
                    last_transaction_timestamp: txn_timestamp,
                    decimals,
                },
            )))
        } else {
            Ok(None)
        }
    }

    pub fn get_v1_from_write_table_item(
        table_item: &APIWriteTableItem,
        txn_version: i64,
        write_set_change_index: i64,
        txn_timestamp: chrono::NaiveDateTime,
    ) -> anyhow::Result<Option<(Self, CurrentTokenDataV2)>> {
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
            if let Some(token_data_id_struct) = maybe_token_data_id {
                let collection_id = token_data_id_struct.get_collection_id();
                let token_data_id = token_data_id_struct.to_id();
                let token_name = token_data_id_struct.get_name_trunc();
                let token_uri = token_data.get_uri_trunc();

                return Ok(Some((
                    Self {
                        transaction_version: txn_version,
                        write_set_change_index,
                        token_data_id: token_data_id.clone(),
                        collection_id: collection_id.clone(),
                        token_name: token_name.clone(),
                        maximum: Some(token_data.maximum.clone()),
                        supply: token_data.supply.clone(),
                        largest_property_version_v1: Some(
                            token_data.largest_property_version.clone(),
                        ),
                        token_uri: token_uri.clone(),
                        token_properties: token_data.default_properties.clone(),
                        description: token_data.description.clone(),
                        token_standard: TokenStandard::V1.to_string(),
                        is_fungible_v2: None,
                        transaction_timestamp: txn_timestamp,
                        decimals: 0,
                    },
                    CurrentTokenDataV2 {
                        token_data_id,
                        collection_id,
                        token_name,
                        maximum: Some(token_data.maximum),
                        supply: token_data.supply,
                        largest_property_version_v1: Some(token_data.largest_property_version),
                        token_uri,
                        token_properties: token_data.default_properties,
                        description: token_data.description,
                        token_standard: TokenStandard::V1.to_string(),
                        is_fungible_v2: None,
                        last_transaction_version: txn_version,
                        last_transaction_timestamp: txn_timestamp,
                        decimals: 0,
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

    /// Try to see if an address is a token. We'll try a few times in case there is a race condition,
    /// and if we can't find after 3 times, we'll assume that it's not a token.
    /// TODO: An improvement is that we'll make another query to see if address is a coin.
    pub fn is_address_token(conn: &mut PgPoolConnection, address: &str) -> bool {
        let mut retried = 0;
        while retried < QUERY_RETRIES {
            retried += 1;
            match Self::get_by_token_data_id(conn, address) {
                Ok(_) => return true,
                Err(_) => {
                    std::thread::sleep(std::time::Duration::from_millis(QUERY_RETRY_DELAY_MS));
                },
            }
        }
        false
    }

    /// TODO: Change this to a KV store
    fn get_by_token_data_id(conn: &mut PgPoolConnection, address: &str) -> anyhow::Result<String> {
        let mut res: Vec<Option<TokenDataIdFromTable>> =
            sql_query("SELECT token_data_id FROM current_token_datas_v2 WHERE token_data_id = $1")
                .bind::<Text, _>(address)
                .get_results(conn)?;
        Ok(res
            .pop()
            .context("token data result empty")?
            .context("token data result null")?
            .token_data_id)
    }
}
