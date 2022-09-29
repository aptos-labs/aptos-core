// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]
#![allow(clippy::unused_unit)]

use super::{
    token_utils::TokenWriteSet,
    tokens::{TableHandleToOwner, TableMetadataForToken},
};
use crate::{
    schema::{current_token_escrows, token_escrows},
    util::{bigdecimal_to_u64, parse_timestamp_secs},
};
use aptos_api_types::WriteTableItem as APIWriteTableItem;
use bigdecimal::BigDecimal;
use field_count::FieldCount;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Queryable, Serialize)]
#[diesel(primary_key(
    table_handle,
    token_data_id_hash,
    property_version,
    transaction_version
))]
#[diesel(table_name = token_escrows)]
pub struct TokenEscrow {
    pub table_handle: String,
    pub token_data_id_hash: String,
    pub property_version: BigDecimal,
    pub transaction_version: i64,
    pub collection_data_id_hash: String,
    pub creator_address: String,
    pub collection_name: String,
    pub name: String,
    pub owner_address: Option<String>,
    pub amount: BigDecimal,
    pub locked_until_secs: chrono::NaiveDateTime,
    pub inserted_at: chrono::NaiveDateTime,
}

#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Queryable, Serialize)]
#[diesel(primary_key(owner_address, token_data_id_hash, property_version))]
#[diesel(table_name = current_token_escrows)]
pub struct CurrentTokenEscrow {
    pub owner_address: String,
    pub token_data_id_hash: String,
    pub property_version: BigDecimal,
    pub collection_data_id_hash: String,
    pub creator_address: String,
    pub collection_name: String,
    pub name: String,
    pub amount: BigDecimal,
    pub locked_until_secs: chrono::NaiveDateTime,
    pub table_handle: String,
    pub last_transaction_version: i64,
    pub inserted_at: chrono::NaiveDateTime,
}

impl TokenEscrow {
    /// Token escrow is a table where the key is token_offer_id (token_id + to address)
    /// and value is token escrow (token + time_until_secs)
    pub fn from_write_table_item(
        table_item: &APIWriteTableItem,
        txn_version: i64,
        table_handle_to_owner: &TableHandleToOwner,
    ) -> anyhow::Result<Option<(Self, Option<CurrentTokenEscrow>)>> {
        let table_item_data = table_item.data.as_ref().unwrap();

        let maybe_escrow = match TokenWriteSet::from_table_item_type(
            table_item_data.value_type.as_str(),
            &table_item_data.value,
            txn_version,
        )? {
            Some(TokenWriteSet::TokenEscrow(inner)) => Some(inner),
            _ => None,
        };
        if let Some(escrow) = maybe_escrow {
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

                let maybe_table_metadata = table_handle_to_owner.get(&table_handle);
                let token_data_id = token_id.token_data_id;
                let collection_data_id_hash = token_data_id.get_collection_data_id_hash();
                let token_data_id_hash = token_data_id.to_hash();
                let collection_name = token_data_id.get_collection_trunc();
                let name = token_data_id.get_name_trunc();
                let locked_until_secs =
                    parse_timestamp_secs(bigdecimal_to_u64(&escrow.locked_until_secs), txn_version);

                let (curr_token_escrow, owner_address) = match maybe_table_metadata {
                    Some(tm) => (
                        Some(CurrentTokenEscrow {
                            owner_address: tm.owner_address.clone(),
                            token_data_id_hash: token_data_id_hash.clone(),
                            property_version: token_id.property_version.clone(),
                            collection_data_id_hash: collection_data_id_hash.clone(),
                            creator_address: token_data_id.creator.clone(),
                            collection_name: collection_name.clone(),
                            name: name.clone(),
                            amount: escrow.token.amount.clone(),
                            locked_until_secs,
                            table_handle: table_handle.clone(),
                            last_transaction_version: txn_version,
                            inserted_at: chrono::Utc::now().naive_utc(),
                        }),
                        Some(tm.owner_address.clone()),
                    ),
                    None => {
                        aptos_logger::warn!(
                            transaction_version = txn_version,
                            table_handle = table_handle,
                            "Missing table handle metadata for TokenEscrow. {:?}",
                            table_handle_to_owner
                        );
                        (None, None)
                    }
                };

                return Ok(Some((
                    Self {
                        owner_address,
                        token_data_id_hash,
                        property_version: token_id.property_version,
                        transaction_version: txn_version,
                        collection_data_id_hash,
                        creator_address: token_data_id.creator,
                        collection_name,
                        name,
                        amount: escrow.token.amount,
                        locked_until_secs,
                        table_handle,
                        inserted_at: chrono::Utc::now().naive_utc(),
                    },
                    curr_token_escrow,
                )));
            } else {
                aptos_logger::warn!(
                    transaction_version = txn_version,
                    value_type = table_item_data.value_type,
                    value = table_item_data.value,
                    "Expecting token_id as key for value = token_escrow"
                );
            }
        }
        Ok(None)
    }
}
