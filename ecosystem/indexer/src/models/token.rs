// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::extra_unused_lifetimes)]
use crate::{models::events::Event, schema::token_datas};
use aptos_rest_client::types;
use std::{fmt, fmt::Formatter};

use serde::{Deserialize, Serialize};

#[derive(Associations, Debug, Identifiable, Insertable, Queryable, Serialize, Clone)]
#[diesel(table_name = "token_datas")]
#[primary_key(token_data_id)]
pub struct TokenData {
    pub token_data_id: String,
    pub creator: String,
    pub collection: String,
    pub name: String,
    pub description: String,
    pub max_amount: i64,
    pub supply: i64,
    pub uri: String,
    pub royalty_payee_address: String,
    pub royalty_points_denominator: i64,
    pub royalty_points_numerator: i64,
    pub mutability_config: String,
    pub property_keys: String,
    pub property_values: String,
    pub property_types: String,
    pub minted_at: chrono::NaiveDateTime,
    pub inserted_at: chrono::NaiveDateTime,
    pub last_minted_at: chrono::NaiveDateTime,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TokenDataId {
    pub creator: String,
    pub collection: String,
    pub name: String,
}

impl fmt::Display for TokenDataId {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}::{}::{}", self.creator, self.collection, self.name)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TokenId {
    pub token_data_id: TokenDataId,
    #[serde(deserialize_with = "types::deserialize_from_string")]
    pub property_version: i64,
}

impl fmt::Display for TokenId {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}::{}", self.token_data_id, self.property_version)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WithdrawEventType {
    #[serde(deserialize_with = "types::deserialize_from_string")]
    pub amount: i64,
    pub id: TokenId,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DepositEventType {
    #[serde(deserialize_with = "types::deserialize_from_string")]
    pub amount: i64,
    pub id: TokenId,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateTokenDataEventType {
    pub id: TokenDataId,
    pub description: String,
    #[serde(deserialize_with = "types::deserialize_from_string")]
    pub maximum: i64,
    pub uri: String,
    pub royalty_payee_address: String,
    #[serde(deserialize_with = "types::deserialize_from_string")]
    pub royalty_points_denominator: i64,
    #[serde(deserialize_with = "types::deserialize_from_string")]
    pub royalty_points_numerator: i64,
    pub name: String,
    pub mutability_config: serde_json::Value,
    pub property_keys: serde_json::Value,
    pub property_values: serde_json::Value,
    pub property_types: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MintTokenEventType {
    #[serde(deserialize_with = "types::deserialize_from_string")]
    pub amount: i64,
    pub id: TokenDataId,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BurnTokenEventType {
    #[serde(deserialize_with = "types::deserialize_from_string")]
    pub amount: i64,
    pub id: TokenId,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MutateTokenPropertyMapEventType {
    pub old_id: TokenId,
    pub new_id: TokenId,
    pub keys: serde_json::Value,
    pub values: serde_json::Value,
    pub types: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateCollectionEventType {
    pub creator: String,
    pub collection_name: String,
    pub uri: String,
    pub description: String,
    #[serde(deserialize_with = "types::deserialize_from_string")]
    pub maximum: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TokenEvent {
    WithdrawEvent(WithdrawEventType),
    DepositEvent(DepositEventType),
    CreateTokenDataEvent(CreateTokenDataEventType),
    MintTokenEvent(MintTokenEventType),
    CollectionCreationEvent(CreateCollectionEventType),
    BurnTokenEvent(BurnTokenEventType),
    MutateTokenPropertyMapEvent(MutateTokenPropertyMapEventType),
}

impl TokenEvent {
    pub fn from_event(event: &Event) -> Option<TokenEvent> {
        let data = event.data.clone();
        match event.type_.as_str() {
            "0x3::token::WithdrawEvent" => {
                let event = serde_json::from_value::<WithdrawEventType>(data).unwrap();
                Some(TokenEvent::WithdrawEvent(event))
            }
            "0x3::token::DepositEvent" => {
                let event = serde_json::from_value::<DepositEventType>(data).unwrap();
                Some(TokenEvent::DepositEvent(event))
            }
            "0x3::token::CreateTokenDataEvent" => {
                let event = serde_json::from_value::<CreateTokenDataEventType>(data).unwrap();
                Some(TokenEvent::CreateTokenDataEvent(event))
            }
            "0x3::token::CreateCollectionEvent" => {
                let event = serde_json::from_value::<CreateCollectionEventType>(data).unwrap();
                Some(TokenEvent::CollectionCreationEvent(event))
            }
            "0x3::token::BurnTokenEvent" => {
                let event = serde_json::from_value::<BurnTokenEventType>(data).unwrap();
                Some(TokenEvent::BurnTokenEvent(event))
            }
            "0x3::token::MutateTokenPropertyMapEvent" => {
                let event =
                    serde_json::from_value::<MutateTokenPropertyMapEventType>(data).unwrap();
                Some(TokenEvent::MutateTokenPropertyMapEvent(event))
            }
            "0x3::token::MintTokenEvent" => {
                let event = serde_json::from_value::<MintTokenEventType>(data).unwrap();
                Some(TokenEvent::MintTokenEvent(event))
            }
            _ => None,
        }
    }
}
