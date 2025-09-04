// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::extra_unused_lifetimes)]
use crate::{database::PgPoolConnection, schema::processor_status};
use diesel::{ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl};

#[derive(AsChangeset, Debug, Insertable)]
#[diesel(table_name = processor_status)]
/// Only tracking the latest version successfully processed
pub struct ProcessorStatusV2 {
    pub processor: String,
    pub last_success_version: i64,
}

#[derive(AsChangeset, Debug, Queryable)]
#[diesel(table_name = processor_status)]
/// Only tracking the latest version successfully processed
pub struct ProcessorStatusV2Query {
    pub processor: String,
    pub last_success_version: i64,
    pub last_updated: chrono::NaiveDateTime,
}

impl ProcessorStatusV2Query {
    pub fn get_by_processor(
        processor_name: &String,
        conn: &mut PgPoolConnection,
    ) -> diesel::QueryResult<Option<Self>> {
        processor_status::table
            .filter(processor_status::processor.eq(processor_name))
            .first::<Self>(conn)
            .optional()
    }
}
