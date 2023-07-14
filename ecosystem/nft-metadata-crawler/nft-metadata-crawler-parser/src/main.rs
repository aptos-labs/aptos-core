// Copyright © Aptos Foundation

use google_cloud_auth::project::{create_token_source, Config};
use nft_metadata_crawler_parser::{establish_connection_pool, parser::Parser};
use nft_metadata_crawler_utils::{load_config_from_yaml, NFTMetadataCrawlerEntry};
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(clap::Parser)]
pub struct ServerArgs {
    #[clap(short, long, value_parser)]
    pub config_path: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ParserConfig {
    pub google_application_credentials: String,
    pub bucket: String,
    pub subscription_name: String,
    pub database_url: String,
}

#[tokio::main]
async fn main() {
    info!("Starting Parser");
    let args = <ServerArgs as clap::Parser>::parse();
    let config =
        load_config_from_yaml::<ParserConfig>(args.config_path).expect("Unable to load config");
    let pool = establish_connection_pool(config.database_url.clone());
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
