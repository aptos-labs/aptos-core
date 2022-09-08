// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]
#![allow(clippy::unused_unit)]

use crate::{
    schema::{collection_datas, token_datas, token_ownerships, tokens},
    util::u64_to_bigdecimal,
};
use aptos_protos::tokens::v1::{
    CollectionData as CollectionDataPB, Token as TokenPB, TokenData as TokenDataPB, Tokens,
};
use field_count::FieldCount;
use serde::Serialize;

#[derive(Debug, FieldCount, Identifiable, Insertable, Queryable, Serialize)]
#[primary_key(
    creator_address,
    collection_name,
    name,
    property_version,
    transaction_version
)]
#[diesel(table_name = "tokens")]
pub struct Token {
    pub creator_address: String,
    pub collection_name: String,
    pub name: String,
    pub property_version: bigdecimal::BigDecimal,
    pub transaction_version: i64,
    pub token_properties: serde_json::Value,
    // Default time columns
    pub inserted_at: chrono::NaiveDateTime,
}

impl Token {
    pub fn from_tokens(
        input_tokens: &Tokens,
    ) -> (
        Vec<Self>,
        Vec<TokenData>,
        Vec<TokenOwnership>,
        Vec<CollectionData>,
    ) {
        let mut tokens = vec![];
        let mut token_datas = vec![];
        let mut token_ownerships = vec![];
        let mut collection_datas = vec![];
        for token in &input_tokens.tokens {
            tokens.push(Self::from_token(token));
            token_ownerships.push(TokenOwnership::from_token(token));
        }
        for token_data in &input_tokens.token_datas {
            token_datas.push(TokenData::from_token_data(token_data));
        }
        for collection_data in &input_tokens.collection_datas {
            collection_datas.push(CollectionData::from_collection_data(collection_data));
        }
        (tokens, token_datas, token_ownerships, collection_datas)
    }

    fn from_token(token: &TokenPB) -> Self {
        let token_data_id = token
            .token_id
            .as_ref()
            .unwrap()
            .token_data_id
            .as_ref()
            .unwrap()
            .clone();
        Self {
            creator_address: token_data_id.creator_address,
            collection_name: token_data_id.collection_name,
            name: token_data_id.name,
            property_version: u64_to_bigdecimal(token.token_id.as_ref().unwrap().property_version),
            transaction_version: token.transaction_version as i64,
            token_properties: serde_json::to_value(&token.token_properties).unwrap(),
            inserted_at: chrono::Utc::now().naive_utc(),
        }
    }
}

#[derive(Debug, FieldCount, Identifiable, Insertable, Queryable, Serialize)]
#[primary_key(creator_address, collection_name, name, transaction_version)]
#[diesel(table_name = "token_datas")]
pub struct TokenData {
    pub creator_address: String,
    pub collection_name: String,
    pub name: String,
    pub transaction_version: i64,
    pub maximum: bigdecimal::BigDecimal,
    pub supply: bigdecimal::BigDecimal,
    pub largest_property_version: bigdecimal::BigDecimal,
    pub metadata_uri: String,
    pub royalty_points_numerator: bigdecimal::BigDecimal,
    pub royalty_points_denominator: bigdecimal::BigDecimal,
    pub maximum_mutable: bool,
    pub uri_mutable: bool,
    pub description_mutable: bool,
    pub properties_mutable: bool,
    pub royalty_mutable: bool,
    pub default_properties: serde_json::Value,
    // Default time columns
    pub inserted_at: chrono::NaiveDateTime,
}

impl TokenData {
    fn from_token_data(token_data: &TokenDataPB) -> Self {
        let token_data_id = token_data.token_data_id.as_ref().unwrap().clone();
        Self {
            creator_address: token_data_id.creator_address,
            collection_name: token_data_id.collection_name,
            name: token_data_id.name,
            transaction_version: token_data.transaction_version as i64,
            maximum: u64_to_bigdecimal(token_data.maximum),
            supply: u64_to_bigdecimal(token_data.supply),
            largest_property_version: u64_to_bigdecimal(token_data.largest_property_version),
            metadata_uri: token_data.metadata_uri.clone(),
            royalty_points_numerator: u64_to_bigdecimal(token_data.royalty_points_denominator),
            royalty_points_denominator: u64_to_bigdecimal(token_data.royalty_points_denominator),
            maximum_mutable: token_data.maximum_mutable,
            uri_mutable: token_data.uri_mutable,
            description_mutable: token_data.description_mutable,
            properties_mutable: token_data.properties_mutable,
            royalty_mutable: token_data.royalty_mutable,
            default_properties: serde_json::to_value(&token_data.default_properties).unwrap(),
            inserted_at: chrono::Utc::now().naive_utc(),
        }
    }
}

#[derive(Debug, FieldCount, Identifiable, Insertable, Queryable, Serialize)]
#[primary_key(
    creator_address,
    collection_name,
    name,
    property_version,
    transaction_version,
    table_handle
)]
#[diesel(table_name = "token_ownership")]
pub struct TokenOwnership {
    pub creator_address: String,
    pub collection_name: String,
    pub name: String,
    pub property_version: bigdecimal::BigDecimal,
    pub transaction_version: i64,
    pub owner_address: Option<String>,
    pub amount: bigdecimal::BigDecimal,
    pub table_handle: String,
    pub table_type: Option<String>,
    // Default time columns
    pub inserted_at: chrono::NaiveDateTime,
}

impl TokenOwnership {
    fn from_token(token: &TokenPB) -> Self {
        let token_data_id = token
            .token_id
            .as_ref()
            .unwrap()
            .token_data_id
            .as_ref()
            .unwrap()
            .clone();
        Self {
            creator_address: token_data_id.creator_address,
            collection_name: token_data_id.collection_name,
            name: token_data_id.name,
            property_version: u64_to_bigdecimal(token.token_id.as_ref().unwrap().property_version),
            transaction_version: token.transaction_version as i64,
            owner_address: token.owner_address.as_ref().cloned(),
            amount: u64_to_bigdecimal(token.amount),
            table_handle: token.table_handle.clone(),
            table_type: token.table_type.as_ref().cloned(),
            inserted_at: chrono::Utc::now().naive_utc(),
        }
    }
}

#[derive(Debug, FieldCount, Identifiable, Insertable, Queryable, Serialize)]
#[primary_key(creator_address, collection_name, transaction_version)]
#[diesel(table_name = "collection_datas")]
pub struct CollectionData {
    pub creator_address: String,
    pub collection_name: String,
    pub description: String,
    pub transaction_version: i64,
    pub metadata_uri: String,
    pub supply: bigdecimal::BigDecimal,
    pub maximum: bigdecimal::BigDecimal,
    pub maximum_mutable: bool,
    pub uri_mutable: bool,
    pub description_mutable: bool,
    // Default time columns
    pub inserted_at: chrono::NaiveDateTime,
}

impl CollectionData {
    fn from_collection_data(collection_data: &CollectionDataPB) -> Self {
        Self {
            creator_address: collection_data.creator_address.clone(),
            collection_name: collection_data.collection_name.clone(),
            description: collection_data.description.clone(),
            transaction_version: collection_data.transaction_version as i64,
            metadata_uri: collection_data.metadata_uri.clone(),
            supply: u64_to_bigdecimal(collection_data.supply),
            maximum: u64_to_bigdecimal(collection_data.maximum),
            maximum_mutable: collection_data.maximum_mutable,
            uri_mutable: collection_data.uri_mutable,
            description_mutable: collection_data.description_mutable,
            inserted_at: chrono::Utc::now().naive_utc(),
        }
    }
}
