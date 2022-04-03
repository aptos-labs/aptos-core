// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! Database-related functions
use std::sync::Arc;

use diesel::{
    pg::PgConnection,
    r2d2::{ConnectionManager, PoolError, PooledConnection},
    RunQueryDsl,
};

pub type PgPool = diesel::r2d2::Pool<ConnectionManager<PgConnection>>;
pub type PgDbPool = Arc<PgPool>;
pub type PgPoolConnection = PooledConnection<ConnectionManager<PgConnection>>;

pub fn new_db_pool(database_url: &str) -> Result<PgDbPool, PoolError> {
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    PgPool::builder().build(manager).map(Arc::new)
}

pub fn execute_with_better_error<
    T: diesel::Table + diesel::QuerySource,
    U: diesel::query_builder::QueryFragment<diesel::pg::Pg>
        + diesel::insertable::CanInsertInSingleQuery<diesel::pg::Pg>,
>(
    conn: &PgPoolConnection,
    query: diesel::query_builder::InsertStatement<T, U>,
) -> diesel::QueryResult<usize>
where
    <T as diesel::QuerySource>::FromClause: diesel::query_builder::QueryFragment<diesel::pg::Pg>,
{
    let debug = diesel::debug_query::<diesel::pg::Pg, _>(&query).to_string();
    aptos_logger::debug!("Executing query: {:?}", debug);
    let res = query.execute(conn);
    if res.is_err() {
        let e = res.as_ref().err().unwrap();
        aptos_logger::warn!("Error running query: {:?}\n{}", e, debug);
    }
    res
}
