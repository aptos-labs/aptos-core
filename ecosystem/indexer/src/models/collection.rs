// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::schema::collections;
use serde::Serialize;

#[derive(Associations, Debug, Identifiable, Insertable, Queryable, Serialize, Clone)]
#[diesel(table_name = "collections")]
#[primary_key(collection_id)]
pub struct Collection {
    pub collection_id: String,
    pub creator: String,
    pub name: String,
    pub description: String,
    pub max_amount: Option<i64>,
    pub uri: String,
    pub created_at: chrono::NaiveDateTime,
    pub inserted_at: chrono::NaiveDateTime,
}

impl Collection {
    pub fn new(
        creator: String,
        name: String,
        description: String,
        max_amount: Option<i64>,
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
            max_amount,
            uri,
            created_at,
            inserted_at,
        }
    }
}
