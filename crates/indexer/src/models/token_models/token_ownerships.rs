// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]
#![allow(clippy::unused_unit)]

use super::{
    token_utils::TokenWriteSet,
    tokens::{TableHandleToOwner, Token},
};
use crate::{
    schema::{current_token_ownerships, token_ownerships},
    util::standardize_address,
};
use bigdecimal::BigDecimal;
use field_count::FieldCount;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
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
    pub collection_data_id_hash: String,
    pub transaction_timestamp: chrono::NaiveDateTime,
}

#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
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
    pub collection_data_id_hash: String,
    pub table_type: String,
    pub last_transaction_timestamp: chrono::NaiveDateTime,
}

impl TokenOwnership {
    /// We only want to track tokens in 0x1::token::TokenStore for now. This is because the table
    /// schema doesn't have table type (i.e. token container) as primary key. TokenStore has token_id
    /// as key and token as value.
    pub fn from_token(
        token: &Token,
        table_item_key_type: &str,
        table_item_key: &Value,
        amount: BigDecimal,
        table_handle: String,
        table_handle_to_owner: &TableHandleToOwner,
    ) -> anyhow::Result<Option<(Self, Option<CurrentTokenOwnership>)>> {
        let txn_version = token.transaction_version;
        let maybe_token_id = match TokenWriteSet::from_table_item_type(
            table_item_key_type,
            table_item_key,
            txn_version,
        )? {
            Some(TokenWriteSet::TokenId(inner)) => Some(inner),
            _ => None,
        };
        // Return early if table key is not token id
        if maybe_token_id.is_none() {
            return Ok(None);
        }
        let table_handle = standardize_address(&table_handle);
        let maybe_table_metadata = table_handle_to_owner.get(&table_handle);
        // Return early if table type is not tokenstore
        if let Some(tm) = maybe_table_metadata {
            if tm.table_type != "0x3::token::TokenStore" {
                return Ok(None);
            }
        }
        let (curr_token_ownership, owner_address, table_type) = match maybe_table_metadata {
            Some(tm) => (
                Some(CurrentTokenOwnership {
                    collection_data_id_hash: token.collection_data_id_hash.clone(),
                    token_data_id_hash: token.token_data_id_hash.clone(),
                    property_version: token.property_version.clone(),
                    owner_address: standardize_address(&tm.owner_address),
                    creator_address: standardize_address(&token.creator_address.clone()),
                    collection_name: token.collection_name.clone(),
                    name: token.name.clone(),
                    amount: amount.clone(),
                    token_properties: token.token_properties.clone(),
                    last_transaction_version: txn_version,
                    table_type: tm.table_type.clone(),
                    last_transaction_timestamp: token.transaction_timestamp,
                }),
                Some(standardize_address(&tm.owner_address)),
                Some(tm.table_type.clone()),
            ),
            None => {
                velor_logger::warn!(
                    transaction_version = txn_version,
                    table_handle = table_handle,
                    "Missing table handle metadata for TokenStore. {:?}",
                    table_handle_to_owner
                );
                (None, None, None)
            },
        };

        Ok(Some((
            Self {
                collection_data_id_hash: token.collection_data_id_hash.clone(),
                token_data_id_hash: token.token_data_id_hash.clone(),
                property_version: token.property_version.clone(),
                owner_address: owner_address.map(|s| standardize_address(&s)),
                creator_address: standardize_address(&token.creator_address),
                collection_name: token.collection_name.clone(),
                name: token.name.clone(),
                amount,
                table_type,
                transaction_version: token.transaction_version,
                table_handle,
                transaction_timestamp: token.transaction_timestamp,
            },
            curr_token_ownership,
        )))
    }
}
