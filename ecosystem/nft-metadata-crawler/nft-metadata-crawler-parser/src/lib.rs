// Copyright © Aptos Foundation

pub mod db;
pub mod models;
pub mod parser;
pub mod schema;

use diesel::{
    pg::PgConnection,
    r2d2::{ConnectionManager, Pool},
};

pub fn establish_connection_pool(database_url: String) -> Pool<ConnectionManager<PgConnection>> {
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    Pool::builder()
        .build(manager)
        .expect("Failed to create pool.")
}
