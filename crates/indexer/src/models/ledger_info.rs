// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::extra_unused_lifetimes)]
use crate::schema::ledger_infos;

#[derive(Debug, Identifiable, Insertable, Queryable)]
#[diesel(table_name = ledger_infos)]
#[diesel(primary_key(chain_id))]
pub struct LedgerInfo {
    pub chain_id: i64,
}
