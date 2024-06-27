// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    asset_uploader::AssetUploaderContext,
    parser::ParserContext,
    utils::{
        constants::{
            DEFAULT_IMAGE_QUALITY, DEFAULT_MAX_FILE_SIZE_BYTES, DEFAULT_MAX_IMAGE_DIMENSIONS,
            DEFAULT_MAX_NUM_PARSE_RETRIES,
        },
        database::{establish_connection_pool, run_migrations},
    },
    Server, ServerType,
};
use aptos_indexer_grpc_server_framework::RunnableConfig;
use axum::{routing::post, Router};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc};
use tracing::info;

/// Structs to hold config from YAML
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ParserConfig {
    pub google_application_credentials: Option<String>,
    pub bucket: String,
    pub database_url: String,
    pub cdn_prefix: String,
    pub ipfs_prefix: String,
    pub ipfs_auth_key: Option<String>,
    #[serde(default = "ParserConfig::default_max_file_size_bytes")]
    pub max_file_size_bytes: u32,
    #[serde(default = "ParserConfig::default_image_quality")]
    pub image_quality: u8, // Quality up to 100
    #[serde(default = "ParserConfig::default_max_image_dimensions")]
    pub max_image_dimensions: u32,
    #[serde(default = "ParserConfig::default_max_num_parse_retries")]
    pub max_num_parse_retries: i32,
    #[serde(default)]
    pub ack_parsed_uris: bool,
    #[serde(default)]
    pub uri_blacklist: Vec<String>,
    pub server_port: u16,
    #[serde(default = "ParserConfig::default_server_type")]
    pub server_type: ServerType,
    pub cloudflare_auth_key: Option<String>,
}

impl ParserConfig {
    pub const fn default_max_file_size_bytes() -> u32 {
        DEFAULT_MAX_FILE_SIZE_BYTES
    }

    pub const fn default_image_quality() -> u8 {
        DEFAULT_IMAGE_QUALITY
    }

    pub const fn default_max_image_dimensions() -> u32 {
        DEFAULT_MAX_IMAGE_DIMENSIONS
    }

    pub const fn default_max_num_parse_retries() -> i32 {
        DEFAULT_MAX_NUM_PARSE_RETRIES
    }

    pub const fn default_server_type() -> ServerType {
        ServerType::Parser
    }
}

#[async_trait::async_trait]
impl RunnableConfig for ParserConfig {
    /// Main driver function that establishes a connection to Pubsub and parses the Pubsub entries in parallel
    async fn run(&self) -> anyhow::Result<()> {
        info!(
            "[NFT Metadata Crawler] Starting parser with config: {:?}",
            self
        );

        info!("[NFT Metadata Crawler] Connecting to database");
        let pool = establish_connection_pool(&self.database_url);
        info!("[NFT Metadata Crawler] Database connection successful");

        info!("[NFT Metadata Crawler] Running migrations");
        run_migrations(&pool);
        info!("[NFT Metadata Crawler] Finished migrations");

        // Create request context
        // Dynamic dispatch over different variations of servers
        let config = Arc::new(self.clone());
        let context: Arc<dyn Server> = match self.server_type {
            ServerType::Parser => Arc::new(ParserContext::new(config, pool).await),
            ServerType::AssetUploader => Arc::new(AssetUploaderContext::new(config, pool).await),
        };

        // Create web server
        let router = Router::new().route(
            "/",
            post(|bytes| async move { context.handle_request(bytes).await }),
        );

        let addr = SocketAddr::from(([0, 0, 0, 0], self.server_port));
        axum::Server::bind(&addr)
            .serve(router.into_make_service())
            .await
            .unwrap();
        Ok(())
    }

    fn get_server_name(&self) -> String {
        "parser".to_string()
    }
}
