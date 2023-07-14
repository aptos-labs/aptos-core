// Copyright © Aptos Foundation

use google_cloud_auth::project::{create_token_source, Config};
use nft_metadata_crawler_parser::{establish_connection_pool, parser::Parser};
use nft_metadata_crawler_utils::NFTMetadataCrawlerEntry;
use tracing::info;

#[tokio::main]
async fn main() {
    info!("Starting Parser");
    let pool = establish_connection_pool();
    let conn = pool.get().unwrap();

    // Temporary to test compilation
    let (entry, force) = NFTMetadataCrawlerEntry::new("test,csv".to_string()).unwrap();
    let ts = create_token_source(Config {
        audience: None,
        scopes: Some(&["https://www.googleapis.com/auth/cloud-platform"]),
        sub: None,
    })
    .await
    .expect("No token source");

    let _parser = Parser::new(entry, "test_bucket".to_string(), force, ts.as_ref(), conn);
}
