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
};
use aptos_indexer_grpc_server_framework::RunnableConfig;
use axum::Router;
use diesel::{
    r2d2::{ConnectionManager, Pool},
    PgConnection,
};
use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};
use tracing::info;

/// Trait for building a router for axum
#[enum_dispatch]
pub trait Server: Send + Sync {
    fn build_router(&self) -> Router;
}

/// Required account data and auth keys for Cloudflare
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AssetUploaderConfig {
    /// Cloudflare API key
    pub cloudflare_auth_key: String,
    /// Cloudflare Account ID provided at the images home page used to authenticate requests
    pub cloudflare_account_id: String,
    /// Cloudflare Account Hash provided at the images home page used for generating the CDN image URLs
    pub cloudflare_account_hash: String,
    /// Cloudflare Image Delivery URL prefix provided at the images home page used for generating the CDN image URLs
    pub cloudflare_image_delivery_prefix: String,
    /// In addition to on the fly transformations, Cloudflare images can be returned in preset variants. This is the default variant used with the saved CDN image URLs.
    pub cloudflare_default_variant: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ParserConfig {
    pub google_application_credentials: Option<String>,
    pub bucket: String,
    pub cdn_prefix: String,
    pub ipfs_prefix: String,
    pub ipfs_auth_key: Option<String>,
    #[serde(default = "NFTMetadataCrawlerConfig::default_max_file_size_bytes")]
    pub max_file_size_bytes: u32,
    #[serde(default = "NFTMetadataCrawlerConfig::default_image_quality")]
    pub image_quality: u8, // Quality up to 100
    #[serde(default = "NFTMetadataCrawlerConfig::default_max_image_dimensions")]
    pub max_image_dimensions: u32,
    #[serde(default)]
    pub ack_parsed_uris: bool,
    #[serde(default)]
    pub uri_blacklist: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerConfig {
    Parser(ParserConfig),
    AssetUploader(AssetUploaderConfig),
}

/// Structs to hold config from YAML
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NFTMetadataCrawlerConfig {
    pub database_url: String,
    #[serde(default = "NFTMetadataCrawlerConfig::default_max_num_parse_retries")]
    pub max_num_parse_retries: i32,
    pub server_port: u16,
    pub server_config: ServerConfig,
}

impl NFTMetadataCrawlerConfig {
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
}

#[derive(Clone)]
#[enum_dispatch(Server)]
pub enum ServerContext {
    Parser(ParserContext),
    AssetUploader(AssetUploaderContext),
}

impl ServerConfig {
    pub async fn build_context(
        &self,
        pool: Pool<ConnectionManager<PgConnection>>,
        max_num_retries: i32,
    ) -> ServerContext {
        match self {
            ServerConfig::Parser(parser_config) => ServerContext::Parser(
                ParserContext::new(parser_config.clone(), pool, max_num_retries).await,
            ),
            ServerConfig::AssetUploader(asset_uploader_config) => ServerContext::AssetUploader(
                AssetUploaderContext::new(asset_uploader_config.clone(), pool),
            ),
        }
    }
}

#[async_trait::async_trait]
impl RunnableConfig for NFTMetadataCrawlerConfig {
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
        let context = self
            .server_config
            .build_context(pool, self.max_num_parse_retries)
            .await;
        let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", self.server_port))
            .await
            .expect("Failed to bind TCP listener");
        axum::serve(listener, context.build_router()).await.unwrap();

        Ok(())
    }

    fn get_server_name(&self) -> String {
        "parser".to_string()
    }
}
