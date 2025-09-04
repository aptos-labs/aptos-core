// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]
#![allow(clippy::unused_unit)]

use super::token_utils::{TokenDataIdType, TokenEvent};
use crate::{
    schema::token_activities,
    util::{parse_timestamp, standardize_address},
};
use velor_api_types::{Event as APIEvent, Transaction as APITransaction};
use bigdecimal::{BigDecimal, Zero};
use field_count::FieldCount;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(
    transaction_version,
    event_account_address,
    event_creation_number,
    event_sequence_number
))]
#[diesel(table_name = token_activities)]
pub struct TokenActivity {
    pub transaction_version: i64,
    pub event_account_address: String,
    pub event_creation_number: i64,
    pub event_sequence_number: i64,
    pub token_data_id_hash: String,
    pub property_version: BigDecimal,
    pub creator_address: String,
    pub collection_name: String,
    pub name: String,
    pub transfer_type: String,
    pub from_address: Option<String>,
    pub to_address: Option<String>,
    pub token_amount: BigDecimal,
    pub coin_type: Option<String>,
    pub coin_amount: Option<BigDecimal>,
    pub collection_data_id_hash: String,
    pub transaction_timestamp: chrono::NaiveDateTime,
    pub event_index: Option<i64>,
}

/// A simplified TokenActivity (excluded common fields) to reduce code duplication
struct TokenActivityHelper<'a> {
    pub token_data_id: &'a TokenDataIdType,
    pub property_version: BigDecimal,
    pub from_address: Option<String>,
    pub to_address: Option<String>,
    pub token_amount: BigDecimal,
    pub coin_type: Option<String>,
    pub coin_amount: Option<BigDecimal>,
}

impl TokenActivity {
    pub fn from_transaction(transaction: &APITransaction) -> Vec<Self> {
        let mut token_activities = vec![];
        if let APITransaction::UserTransaction(user_txn) = transaction {
            for (index, event) in user_txn.events.iter().enumerate() {
                let txn_version = user_txn.info.version.0 as i64;
                let event_type = event.typ.to_string();
                if let Some(token_event) =
                    TokenEvent::from_event(event_type.as_str(), &event.data, txn_version).unwrap()
                {
                    token_activities.push(Self::from_parsed_event(
                        &event_type,
                        event,
                        &token_event,
                        txn_version,
                        parse_timestamp(user_txn.timestamp.0, txn_version),
                        index as i64,
                    ))
                }
            }
        }
        token_activities
    }

    pub fn from_parsed_event(
        event_type: &str,
        event: &APIEvent,
        token_event: &TokenEvent,
        txn_version: i64,
        txn_timestamp: chrono::NaiveDateTime,
        event_index: i64,
    ) -> Self {
        let event_account_address = standardize_address(&event.guid.account_address.to_string());
        let event_creation_number = event.guid.creation_number.0 as i64;
        let event_sequence_number = event.sequence_number.0 as i64;
        let token_activity_helper = match token_event {
            TokenEvent::MintTokenEvent(inner) => TokenActivityHelper {
                token_data_id: &inner.id,
                property_version: BigDecimal::zero(),
                from_address: Some(event_account_address.clone()),
                to_address: None,
                token_amount: inner.amount.clone(),
                coin_type: None,
                coin_amount: None,
            },
            TokenEvent::BurnTokenEvent(inner) => TokenActivityHelper {
                token_data_id: &inner.id.token_data_id,
                property_version: inner.id.property_version.clone(),
                from_address: Some(event_account_address.clone()),
                to_address: None,
                token_amount: inner.amount.clone(),
                coin_type: None,
                coin_amount: None,
            },
            TokenEvent::MutateTokenPropertyMapEvent(inner) => TokenActivityHelper {
                token_data_id: &inner.new_id.token_data_id,
                property_version: inner.new_id.property_version.clone(),
                from_address: Some(event_account_address.clone()),
                to_address: None,
                token_amount: BigDecimal::zero(),
                coin_type: None,
                coin_amount: None,
            },
            TokenEvent::WithdrawTokenEvent(inner) => TokenActivityHelper {
                token_data_id: &inner.id.token_data_id,
                property_version: inner.id.property_version.clone(),
                from_address: Some(event_account_address.clone()),
                to_address: None,
                token_amount: inner.amount.clone(),
                coin_type: None,
                coin_amount: None,
            },
            TokenEvent::DepositTokenEvent(inner) => TokenActivityHelper {
                token_data_id: &inner.id.token_data_id,
                property_version: inner.id.property_version.clone(),
                from_address: None,
                to_address: Some(standardize_address(&event_account_address)),
                token_amount: inner.amount.clone(),
                coin_type: None,
                coin_amount: None,
            },
            TokenEvent::OfferTokenEvent(inner) => TokenActivityHelper {
                token_data_id: &inner.token_id.token_data_id,
                property_version: inner.token_id.property_version.clone(),
                from_address: Some(event_account_address.clone()),
                to_address: Some(standardize_address(&inner.to_address)),
                token_amount: inner.amount.clone(),
                coin_type: None,
                coin_amount: None,
            },
            TokenEvent::CancelTokenOfferEvent(inner) => TokenActivityHelper {
                token_data_id: &inner.token_id.token_data_id,
                property_version: inner.token_id.property_version.clone(),
                from_address: Some(event_account_address.clone()),
                to_address: Some(standardize_address(&inner.to_address)),
                token_amount: inner.amount.clone(),
                coin_type: None,
                coin_amount: None,
            },
            TokenEvent::ClaimTokenEvent(inner) => TokenActivityHelper {
                token_data_id: &inner.token_id.token_data_id,
                property_version: inner.token_id.property_version.clone(),
                from_address: Some(event_account_address.clone()),
                to_address: Some(standardize_address(&inner.to_address)),
                token_amount: inner.amount.clone(),
                coin_type: None,
                coin_amount: None,
            },
        };
        let token_data_id = token_activity_helper.token_data_id;
        Self {
            event_account_address,
            event_creation_number,
            event_sequence_number,
            token_data_id_hash: token_data_id.to_hash(),
            property_version: token_activity_helper.property_version,
            collection_data_id_hash: token_data_id.get_collection_data_id_hash(),
            creator_address: standardize_address(&token_data_id.creator),
            collection_name: token_data_id.get_collection_trunc(),
            name: token_data_id.get_name_trunc(),
            transaction_version: txn_version,
            transfer_type: event_type.to_string(),
            from_address: token_activity_helper.from_address,
            to_address: token_activity_helper.to_address,
            token_amount: token_activity_helper.token_amount,
            coin_type: token_activity_helper.coin_type,
            coin_amount: token_activity_helper.coin_amount,
            transaction_timestamp: txn_timestamp,
            event_index: Some(event_index),
        }
    }
}
