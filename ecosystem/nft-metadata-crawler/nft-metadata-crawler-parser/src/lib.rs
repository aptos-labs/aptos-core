// Copyright © Aptos Foundation

pub mod models;
pub mod parser;
pub mod schema;
pub mod utils;

use diesel::{
    pg::PgConnection,
    r2d2::{ConnectionManager, Pool},
};

/**
 * Establishes a connection pool to Postgres
 */
pub fn establish_connection_pool(database_url: String) -> Pool<ConnectionManager<PgConnection>> {
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    Pool::builder()
        .build(manager)
        .expect("Failed to create pool.")
}
