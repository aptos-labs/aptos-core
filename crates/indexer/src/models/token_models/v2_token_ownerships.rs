// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]
#![allow(clippy::unused_unit)]

use super::{
    collection_datas::{QUERY_RETRIES, QUERY_RETRY_DELAY_MS},
    token_utils::TokenWriteSet,
    tokens::TableHandleToOwner,
    v2_token_datas::TokenDataV2,
    v2_token_utils::{
        ObjectWithMetadata, TokenStandard, TokenV2AggregatedDataMapping, TokenV2Burned,
    },
};
use crate::{
    database::PgPoolConnection,
    models::{
        coin_models::v2_fungible_asset_utils::V2FungibleAssetResource, move_resources::MoveResource,
    },
    schema::{current_token_ownerships_v2, token_ownerships_v2},
    util::{ensure_not_negative, standardize_address},
};
use anyhow::Context;
use velor_api_types::{
    DeleteResource, DeleteTableItem as APIDeleteTableItem, WriteResource,
    WriteTableItem as APIWriteTableItem,
};
use bigdecimal::{BigDecimal, One, Zero};
use diesel::{prelude::*, ExpressionMethods};
use field_count::FieldCount;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// PK of current_token_ownerships_v2, i.e. token_data_id, property_version_v1, owner_address, storage_id
pub type CurrentTokenOwnershipV2PK = (String, BigDecimal, String, String);

#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(transaction_version, write_set_change_index))]
#[diesel(table_name = token_ownerships_v2)]
pub struct TokenOwnershipV2 {
    pub transaction_version: i64,
    pub write_set_change_index: i64,
    pub token_data_id: String,
    pub property_version_v1: BigDecimal,
    pub owner_address: Option<String>,
    pub storage_id: String,
    pub amount: BigDecimal,
    pub table_type_v1: Option<String>,
    pub token_properties_mutated_v1: Option<serde_json::Value>,
    pub is_soulbound_v2: Option<bool>,
    pub token_standard: String,
    pub is_fungible_v2: Option<bool>,
    pub transaction_timestamp: chrono::NaiveDateTime,
    pub non_transferrable_by_owner: Option<bool>,
}

#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(token_data_id, property_version_v1, owner_address, storage_id))]
#[diesel(table_name = current_token_ownerships_v2)]
pub struct CurrentTokenOwnershipV2 {
    pub token_data_id: String,
    pub property_version_v1: BigDecimal,
    pub owner_address: String,
    pub storage_id: String,
    pub amount: BigDecimal,
    pub table_type_v1: Option<String>,
    pub token_properties_mutated_v1: Option<serde_json::Value>,
    pub is_soulbound_v2: Option<bool>,
    pub token_standard: String,
    pub is_fungible_v2: Option<bool>,
    pub last_transaction_version: i64,
    pub last_transaction_timestamp: chrono::NaiveDateTime,
    pub non_transferrable_by_owner: Option<bool>,
}

// Facilitate tracking when a token is burned
#[derive(Clone, Debug)]
pub struct NFTOwnershipV2 {
    pub token_data_id: String,
    pub owner_address: String,
    pub is_soulbound: Option<bool>,
}

/// Need a separate struct for queryable because we don't want to define the inserted_at column (letting DB fill)
#[derive(Debug, Identifiable, Queryable)]
#[diesel(primary_key(token_data_id, property_version_v1, owner_address, storage_id))]
#[diesel(table_name = current_token_ownerships_v2)]
pub struct CurrentTokenOwnershipV2Query {
    pub token_data_id: String,
    pub property_version_v1: BigDecimal,
    pub owner_address: String,
    pub storage_id: String,
    pub amount: BigDecimal,
    pub table_type_v1: Option<String>,
    pub token_properties_mutated_v1: Option<serde_json::Value>,
    pub is_soulbound_v2: Option<bool>,
    pub token_standard: String,
    pub is_fungible_v2: Option<bool>,
    pub last_transaction_version: i64,
    pub last_transaction_timestamp: chrono::NaiveDateTime,
    pub inserted_at: chrono::NaiveDateTime,
    pub non_transferrable_by_owner: Option<bool>,
}

impl TokenOwnershipV2 {
    /// For nfts it's the same resources that we parse tokendatas from so we leverage the work done in there to get ownership data
    pub fn get_nft_v2_from_token_data(
        token_data: &TokenDataV2,
        token_v2_metadata: &TokenV2AggregatedDataMapping,
    ) -> anyhow::Result<
        Option<(
            Self,
            CurrentTokenOwnershipV2,
            Option<Self>, // If token was transferred, the previous ownership record
            Option<CurrentTokenOwnershipV2>, // If token was transferred, the previous ownership record
        )>,
    > {
        // We should be indexing v1 token or v2 fungible token here
        if token_data.is_fungible_v2 != Some(false) {
            return Ok(None);
        }
        let metadata = token_v2_metadata
            .get(&token_data.token_data_id)
            .context("If token data exists objectcore must exist")?;
        let object_core = metadata.object.object_core.clone();
        let token_data_id = token_data.token_data_id.clone();
        let owner_address = object_core.get_owner_address();
        let storage_id = token_data_id.clone();
        let is_soulbound = !object_core.allow_ungated_transfer;

        let ownership = Self {
            transaction_version: token_data.transaction_version,
            write_set_change_index: token_data.write_set_change_index,
            token_data_id: token_data_id.clone(),
            property_version_v1: BigDecimal::zero(),
            owner_address: Some(owner_address.clone()),
            storage_id: storage_id.clone(),
            amount: BigDecimal::one(),
            table_type_v1: None,
            token_properties_mutated_v1: None,
            is_soulbound_v2: Some(is_soulbound),
            token_standard: TokenStandard::V2.to_string(),
            is_fungible_v2: token_data.is_fungible_v2,
            transaction_timestamp: token_data.transaction_timestamp,
            non_transferrable_by_owner: Some(is_soulbound),
        };
        let current_ownership = CurrentTokenOwnershipV2 {
            token_data_id: token_data_id.clone(),
            property_version_v1: BigDecimal::zero(),
            owner_address,
            storage_id: storage_id.clone(),
            amount: BigDecimal::one(),
            table_type_v1: None,
            token_properties_mutated_v1: None,
            is_soulbound_v2: Some(is_soulbound),
            token_standard: TokenStandard::V2.to_string(),
            is_fungible_v2: token_data.is_fungible_v2,
            last_transaction_version: token_data.transaction_version,
            last_transaction_timestamp: token_data.transaction_timestamp,
            non_transferrable_by_owner: Some(is_soulbound),
        };

        // check if token was transferred
        if let Some((event_index, transfer_event)) = &metadata.transfer_event {
            // If it's a self transfer then skip
            if transfer_event.get_to_address() == transfer_event.get_from_address() {
                return Ok(Some((ownership, current_ownership, None, None)));
            }
            Ok(Some((
                ownership,
                current_ownership,
                Some(Self {
                    transaction_version: token_data.transaction_version,
                    // set to negative of event index to avoid collison with write set index
                    write_set_change_index: -1 * event_index,
                    token_data_id: token_data_id.clone(),
                    property_version_v1: BigDecimal::zero(),
                    // previous owner
                    owner_address: Some(transfer_event.get_from_address()),
                    storage_id: storage_id.clone(),
                    // soft delete
                    amount: BigDecimal::zero(),
                    table_type_v1: None,
                    token_properties_mutated_v1: None,
                    is_soulbound_v2: Some(is_soulbound),
                    token_standard: TokenStandard::V2.to_string(),
                    is_fungible_v2: token_data.is_fungible_v2,
                    transaction_timestamp: token_data.transaction_timestamp,
                    non_transferrable_by_owner: Some(is_soulbound),
                }),
                Some(CurrentTokenOwnershipV2 {
                    token_data_id,
                    property_version_v1: BigDecimal::zero(),
                    // previous owner
                    owner_address: transfer_event.get_from_address(),
                    storage_id,
                    // soft delete
                    amount: BigDecimal::zero(),
                    table_type_v1: None,
                    token_properties_mutated_v1: None,
                    is_soulbound_v2: Some(is_soulbound),
                    token_standard: TokenStandard::V2.to_string(),
                    is_fungible_v2: token_data.is_fungible_v2,
                    last_transaction_version: token_data.transaction_version,
                    last_transaction_timestamp: token_data.transaction_timestamp,
                    non_transferrable_by_owner: Some(is_soulbound),
                }),
            )))
        } else {
            Ok(Some((ownership, current_ownership, None, None)))
        }
    }

    /// This handles the case where token is burned but objectCore is still there
    pub fn get_burned_nft_v2_from_write_resource(
        write_resource: &WriteResource,
        txn_version: i64,
        write_set_change_index: i64,
        txn_timestamp: chrono::NaiveDateTime,
        tokens_burned: &TokenV2Burned,
    ) -> anyhow::Result<Option<(Self, CurrentTokenOwnershipV2)>> {
        if let Some(token_address) =
            tokens_burned.get(&standardize_address(&write_resource.address.to_string()))
        {
            if let Some(object) =
                &ObjectWithMetadata::from_write_resource(write_resource, txn_version)?
            {
                let object_core = &object.object_core;
                let token_data_id = token_address.clone();
                let owner_address = object_core.get_owner_address();
                let storage_id = token_data_id.clone();
                let is_soulbound = !object_core.allow_ungated_transfer;

                return Ok(Some((
                    Self {
                        transaction_version: txn_version,
                        write_set_change_index,
                        token_data_id: token_data_id.clone(),
                        property_version_v1: BigDecimal::zero(),
                        owner_address: Some(owner_address.clone()),
                        storage_id: storage_id.clone(),
                        amount: BigDecimal::zero(),
                        table_type_v1: None,
                        token_properties_mutated_v1: None,
                        is_soulbound_v2: Some(is_soulbound),
                        token_standard: TokenStandard::V2.to_string(),
                        is_fungible_v2: Some(false),
                        transaction_timestamp: txn_timestamp,
                        non_transferrable_by_owner: Some(is_soulbound),
                    },
                    CurrentTokenOwnershipV2 {
                        token_data_id,
                        property_version_v1: BigDecimal::zero(),
                        owner_address,
                        storage_id,
                        amount: BigDecimal::zero(),
                        table_type_v1: None,
                        token_properties_mutated_v1: None,
                        is_soulbound_v2: Some(is_soulbound),
                        token_standard: TokenStandard::V2.to_string(),
                        is_fungible_v2: Some(false),
                        last_transaction_version: txn_version,
                        last_transaction_timestamp: txn_timestamp,
                        non_transferrable_by_owner: Some(is_soulbound),
                    },
                )));
            }
        }
        Ok(None)
    }

    /// This handles the case where token is burned and objectCore is deleted
    pub fn get_burned_nft_v2_from_delete_resource(
        write_resource: &DeleteResource,
        txn_version: i64,
        write_set_change_index: i64,
        txn_timestamp: chrono::NaiveDateTime,
        prior_nft_ownership: &HashMap<String, NFTOwnershipV2>,
        tokens_burned: &TokenV2Burned,
        conn: &mut PgPoolConnection,
    ) -> anyhow::Result<Option<(Self, CurrentTokenOwnershipV2)>> {
        if let Some(token_address) =
            tokens_burned.get(&standardize_address(&write_resource.address.to_string()))
        {
            let latest_nft_ownership: NFTOwnershipV2 = match prior_nft_ownership.get(token_address)
            {
                Some(inner) => inner.clone(),
                None => {
                    match CurrentTokenOwnershipV2Query::get_nft_by_token_data_id(
                        conn,
                        token_address,
                    ) {
                        Ok(nft) => nft,
                        Err(_) => {
                            velor_logger::error!(
                                transaction_version = txn_version,
                                lookup_key = &token_address,
                                "Failed to find NFT for burned token. You probably should backfill db."
                            );
                            return Ok(None);
                        },
                    }
                },
            };

            let token_data_id = token_address.clone();
            let owner_address = latest_nft_ownership.owner_address.clone();
            let storage_id = token_data_id.clone();
            let is_soulbound = latest_nft_ownership.is_soulbound;

            return Ok(Some((
                Self {
                    transaction_version: txn_version,
                    write_set_change_index,
                    token_data_id: token_data_id.clone(),
                    property_version_v1: BigDecimal::zero(),
                    owner_address: Some(owner_address.clone()),
                    storage_id: storage_id.clone(),
                    amount: BigDecimal::zero(),
                    table_type_v1: None,
                    token_properties_mutated_v1: None,
                    is_soulbound_v2: is_soulbound,
                    token_standard: TokenStandard::V2.to_string(),
                    is_fungible_v2: Some(false),
                    transaction_timestamp: txn_timestamp,
                    non_transferrable_by_owner: is_soulbound,
                },
                CurrentTokenOwnershipV2 {
                    token_data_id,
                    property_version_v1: BigDecimal::zero(),
                    owner_address,
                    storage_id,
                    amount: BigDecimal::zero(),
                    table_type_v1: None,
                    token_properties_mutated_v1: None,
                    is_soulbound_v2: is_soulbound,
                    token_standard: TokenStandard::V2.to_string(),
                    is_fungible_v2: Some(false),
                    last_transaction_version: txn_version,
                    last_transaction_timestamp: txn_timestamp,
                    non_transferrable_by_owner: is_soulbound,
                },
            )));
        }
        Ok(None)
    }

    // Getting this from 0x1::fungible_asset::FungibleStore
    pub fn get_ft_v2_from_write_resource(
        write_resource: &WriteResource,
        txn_version: i64,
        write_set_change_index: i64,
        txn_timestamp: chrono::NaiveDateTime,
        token_v2_metadata: &TokenV2AggregatedDataMapping,
    ) -> anyhow::Result<Option<(Self, CurrentTokenOwnershipV2)>> {
        let type_str = format!(
            "{}::{}::{}",
            write_resource.data.typ.address,
            write_resource.data.typ.module,
            write_resource.data.typ.name
        );
        if !V2FungibleAssetResource::is_resource_supported(type_str.as_str()) {
            return Ok(None);
        }
        let resource = MoveResource::from_write_resource(
            write_resource,
            0, // Placeholder, this isn't used anyway
            txn_version,
            0, // Placeholder, this isn't used anyway
        );

        if let V2FungibleAssetResource::FungibleAssetStore(inner) =
            V2FungibleAssetResource::from_resource(
                &type_str,
                resource.data.as_ref().unwrap(),
                txn_version,
            )?
        {
            if let Some(metadata) = token_v2_metadata.get(&resource.address) {
                let object_core = &metadata.object.object_core;
                let token_data_id = inner.metadata.get_reference_address();
                let storage_id = token_data_id.clone();
                let is_soulbound = inner.frozen;
                let amount = inner.balance;
                let owner_address = object_core.get_owner_address();

                return Ok(Some((
                    Self {
                        transaction_version: txn_version,
                        write_set_change_index,
                        token_data_id: token_data_id.clone(),
                        property_version_v1: BigDecimal::zero(),
                        owner_address: Some(owner_address.clone()),
                        storage_id: storage_id.clone(),
                        amount: amount.clone(),
                        table_type_v1: None,
                        token_properties_mutated_v1: None,
                        is_soulbound_v2: Some(is_soulbound),
                        token_standard: TokenStandard::V2.to_string(),
                        is_fungible_v2: Some(true),
                        transaction_timestamp: txn_timestamp,
                        non_transferrable_by_owner: Some(is_soulbound),
                    },
                    CurrentTokenOwnershipV2 {
                        token_data_id,
                        property_version_v1: BigDecimal::zero(),
                        owner_address,
                        storage_id,
                        amount,
                        table_type_v1: None,
                        token_properties_mutated_v1: None,
                        is_soulbound_v2: Some(is_soulbound),
                        token_standard: TokenStandard::V2.to_string(),
                        is_fungible_v2: Some(true),
                        last_transaction_version: txn_version,
                        last_transaction_timestamp: txn_timestamp,
                        non_transferrable_by_owner: Some(is_soulbound),
                    },
                )));
            }
        }
        Ok(None)
    }

    /// We want to track tokens in any offer/claims and tokenstore
    pub fn get_v1_from_write_table_item(
        table_item: &APIWriteTableItem,
        txn_version: i64,
        write_set_change_index: i64,
        txn_timestamp: chrono::NaiveDateTime,
        table_handle_to_owner: &TableHandleToOwner,
    ) -> anyhow::Result<Option<(Self, Option<CurrentTokenOwnershipV2>)>> {
        let table_item_data = table_item.data.as_ref().unwrap();

        let maybe_token = match TokenWriteSet::from_table_item_type(
            table_item_data.value_type.as_str(),
            &table_item_data.value,
            txn_version,
        )? {
            Some(TokenWriteSet::Token(inner)) => Some(inner),
            _ => None,
        };

        if let Some(token) = maybe_token {
            let table_handle = standardize_address(&table_item.handle.to_string());
            let amount = ensure_not_negative(token.amount);
            let token_id_struct = token.id;
            let token_data_id_struct = token_id_struct.token_data_id;
            let token_data_id = token_data_id_struct.to_id();

            let maybe_table_metadata = table_handle_to_owner.get(&table_handle);
            let (curr_token_ownership, owner_address, table_type) = match maybe_table_metadata {
                Some(tm) => {
                    if tm.table_type != "0x3::token::TokenStore" {
                        return Ok(None);
                    }
                    let owner_address = standardize_address(&tm.owner_address);
                    (
                        Some(CurrentTokenOwnershipV2 {
                            token_data_id: token_data_id.clone(),
                            property_version_v1: token_id_struct.property_version.clone(),
                            owner_address: owner_address.clone(),
                            storage_id: table_handle.clone(),
                            amount: amount.clone(),
                            table_type_v1: Some(tm.table_type.clone()),
                            token_properties_mutated_v1: Some(token.token_properties.clone()),
                            is_soulbound_v2: None,
                            token_standard: TokenStandard::V1.to_string(),
                            is_fungible_v2: None,
                            last_transaction_version: txn_version,
                            last_transaction_timestamp: txn_timestamp,
                            non_transferrable_by_owner: None,
                        }),
                        Some(owner_address),
                        Some(tm.table_type.clone()),
                    )
                },
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
                    transaction_version: txn_version,
                    write_set_change_index,
                    token_data_id,
                    property_version_v1: token_id_struct.property_version,
                    owner_address,
                    storage_id: table_handle,
                    amount,
                    table_type_v1: table_type,
                    token_properties_mutated_v1: Some(token.token_properties),
                    is_soulbound_v2: None,
                    token_standard: TokenStandard::V1.to_string(),
                    is_fungible_v2: None,
                    transaction_timestamp: txn_timestamp,
                    non_transferrable_by_owner: None,
                },
                curr_token_ownership,
            )))
        } else {
            Ok(None)
        }
    }

    /// We want to track tokens in any offer/claims and tokenstore
    pub fn get_v1_from_delete_table_item(
        table_item: &APIDeleteTableItem,
        txn_version: i64,
        write_set_change_index: i64,
        txn_timestamp: chrono::NaiveDateTime,
        table_handle_to_owner: &TableHandleToOwner,
    ) -> anyhow::Result<Option<(Self, Option<CurrentTokenOwnershipV2>)>> {
        let table_item_data = table_item.data.as_ref().unwrap();

        let maybe_token_id = match TokenWriteSet::from_table_item_type(
            table_item_data.key_type.as_str(),
            &table_item_data.key,
            txn_version,
        )? {
            Some(TokenWriteSet::TokenId(inner)) => Some(inner),
            _ => None,
        };

        if let Some(token_id_struct) = maybe_token_id {
            let table_handle = standardize_address(&table_item.handle.to_string());
            let token_data_id_struct = token_id_struct.token_data_id;
            let token_data_id = token_data_id_struct.to_id();

            let maybe_table_metadata = table_handle_to_owner.get(&table_handle);
            let (curr_token_ownership, owner_address, table_type) = match maybe_table_metadata {
                Some(tm) => {
                    if tm.table_type != "0x3::token::TokenStore" {
                        return Ok(None);
                    }
                    let owner_address = standardize_address(&tm.owner_address);
                    (
                        Some(CurrentTokenOwnershipV2 {
                            token_data_id: token_data_id.clone(),
                            property_version_v1: token_id_struct.property_version.clone(),
                            owner_address: owner_address.clone(),
                            storage_id: table_handle.clone(),
                            amount: BigDecimal::zero(),
                            table_type_v1: Some(tm.table_type.clone()),
                            token_properties_mutated_v1: None,
                            is_soulbound_v2: None,
                            token_standard: TokenStandard::V1.to_string(),
                            is_fungible_v2: None,
                            last_transaction_version: txn_version,
                            last_transaction_timestamp: txn_timestamp,
                            non_transferrable_by_owner: None,
                        }),
                        Some(owner_address),
                        Some(tm.table_type.clone()),
                    )
                },
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
                    transaction_version: txn_version,
                    write_set_change_index,
                    token_data_id,
                    property_version_v1: token_id_struct.property_version,
                    owner_address,
                    storage_id: table_handle,
                    amount: BigDecimal::zero(),
                    table_type_v1: table_type,
                    token_properties_mutated_v1: None,
                    is_soulbound_v2: None,
                    token_standard: TokenStandard::V1.to_string(),
                    is_fungible_v2: None,
                    transaction_timestamp: txn_timestamp,
                    non_transferrable_by_owner: None,
                },
                curr_token_ownership,
            )))
        } else {
            Ok(None)
        }
    }
}

impl CurrentTokenOwnershipV2Query {
    pub fn get_nft_by_token_data_id(
        conn: &mut PgPoolConnection,
        token_data_id: &str,
    ) -> anyhow::Result<NFTOwnershipV2> {
        let mut retried = 0;
        while retried < QUERY_RETRIES {
            retried += 1;
            match Self::get_nft_by_token_data_id_impl(conn, token_data_id) {
                Ok(inner) => {
                    return Ok(NFTOwnershipV2 {
                        token_data_id: inner.token_data_id.clone(),
                        owner_address: inner.owner_address.clone(),
                        is_soulbound: inner.is_soulbound_v2,
                    })
                },
                Err(_) => {
                    std::thread::sleep(std::time::Duration::from_millis(QUERY_RETRY_DELAY_MS));
                },
            }
        }
        Err(anyhow::anyhow!(
            "Failed to get nft by token data id: {}",
            token_data_id
        ))
    }

    fn get_nft_by_token_data_id_impl(
        conn: &mut PgPoolConnection,
        token_data_id: &str,
    ) -> diesel::QueryResult<Self> {
        current_token_ownerships_v2::table
            .filter(current_token_ownerships_v2::token_data_id.eq(token_data_id))
            .first::<Self>(conn)
    }
}
