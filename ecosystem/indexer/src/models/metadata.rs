// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::schema::metadatas;
use serde::{Deserialize, Serialize};

#[derive(Associations, Debug, Identifiable, Insertable, Queryable, Serialize, Clone)]
#[diesel(table_name = "metadata")]
#[primary_key(token_id)]
pub struct Metadata {
    pub token_id: String,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub seller_fee_basis_points: Option<i64>,
    pub description: Option<String>,
    pub image: String,
    pub external_url: Option<String>,
    pub animation_url: Option<String>,
    pub attributes: Option<serde_json::Value>,
    pub properties: Option<serde_json::Value>,
    pub last_updated_at: chrono::NaiveDateTime,
    pub inserted_at: chrono::NaiveDateTime,
}

impl Metadata {
    pub fn from_token_uri_meta(token_uri: TokenMetaFromURI, token_id_str: String) -> Option<Self> {
        if token_uri.image.is_some() {
            Some(Self {
                token_id: token_id_str,
                name: token_uri.name,
                symbol: token_uri.symbol,
                seller_fee_basis_points: token_uri.seller_fee_basis_points,
                description: token_uri.description,
                image: token_uri.image.unwrap(),
                external_url: token_uri.external_url,
                animation_url: token_uri.animation_url,
                attributes: token_uri.attributes,
                properties: token_uri.properties,
                last_updated_at: chrono::Utc::now().naive_utc(),
                inserted_at: chrono::Utc::now().naive_utc(),
            })
        } else {
            None
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TokenMetaFromURI {
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub seller_fee_basis_points: Option<i64>,
    pub description: Option<String>,
    pub image: Option<String>,
    pub external_url: Option<String>,
    pub animation_url: Option<String>,
    pub attributes: Option<serde_json::Value>,
    pub properties: Option<serde_json::Value>,
}
