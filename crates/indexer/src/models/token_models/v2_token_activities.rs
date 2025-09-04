// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]
#![allow(clippy::unused_unit)]

use super::{
    token_utils::{TokenDataIdType, TokenEvent},
    v2_token_datas::TokenDataV2,
    v2_token_utils::{TokenStandard, TokenV2AggregatedDataMapping, V2TokenEvent},
};
use crate::{
    database::PgPoolConnection, models::coin_models::v2_fungible_asset_utils::FungibleAssetEvent,
    schema::token_activities_v2, util::standardize_address,
};
use velor_api_types::Event as APIEvent;
use bigdecimal::{BigDecimal, One, Zero};
use field_count::FieldCount;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(transaction_version, event_index))]
#[diesel(table_name = token_activities_v2)]
pub struct TokenActivityV2 {
    pub transaction_version: i64,
    pub event_index: i64,
    pub event_account_address: String,
    pub token_data_id: String,
    pub property_version_v1: BigDecimal,
    pub type_: String,
    pub from_address: Option<String>,
    pub to_address: Option<String>,
    pub token_amount: BigDecimal,
    pub before_value: Option<String>,
    pub after_value: Option<String>,
    pub entry_function_id_str: Option<String>,
    pub token_standard: String,
    pub is_fungible_v2: Option<bool>,
    pub transaction_timestamp: chrono::NaiveDateTime,
}

/// A simplified TokenActivity (excluded common fields) to reduce code duplication
struct TokenActivityHelperV1 {
    pub token_data_id_struct: TokenDataIdType,
    pub property_version: BigDecimal,
    pub from_address: Option<String>,
    pub to_address: Option<String>,
    pub token_amount: BigDecimal,
}

/// A simplified TokenActivity (excluded common fields) to reduce code duplication
struct TokenActivityHelperV2 {
    pub from_address: Option<String>,
    pub to_address: Option<String>,
    pub token_amount: BigDecimal,
    pub before_value: Option<String>,
    pub after_value: Option<String>,
}

impl TokenActivityV2 {
    /// We'll go from 0x1::fungible_asset::withdraw/deposit events.
    /// We're guaranteed to find a 0x1::fungible_asset::FungibleStore which has a pointer to the
    /// fungible asset metadata which could be a token. We'll either find that token in token_v2_metadata
    /// or by looking up the postgres table.
    /// TODO: Create artificial events for mint and burn. There are no mint and burn events so we'll have to
    /// add all the deposits/withdrawals and if it's positive/negative it's a mint/burn.
    pub fn get_ft_v2_from_parsed_event(
        event: &APIEvent,
        txn_version: i64,
        txn_timestamp: chrono::NaiveDateTime,
        event_index: i64,
        entry_function_id_str: &Option<String>,
        token_v2_metadata: &TokenV2AggregatedDataMapping,
        conn: &mut PgPoolConnection,
    ) -> anyhow::Result<Option<Self>> {
        let event_type = event.typ.to_string();
        if let Some(fa_event) =
            &FungibleAssetEvent::from_event(event_type.as_str(), &event.data, txn_version)?
        {
            let event_account_address =
                standardize_address(&event.guid.account_address.to_string());
            // The event account address will also help us find fungible store which tells us where to find
            // the metadata
            if let Some(metadata) = token_v2_metadata.get(&event_account_address) {
                let object_core = &metadata.object.object_core;
                let fungible_asset = metadata.fungible_asset_store.as_ref().unwrap();
                let maybe_token_data_id = fungible_asset.metadata.get_reference_address();
                // Now we try to see if the fungible asset is actually a token. If it's not token, return early
                let is_token = if token_v2_metadata.get(&maybe_token_data_id).is_some() {
                    true
                } else {
                    // Look up in the db
                    TokenDataV2::is_address_token(conn, &maybe_token_data_id)
                };
                if !is_token {
                    return Ok(None);
                }

                let token_activity_helper = match fa_event {
                    FungibleAssetEvent::WithdrawEvent(inner) => TokenActivityHelperV2 {
                        from_address: Some(object_core.get_owner_address()),
                        to_address: None,
                        token_amount: inner.amount.clone(),
                        before_value: None,
                        after_value: None,
                    },
                    FungibleAssetEvent::DepositEvent(inner) => TokenActivityHelperV2 {
                        from_address: None,
                        to_address: Some(object_core.get_owner_address()),
                        token_amount: inner.amount.clone(),
                        before_value: None,
                        after_value: None,
                    },
                };

                return Ok(Some(Self {
                    transaction_version: txn_version,
                    event_index,
                    event_account_address,
                    token_data_id: maybe_token_data_id.clone(),
                    property_version_v1: BigDecimal::zero(),
                    type_: event_type.to_string(),
                    from_address: token_activity_helper.from_address,
                    to_address: token_activity_helper.to_address,
                    token_amount: token_activity_helper.token_amount,
                    before_value: token_activity_helper.before_value,
                    after_value: token_activity_helper.after_value,
                    entry_function_id_str: entry_function_id_str.clone(),
                    token_standard: TokenStandard::V2.to_string(),
                    is_fungible_v2: Some(true),
                    transaction_timestamp: txn_timestamp,
                }));
            }
        }
        Ok(None)
    }

    /// Parse token v2 events, and then get the metadata from resources via token_v2_metadata
    pub fn get_nft_v2_from_parsed_event(
        event: &APIEvent,
        txn_version: i64,
        txn_timestamp: chrono::NaiveDateTime,
        event_index: i64,
        entry_function_id_str: &Option<String>,
        token_v2_metadata: &TokenV2AggregatedDataMapping,
    ) -> anyhow::Result<Option<Self>> {
        let event_type = event.typ.to_string();
        if let Some(token_event) =
            &V2TokenEvent::from_event(event_type.as_str(), &event.data, txn_version)?
        {
            let event_account_address =
                standardize_address(&event.guid.account_address.to_string());
            // burn and mint events are attached to the collection. The rest should be attached to the token
            let token_data_id = match token_event {
                V2TokenEvent::MintEvent(inner) => inner.get_token_address(),
                V2TokenEvent::BurnEvent(inner) => inner.get_token_address(),
                V2TokenEvent::TransferEvent(inner) => inner.get_object_address(),
                _ => event_account_address.clone(),
            };

            if let Some(metadata) = token_v2_metadata.get(&token_data_id) {
                let object_core = &metadata.object.object_core;
                let token_activity_helper = match token_event {
                    V2TokenEvent::MintEvent(_) => TokenActivityHelperV2 {
                        from_address: Some(object_core.get_owner_address()),
                        to_address: None,
                        token_amount: BigDecimal::one(),
                        before_value: None,
                        after_value: None,
                    },
                    V2TokenEvent::TokenMutationEvent(inner) => TokenActivityHelperV2 {
                        from_address: Some(object_core.get_owner_address()),
                        to_address: None,
                        token_amount: BigDecimal::zero(),
                        before_value: Some(inner.old_value.clone()),
                        after_value: Some(inner.new_value.clone()),
                    },
                    V2TokenEvent::BurnEvent(_) => TokenActivityHelperV2 {
                        from_address: Some(object_core.get_owner_address()),
                        to_address: None,
                        token_amount: BigDecimal::one(),
                        before_value: None,
                        after_value: None,
                    },
                    V2TokenEvent::TransferEvent(inner) => TokenActivityHelperV2 {
                        from_address: Some(inner.get_from_address()),
                        to_address: Some(inner.get_to_address()),
                        token_amount: BigDecimal::one(),
                        before_value: None,
                        after_value: None,
                    },
                };
                return Ok(Some(Self {
                    transaction_version: txn_version,
                    event_index,
                    event_account_address,
                    token_data_id,
                    property_version_v1: BigDecimal::zero(),
                    type_: event_type.to_string(),
                    from_address: token_activity_helper.from_address,
                    to_address: token_activity_helper.to_address,
                    token_amount: token_activity_helper.token_amount,
                    before_value: token_activity_helper.before_value,
                    after_value: token_activity_helper.after_value,
                    entry_function_id_str: entry_function_id_str.clone(),
                    token_standard: TokenStandard::V2.to_string(),
                    is_fungible_v2: Some(false),
                    transaction_timestamp: txn_timestamp,
                }));
            }
        }
        Ok(None)
    }

    /// Pretty straightforward, parse specific events related to tokens into the right format
    pub fn get_v1_from_parsed_event(
        event: &APIEvent,
        txn_version: i64,
        txn_timestamp: chrono::NaiveDateTime,
        event_index: i64,
        entry_function_id_str: &Option<String>,
    ) -> anyhow::Result<Option<Self>> {
        let event_type = event.typ.to_string();
        if let Some(token_event) =
            &TokenEvent::from_event(event_type.as_str(), &event.data, txn_version)?
        {
            let event_account_address =
                standardize_address(&event.guid.account_address.to_string());
            let token_activity_helper = match token_event {
                TokenEvent::MintTokenEvent(inner) => TokenActivityHelperV1 {
                    token_data_id_struct: inner.id.clone(),
                    property_version: BigDecimal::zero(),
                    from_address: Some(event_account_address.clone()),
                    to_address: None,
                    token_amount: inner.amount.clone(),
                },
                TokenEvent::BurnTokenEvent(inner) => TokenActivityHelperV1 {
                    token_data_id_struct: inner.id.token_data_id.clone(),
                    property_version: inner.id.property_version.clone(),
                    from_address: Some(event_account_address.clone()),
                    to_address: None,
                    token_amount: inner.amount.clone(),
                },
                TokenEvent::MutateTokenPropertyMapEvent(inner) => TokenActivityHelperV1 {
                    token_data_id_struct: inner.new_id.token_data_id.clone(),
                    property_version: inner.new_id.property_version.clone(),
                    from_address: Some(event_account_address.clone()),
                    to_address: None,
                    token_amount: BigDecimal::zero(),
                },
                TokenEvent::WithdrawTokenEvent(inner) => TokenActivityHelperV1 {
                    token_data_id_struct: inner.id.token_data_id.clone(),
                    property_version: inner.id.property_version.clone(),
                    from_address: Some(event_account_address.clone()),
                    to_address: None,
                    token_amount: inner.amount.clone(),
                },
                TokenEvent::DepositTokenEvent(inner) => TokenActivityHelperV1 {
                    token_data_id_struct: inner.id.token_data_id.clone(),
                    property_version: inner.id.property_version.clone(),
                    from_address: None,
                    to_address: Some(standardize_address(&event_account_address)),
                    token_amount: inner.amount.clone(),
                },
                TokenEvent::OfferTokenEvent(inner) => TokenActivityHelperV1 {
                    token_data_id_struct: inner.token_id.token_data_id.clone(),
                    property_version: inner.token_id.property_version.clone(),
                    from_address: Some(event_account_address.clone()),
                    to_address: Some(standardize_address(&inner.to_address)),
                    token_amount: inner.amount.clone(),
                },
                TokenEvent::CancelTokenOfferEvent(inner) => TokenActivityHelperV1 {
                    token_data_id_struct: inner.token_id.token_data_id.clone(),
                    property_version: inner.token_id.property_version.clone(),
                    from_address: Some(event_account_address.clone()),
                    to_address: Some(standardize_address(&inner.to_address)),
                    token_amount: inner.amount.clone(),
                },
                TokenEvent::ClaimTokenEvent(inner) => TokenActivityHelperV1 {
                    token_data_id_struct: inner.token_id.token_data_id.clone(),
                    property_version: inner.token_id.property_version.clone(),
                    from_address: Some(event_account_address.clone()),
                    to_address: Some(standardize_address(&inner.to_address)),
                    token_amount: inner.amount.clone(),
                },
            };
            let token_data_id_struct = token_activity_helper.token_data_id_struct;
            return Ok(Some(Self {
                transaction_version: txn_version,
                event_index,
                event_account_address,
                token_data_id: token_data_id_struct.to_id(),
                property_version_v1: token_activity_helper.property_version,
                type_: event_type.to_string(),
                from_address: token_activity_helper.from_address,
                to_address: token_activity_helper.to_address,
                token_amount: token_activity_helper.token_amount,
                before_value: None,
                after_value: None,
                entry_function_id_str: entry_function_id_str.clone(),
                token_standard: TokenStandard::V1.to_string(),
                is_fungible_v2: None,
                transaction_timestamp: txn_timestamp,
            }));
        }
        Ok(None)
    }
}
