// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]
#![allow(clippy::unused_unit)]

use super::{
    token_utils::{TokenDataIdType, TokenEvent},
    v2_token_utils::TokenStandard,
};
use crate::{schema::token_activities_v2, util::standardize_address};
use aptos_api_types::Event as APIEvent;
use bigdecimal::{BigDecimal, Zero};
use field_count::FieldCount;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(transaction_version, event_index))]
#[diesel(table_name = token_activities_v2)]
pub struct TokenActivityV2 {
    pub transaction_version: i64,
    pub event_index: i64,
    pub event_account_address: String,
    pub token_data_id: Option<String>,
    pub property_version_v1: Option<BigDecimal>,
    pub type_: String,
    pub from_address: Option<String>,
    pub to_address: Option<String>,
    pub token_amount: Option<BigDecimal>,
    pub before_value: Option<String>,
    pub after_value: Option<String>,
    pub entry_function_id_str: Option<String>,
    pub token_standard: String,
    pub is_fungible_v2: Option<bool>,
    pub transaction_timestamp: chrono::NaiveDateTime,
}

/// A simplified TokenActivity (excluded common fields) to reduce code duplication
struct TokenActivityHelperV2 {
    pub token_data_id_struct: TokenDataIdType,
    pub property_version: Option<BigDecimal>,
    pub from_address: Option<String>,
    pub to_address: Option<String>,
    pub token_amount: BigDecimal,
    pub before_value: Option<String>,
    pub after_value: Option<String>,
}

impl TokenActivityV2 {
    pub fn get_v1_from_parsed_event(
        event: &APIEvent,
        txn_version: i64,
        txn_timestamp: chrono::NaiveDateTime,
        event_index: i64,
        entry_function_id_str: &Option<String>,
    ) -> anyhow::Result<Option<Self>> {
        let event_type = event.typ.to_string();
        if let Some(token_event) =
            TokenEvent::from_event(event_type.as_str(), &event.data, txn_version)?
        {
            let event_account_address =
                standardize_address(&event.guid.account_address.to_string());
            let token_activity_helper = match token_event {
                TokenEvent::MintTokenEvent(inner) => TokenActivityHelperV2 {
                    token_data_id_struct: inner.id.clone(),
                    property_version: Some(BigDecimal::zero()),
                    from_address: Some(event_account_address.clone()),
                    to_address: None,
                    token_amount: inner.amount,
                    before_value: None,
                    after_value: None,
                },
                TokenEvent::BurnTokenEvent(inner) => TokenActivityHelperV2 {
                    token_data_id_struct: inner.id.token_data_id.clone(),
                    property_version: Some(inner.id.property_version.clone()),
                    from_address: Some(event_account_address.clone()),
                    to_address: None,
                    token_amount: inner.amount,
                    before_value: None,
                    after_value: None,
                },
                TokenEvent::MutateTokenPropertyMapEvent(inner) => TokenActivityHelperV2 {
                    token_data_id_struct: inner.new_id.token_data_id.clone(),
                    property_version: Some(inner.new_id.property_version),
                    from_address: Some(event_account_address.clone()),
                    to_address: None,
                    token_amount: BigDecimal::zero(),
                    before_value: None,
                    after_value: None,
                },
                TokenEvent::WithdrawTokenEvent(inner) => TokenActivityHelperV2 {
                    token_data_id_struct: inner.id.token_data_id.clone(),
                    property_version: Some(inner.id.property_version.clone()),
                    from_address: Some(event_account_address.clone()),
                    to_address: None,
                    token_amount: inner.amount,
                    before_value: None,
                    after_value: None,
                },
                TokenEvent::DepositTokenEvent(inner) => TokenActivityHelperV2 {
                    token_data_id_struct: inner.id.token_data_id.clone(),
                    property_version: Some(inner.id.property_version.clone()),
                    from_address: None,
                    to_address: Some(standardize_address(&event_account_address)),
                    token_amount: inner.amount,
                    before_value: None,
                    after_value: None,
                },
                TokenEvent::OfferTokenEvent(inner) => TokenActivityHelperV2 {
                    token_data_id_struct: inner.token_id.token_data_id.clone(),
                    property_version: Some(inner.token_id.property_version.clone()),
                    from_address: Some(event_account_address.clone()),
                    to_address: Some(standardize_address(&inner.to_address)),
                    token_amount: inner.amount,
                    before_value: None,
                    after_value: None,
                },
                TokenEvent::CancelTokenOfferEvent(inner) => TokenActivityHelperV2 {
                    token_data_id_struct: inner.token_id.token_data_id.clone(),
                    property_version: Some(inner.token_id.property_version.clone()),
                    from_address: Some(event_account_address.clone()),
                    to_address: Some(standardize_address(&inner.to_address)),
                    token_amount: inner.amount,
                    before_value: None,
                    after_value: None,
                },
                TokenEvent::ClaimTokenEvent(inner) => TokenActivityHelperV2 {
                    token_data_id_struct: inner.token_id.token_data_id.clone(),
                    property_version: Some(inner.token_id.property_version.clone()),
                    from_address: Some(event_account_address.clone()),
                    to_address: Some(standardize_address(&inner.to_address)),
                    token_amount: inner.amount,
                    before_value: None,
                    after_value: None,
                },
            };
            let token_data_id_struct = token_activity_helper.token_data_id_struct;
            Ok(Some(Self {
                transaction_version: txn_version,
                event_index,
                event_account_address,
                token_data_id: Some(token_data_id_struct.to_id()),
                property_version_v1: token_activity_helper.property_version,
                type_: event_type.to_string(),
                from_address: token_activity_helper.from_address,
                to_address: token_activity_helper.to_address,
                token_amount: Some(token_activity_helper.token_amount),
                before_value: token_activity_helper.before_value,
                after_value: token_activity_helper.after_value,
                entry_function_id_str: entry_function_id_str.clone(),
                token_standard: TokenStandard::V2.to_string(),
                is_fungible_v2: None,
                transaction_timestamp: txn_timestamp,
            }))
        } else {
            Ok(None)
        }
    }
}
