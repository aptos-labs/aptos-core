// Copyright © Aptos Foundation

use diesel::{
    r2d2::{ConnectionManager, Pool},
    PgConnection,
};
use nft_metadata_crawler_parser::establish_connection_pool;

#[tokio::main]
async fn main() {
    println!("Starting parser");
    let _pool: Pool<ConnectionManager<PgConnection>> = establish_connection_pool();
}
