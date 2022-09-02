// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::extra_unused_lifetimes)]
use crate::{schema::collections, util::u64_to_bigdecimal};
use serde::Serialize;

#[derive(Associations, Debug, Identifiable, Insertable, Queryable, Serialize, Clone)]
#[diesel(table_name = "collections")]
#[primary_key(collection_id)]
pub struct Collection {
    pub collection_id: String,
    pub creator: String,
    pub name: String,
    pub description: String,
    pub max_amount: bigdecimal::BigDecimal,
    pub uri: String,
    pub created_at: chrono::NaiveDateTime,
    pub inserted_at: chrono::NaiveDateTime,
}

impl Collection {
    pub fn new(
        creator: String,
        name: String,
        description: String,
        max_amount: u64,
        uri: String,
        created_at: chrono::NaiveDateTime,
        inserted_at: chrono::NaiveDateTime,
    ) -> Self {
        let collection_id = format!("{}::{}", creator, name);
        Collection {
            collection_id,
            creator,
            name,
            description,
            max_amount: u64_to_bigdecimal(max_amount),
            uri,
            created_at,
            inserted_at,
        }
    }
}
