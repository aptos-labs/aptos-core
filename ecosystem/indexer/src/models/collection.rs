// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::schema::collections;
use serde::Serialize;

#[derive(Associations, Debug, Identifiable, Insertable, Queryable, Serialize, Clone)]
#[diesel(table_name = "collections")]
#[primary_key(creator, name)]
pub struct Collection {
    pub creator: String,
    pub name: String,
    pub description: String,
    pub max_amount: Option<i64>,
    pub uri: String,
    pub created_at: chrono::NaiveDateTime,
    pub inserted_at: chrono::NaiveDateTime,
}
