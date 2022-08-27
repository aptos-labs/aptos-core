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
    pub max_amount: bigdecimal::BigDecimal,
    pub supply: bigdecimal::BigDecimal,
    pub uri: String,
    pub royalty_payee_address: String,
    pub royalty_points_denominator: bigdecimal::BigDecimal,
    pub royalty_points_numerator: bigdecimal::BigDecimal,
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
    pub property_version: bigdecimal::BigDecimal,
}

impl fmt::Display for TokenId {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}::{}", self.token_data_id, self.property_version)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WithdrawEventType {
    #[serde(deserialize_with = "types::deserialize_from_string")]
    pub amount: bigdecimal::BigDecimal,
    pub id: TokenId,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DepositEventType {
    #[serde(deserialize_with = "types::deserialize_from_string")]
    pub amount: bigdecimal::BigDecimal,
    pub id: TokenId,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateTokenDataEventType {
    pub id: TokenDataId,
    pub description: String,
    #[serde(deserialize_with = "types::deserialize_from_string")]
    pub maximum: u64,
    pub uri: String,
    pub royalty_payee_address: String,
    #[serde(deserialize_with = "types::deserialize_from_string")]
    pub royalty_points_denominator: bigdecimal::BigDecimal,
    #[serde(deserialize_with = "types::deserialize_from_string")]
    pub royalty_points_numerator: bigdecimal::BigDecimal,
    pub name: String,
    pub mutability_config: serde_json::Value,
    pub property_keys: serde_json::Value,
    pub property_values: serde_json::Value,
    pub property_types: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MintTokenEventType {
    #[serde(deserialize_with = "types::deserialize_from_string")]
    pub amount: bigdecimal::BigDecimal,
    pub id: TokenDataId,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BurnTokenEventType {
    #[serde(deserialize_with = "types::deserialize_from_string")]
    pub amount: bigdecimal::BigDecimal,
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
    pub maximum: u64,
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
            "0x3::token::WithdrawEvent" => serde_json::from_value(data)
                .map(|inner| Some(TokenEvent::WithdrawEvent(inner)))
                .unwrap_or(None),
            "0x3::token::DepositEvent" => serde_json::from_value(data)
                .map(|inner| Some(TokenEvent::DepositEvent(inner)))
                .unwrap_or(None),
            "0x3::token::CreateTokenDataEvent" => serde_json::from_value(data)
                .map(|inner| Some(TokenEvent::CreateTokenDataEvent(inner)))
                .unwrap_or(None),
            "0x3::token::CreateCollectionEvent" => serde_json::from_value(data)
                .map(|inner| Some(TokenEvent::CollectionCreationEvent(inner)))
                .unwrap_or(None),
            "0x3::token::BurnTokenEvent" => serde_json::from_value(data)
                .map(|inner| Some(TokenEvent::BurnTokenEvent(inner)))
                .unwrap_or(None),
            "0x3::token::MutateTokenPropertyMapEvent" => serde_json::from_value(data)
                .map(|inner| Some(TokenEvent::MutateTokenPropertyMapEvent(inner)))
                .unwrap_or(None),
            "0x3::token::MintTokenEvent" => serde_json::from_value(data)
                .map(|inner| Some(TokenEvent::MintTokenEvent(inner)))
                .unwrap_or(None),
            _ => None,
        }
    }
}
