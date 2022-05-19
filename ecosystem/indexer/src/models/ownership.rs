// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::schema::ownerships;
use serde::Serialize;

#[derive(Associations, Debug, Identifiable, Insertable, Queryable, Serialize, Clone)]
#[diesel(table_name = "ownerships")]
#[primary_key(token_id, owner)]
pub struct Ownership {
    pub token_id: String,
    pub owner: String,
    pub amount: i64,
    pub updated_at: chrono::NaiveDateTime,
    pub inserted_at: chrono::NaiveDateTime,
}
