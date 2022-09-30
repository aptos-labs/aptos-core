// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]
#![allow(clippy::unused_unit)]

use super::tokens::{TableHandleToOwner, TableMetadataForToken, Token};
use crate::schema::{current_token_ownerships, token_ownerships};
use bigdecimal::BigDecimal;
use field_count::FieldCount;
use serde::{Deserialize, Serialize};

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
    pub inserted_at: chrono::NaiveDateTime,
    pub collection_data_id_hash: String,
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
    pub inserted_at: chrono::NaiveDateTime,
    pub collection_data_id_hash: String,
    pub table_type: String,
}

impl TokenOwnership {
    pub fn from_token(
        token: &Token,
        amount: BigDecimal,
        table_handle: String,
        table_handle_to_owner: &TableHandleToOwner,
        // Escrow tables somehow don't appear in resources so this is just a temporary workaround to record that it's an escrow table
        value_type: Option<&str>,
    ) -> (Self, Option<CurrentTokenOwnership>) {
        let table_handle = TableMetadataForToken::standardize_handle(&table_handle);
        let txn_version = token.transaction_version;
        let maybe_table_metadata = table_handle_to_owner.get(&table_handle);
        let (curr_token_ownership, owner_address, mut table_type) = match maybe_table_metadata {
            Some(tm) => (
                Some(CurrentTokenOwnership {
                    collection_data_id_hash: token.collection_data_id_hash.clone(),
                    token_data_id_hash: token.token_data_id_hash.clone(),
                    property_version: token.property_version.clone(),
                    owner_address: tm.owner_address.clone(),
                    creator_address: token.creator_address.clone(),
                    collection_name: token.collection_name.clone(),
                    name: token.name.clone(),
                    amount: amount.clone(),
                    token_properties: token.token_properties.clone(),
                    last_transaction_version: txn_version,
                    inserted_at: chrono::Utc::now().naive_utc(),
                    table_type: tm.table_type.clone(),
                }),
                Some(tm.owner_address.clone()),
                Some(tm.table_type.clone()),
            ),
            None => {
                aptos_logger::warn!(
                    transaction_version = txn_version,
                    table_handle = table_handle,
                    "Missing table handle metadata for TokenStore. {:?}",
                    table_handle_to_owner
                );
                (None, None, None)
            }
        };

        // Hacky handling of escrow tables that are generally present as a resource
        if let Some(val) = value_type {
            if val == "0x3::token_coin_swap::TokenEscrow" {
                table_type = Some(
                    table_type
                        .unwrap_or_else(|| "0x3::token_coin_swap::TokenStoreEscrow".to_string()),
                )
            }
        }

        (
            Self {
                collection_data_id_hash: token.collection_data_id_hash.clone(),
                token_data_id_hash: token.token_data_id_hash.clone(),
                property_version: token.property_version.clone(),
                owner_address,
                creator_address: token.creator_address.clone(),
                collection_name: token.collection_name.clone(),
                name: token.name.clone(),
                amount,
                table_type,
                transaction_version: token.transaction_version,
                table_handle,
                inserted_at: chrono::Utc::now().naive_utc(),
            },
            curr_token_ownership,
        )
    }
}
