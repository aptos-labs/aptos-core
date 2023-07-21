// Copyright © Aptos Foundation

use aptos_indexer_grpc_server_framework::{RunnableConfig, ServerArgs};
use serde::{Deserialize, Serialize};

/// Structs to hold config from YAML
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ParserConfig {
    pub google_application_credentials: String,
    pub bucket: String,
    pub subscription_name: String,
    pub database_url: String,
    pub cdn_prefix: String,
    pub ipfs_prefix: String,
}

#[async_trait::async_trait]
impl RunnableConfig for ParserConfig {
    /// Main driver function
    async fn run(&self) -> anyhow::Result<()> {
        todo!();
    }

    fn get_server_name(&self) -> String {
        "parser".to_string()
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = <ServerArgs as clap::Parser>::parse();
    args.run::<ParserConfig>().await
}
