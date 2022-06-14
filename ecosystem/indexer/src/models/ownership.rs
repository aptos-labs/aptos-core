// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::schema::ownerships;
use serde::Serialize;

#[derive(Associations, Debug, Identifiable, Insertable, Queryable, Serialize, Clone)]
#[diesel(table_name = "ownerships")]
#[primary_key(ownership_id)]
pub struct Ownership {
    pub ownership_id: String,
    pub token_id: String,
    pub owner: String,
    pub amount: i64,
    pub updated_at: chrono::NaiveDateTime,
    pub inserted_at: chrono::NaiveDateTime,
}

impl Ownership {
    pub fn new(
        token_id: String,
        owner: String,
        amount: i64,
        updated_at: chrono::NaiveDateTime,
        inserted_at: chrono::NaiveDateTime,
    ) -> Self {
        let ownership_id = format!("{}::{}", token_id, owner);
        Ownership {
            ownership_id,
            token_id,
            owner,
            amount,
            updated_at,
            inserted_at,
        }
    }
}
