// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::extra_unused_lifetimes)]
use crate::schema::token_propertys;
use serde::Serialize;

#[derive(Associations, Debug, Identifiable, Insertable, Queryable, Serialize, Clone)]
#[diesel(table_name = "token_propertys")]
#[primary_key(token_id)]
pub struct TokenProperty {
    pub token_id: String,
    pub previous_token_id: String,
    pub property_keys: String,
    pub property_values: String,
    pub property_types: String,
    pub updated_at: chrono::NaiveDateTime,
    pub inserted_at: chrono::NaiveDateTime,
}

impl TokenProperty {
    pub fn new(
        token_id: String,
        previous_token_id: String,
        property_keys: String,
        property_values: String,
        property_types: String,
        updated_at: chrono::NaiveDateTime,
        inserted_at: chrono::NaiveDateTime,
    ) -> Self {
        TokenProperty {
            token_id,
            previous_token_id,
            property_keys,
            property_values,
            property_types,
            updated_at,
            inserted_at,
        }
    }
}
