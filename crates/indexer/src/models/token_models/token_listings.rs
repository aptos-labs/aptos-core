// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]
#![allow(clippy::unused_unit)]

use super::{
    token_utils::TokenWriteSet,
    tokens::{TableHandleToOwner, TableMetadataForToken},
};
use crate::schema::current_token_listings;
use anyhow::Context;
use aptos_api_types::{DeleteTableItem as APIDeleteTableItem, WriteTableItem as APIWriteTableItem};
use bigdecimal::{BigDecimal, Zero};
use field_count::FieldCount;
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Queryable, Serialize)]
#[diesel(primary_key(token_data_id_hash, property_version, lister_address))]
#[diesel(table_name = current_token_listings)]
pub struct CurrentTokenListing {
    pub token_data_id_hash: String,
    pub property_version: BigDecimal,
    pub lister_address: String,
    pub collection_data_id_hash: String,
    pub creator_address: String,
    pub collection_name: String,
    pub name: String,
    pub coin_type: String,
    pub min_price_per_token: BigDecimal,
    pub amount: BigDecimal,
    pub table_handle: String,
    pub last_transaction_version: i64,
    pub inserted_at: chrono::NaiveDateTime,
}

impl CurrentTokenListing {
    /// Token listing is stored in a table in the lister's account. The key is token_id
    /// and value is TokenCoinSwap struct (min_price_per_token and token_amount)
    /// To get the coin type, we will need to rely on the resource type which matches
    /// "0x3::token_coin_swap::TokenListings<CoinType>",
    pub fn from_write_table_item(
        table_item: &APIWriteTableItem,
        txn_version: i64,
        table_handle_to_owner: &TableHandleToOwner,
    ) -> anyhow::Result<Option<Self>> {
        let table_item_data = table_item.data.as_ref().unwrap();

        let table_handle =
            TableMetadataForToken::standardize_handle(&table_item.handle.to_string());

        let maybe_table_metadata = table_handle_to_owner.get(&table_handle);

        if let Some(table_metadata) = maybe_table_metadata {
            let re = Regex::new(r"^0x3::token_coin_swap::TokenListings")?;
            if !re.is_match(table_metadata.table_type.as_str()) {
                return Ok(None);
            }

            // Will return everything inside of outer most <> brackets
            let re = Regex::new(r"<([^']*)>")?;
            let coin_type = re
                .captures(table_metadata.table_type.as_str())
                .map(|matched| matched[1].to_string())
                .context(format!(
                    "Failed to parse coin type from resource type {}",
                    table_metadata.table_type.as_str()
                ))?;

            let listing = match TokenWriteSet::from_table_item_type(
                "0x3::token_coin_swap::TokenCoinSwap",
                &table_item_data.value,
                txn_version,
            )? {
                Some(TokenWriteSet::TokenCoinSwap(inner)) => Some(inner),
                _ => None,
            }
            .context(format!(
                "Failed to resolve TokenCoinSwap, Version: {}, value_type: {}, value: {:?}",
                txn_version, table_item_data.value_type, table_item_data.value
            ))?;

            let token_id = match TokenWriteSet::from_table_item_type(
                table_item_data.key_type.as_str(),
                &table_item_data.key,
                txn_version,
            )? {
                Some(TokenWriteSet::TokenId(inner)) => Some(inner),
                _ => None,
            }
            .context(format!(
                "Failed to resolve TokenId, Version: {}, key_type: {}, key: {:?}",
                txn_version, table_item_data.key_type, table_item_data.key
            ))?;

            let token_data_id = token_id.token_data_id;
            let collection_data_id_hash = token_data_id.get_collection_data_id_hash();
            let token_data_id_hash = token_data_id.to_hash();
            let collection_name = token_data_id.get_collection_trunc();
            let name = token_data_id.get_name_trunc();

            Ok(Some(Self {
                token_data_id_hash,
                property_version: token_id.property_version,
                lister_address: table_metadata.owner_address.clone(),
                collection_data_id_hash,
                creator_address: token_data_id.creator,
                collection_name,
                name,
                coin_type,
                min_price_per_token: listing.min_price_per_token,
                amount: listing.token_amount,
                table_handle,
                last_transaction_version: txn_version,
                inserted_at: chrono::Utc::now().naive_utc(),
            }))
        } else {
            Ok(None)
        }
    }

    pub fn from_delete_table_item(
        table_item: &APIDeleteTableItem,
        txn_version: i64,
        table_handle_to_owner: &TableHandleToOwner,
    ) -> anyhow::Result<Option<Self>> {
        let table_item_data = table_item.data.as_ref().unwrap();

        let table_handle =
            TableMetadataForToken::standardize_handle(&table_item.handle.to_string());

        let maybe_table_metadata = table_handle_to_owner.get(&table_handle);

        if let Some(table_metadata) = maybe_table_metadata {
            let re = Regex::new(r"^0x3::token_coin_swap::TokenListings")?;
            if !re.is_match(table_metadata.table_type.as_str()) {
                return Ok(None);
            }

            // Will return everything inside of outer most <> brackets
            let re = Regex::new(r"<([^']*)>")?;
            let coin_type = re
                .captures(table_metadata.table_type.as_str())
                .map(|matched| matched[1].to_string())
                .context(format!(
                    "Failed to parse coin type from resource type {}",
                    table_metadata.table_type.as_str()
                ))?;

            let token_id = match TokenWriteSet::from_table_item_type(
                table_item_data.key_type.as_str(),
                &table_item_data.key,
                txn_version,
            )? {
                Some(TokenWriteSet::TokenId(inner)) => Some(inner),
                _ => None,
            }
            .context(format!(
                "Failed to resolve TokenId, Version: {}, key_type: {}, key: {:?}",
                txn_version, table_item_data.key_type, table_item_data.key
            ))?;

            let token_data_id = token_id.token_data_id;
            let collection_data_id_hash = token_data_id.get_collection_data_id_hash();
            let token_data_id_hash = token_data_id.to_hash();
            let collection_name = token_data_id.get_collection_trunc();
            let name = token_data_id.get_name_trunc();

            Ok(Some(Self {
                token_data_id_hash,
                property_version: token_id.property_version,
                lister_address: table_metadata.owner_address.clone(),
                collection_data_id_hash,
                creator_address: token_data_id.creator,
                collection_name,
                name,
                coin_type,
                min_price_per_token: BigDecimal::zero(),
                amount: BigDecimal::zero(),
                table_handle,
                last_transaction_version: txn_version,
                inserted_at: chrono::Utc::now().naive_utc(),
            }))
        } else {
            Ok(None)
        }
    }
}
