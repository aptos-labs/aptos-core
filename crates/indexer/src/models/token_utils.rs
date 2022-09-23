// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]

use anyhow::Context;
use aptos_api_types::deserialize_from_string;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Formatter};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TokenDataIdType {
    pub creator: String,
    pub collection: String,
    pub name: String,
}

impl fmt::Display for TokenDataIdType {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}::{}::{}", self.creator, self.collection, self.name)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CollectionDataIdType {
    pub creator: String,
    pub name: String,
}

impl fmt::Display for CollectionDataIdType {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}::{}", self.creator, self.name)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TokenIdType {
    pub token_data_id: TokenDataIdType,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub property_version: bigdecimal::BigDecimal,
}

impl fmt::Display for TokenIdType {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}::{}", self.token_data_id, self.property_version)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TokenDataType {
    pub default_properties: serde_json::Value,
    pub description: String,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub largest_property_version: bigdecimal::BigDecimal,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub maximum: bigdecimal::BigDecimal,
    pub mutability_config: TokenDataMutabilityConfigType,
    pub name: String,
    pub royalty: RoyaltyType,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub supply: bigdecimal::BigDecimal,
    pub uri: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TokenDataMutabilityConfigType {
    pub description: bool,
    pub maximum: bool,
    pub properties: bool,
    pub royalty: bool,
    pub uri: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RoyaltyType {
    pub payee_address: String,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub royalty_points_denominator: bigdecimal::BigDecimal,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub royalty_points_numerator: bigdecimal::BigDecimal,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TokenType {
    #[serde(deserialize_with = "deserialize_from_string")]
    pub amount: bigdecimal::BigDecimal,
    pub id: TokenIdType,
    pub token_properties: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CollectionDataType {
    pub description: String,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub maximum: bigdecimal::BigDecimal,
    pub mutability_config: CollectionDataMutabilityConfigType,
    pub name: String,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub supply: bigdecimal::BigDecimal,
    pub uri: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CollectionDataMutabilityConfigType {
    pub description: bool,
    pub maximum: bool,
    pub uri: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TokenWriteSet {
    TokenDataId(TokenDataIdType),
    TokenId(TokenIdType),
    TokenData(TokenDataType),
    Token(TokenType),
    CollectionData(CollectionDataType),
}

impl TokenWriteSet {
    pub fn from_table_item_type(
        data_type: &str,
        data: &serde_json::Value,
        txn_version: i64,
    ) -> anyhow::Result<Option<TokenWriteSet>> {
        match data_type {
            "0x3::token::TokenDataId" => serde_json::from_value(data.clone())
                .map(|inner| Some(TokenWriteSet::TokenDataId(inner)))
                .context(format!(
                    "version {} failed! failed to parse type {}, data {:?}",
                    txn_version, data_type, data
                )),
            "0x3::token::TokenId" => serde_json::from_value(data.clone())
                .map(|inner| Some(TokenWriteSet::TokenId(inner)))
                .context(format!(
                    "version {} failed! failed to parse type {}, data {:?}",
                    txn_version, data_type, data
                )),
            "0x3::token::TokenData" => serde_json::from_value(data.clone())
                .map(|inner| Some(TokenWriteSet::TokenData(inner)))
                .context(format!(
                    "version {} failed! failed to parse type {}, data {:?}",
                    txn_version, data_type, data
                )),
            "0x3::token::Token" => serde_json::from_value(data.clone())
                .map(|inner| Some(TokenWriteSet::Token(inner)))
                .context(format!(
                    "version {} failed! failed to parse type {}, data {:?}",
                    txn_version, data_type, data
                )),
            "0x3::token::CollectionData" => serde_json::from_value(data.clone())
                .map(|inner| Some(TokenWriteSet::CollectionData(inner)))
                .context(format!(
                    "version {} failed! failed to parse type {}, data {:?}",
                    txn_version, data_type, data
                )),
            _ => Ok(None),
        }
    }
}
