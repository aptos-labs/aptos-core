// Copyright © Aptos Foundation

use crate::{
    models::{
        nft_metadata_crawler_uris::NFTMetadataCrawlerURIs,
        nft_metadata_crawler_uris_query::NFTMetadataCrawlerURIsQuery,
    },
    utils::{
        gcs::{write_image_to_gcs, write_json_to_gcs},
        image_optimizer::ImageOptimizer,
        json_parser::JSONParser,
        uri_parser::URIParser,
    },
};
use aptos_indexer_grpc_server_framework::RunnableConfig;
use diesel::{
    r2d2::{ConnectionManager, PooledConnection},
    PgConnection,
};
use image::ImageFormat;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{error, info};

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
    pub num_parsers: usize,
    pub image_quality: u8, // Quality up to 100
}

#[async_trait::async_trait]
impl RunnableConfig for ParserConfig {
    /// Main driver function that establishes a connection to Pubsub and parses the Pubsub entries in parallel
    async fn run(&self) -> anyhow::Result<()> {
        todo!();
    }

    fn get_server_name(&self) -> String {
        "parser".to_string()
    }
}

/// Stuct that represents a parser for a single entry from queue
#[allow(dead_code)] // Will remove when functions are implemented
pub struct Worker {
    config: ParserConfig,
    conn: PooledConnection<ConnectionManager<PgConnection>>,
    model: NFTMetadataCrawlerURIs,
    token_data_id: String,
    token_uri: String,
    last_transaction_version: i32,
    last_transaction_timestamp: chrono::NaiveDateTime,
    force: bool,
}

impl Worker {
    pub fn new(
        config: ParserConfig,
        conn: PooledConnection<ConnectionManager<PgConnection>>,
        token_data_id: String,
        token_uri: String,
        last_transaction_version: i32,
        last_transaction_timestamp: chrono::NaiveDateTime,
        force: bool,
    ) -> Self {
        Self {
            config,
            conn,
            model: NFTMetadataCrawlerURIs::new(token_uri.clone()),
            token_data_id,
            token_uri,
            last_transaction_version,
            last_transaction_timestamp,
            force,
        }
    }

    /// Main parsing flow
    pub async fn parse(&mut self) -> anyhow::Result<()> {
        info!(
            last_transaction_version = self.last_transaction_version,
            "[NFT Metadata Crawler] Starting parser"
        );

        // Deduplicate token_uri
        // Proceed if force or if token_uri has not been parsed
        if self.force
            || NFTMetadataCrawlerURIsQuery::get_by_token_uri(
                self.token_uri.clone(),
                &mut self.conn,
            )?
            .is_none()
        {
            // Parse token_uri
            self.model.set_token_uri(self.token_uri.clone());
            let token_uri = self.model.get_token_uri();
            let json_uri = URIParser::parse(token_uri.clone()).unwrap_or(token_uri);

            // Parse JSON for raw_image_uri and raw_animation_uri
            let (raw_image_uri, raw_animation_uri, json) =
                JSONParser::parse(json_uri).await.unwrap_or_else(|e| {
                    // Increment retry count if JSON parsing fails
                    error!(
                        last_transaction_version = self.last_transaction_version,
                        error = ?e,
                        "[NFT Metadata Crawler] JSON parse failed",
                    );
                    self.model.increment_json_parser_retry_count();
                    (None, None, Value::Null)
                });

            self.model.set_raw_image_uri(raw_image_uri);
            self.model.set_raw_animation_uri(raw_animation_uri);

            // Save parsed JSON to GCS
            if json != Value::Null {
                let cdn_json_uri =
                    write_json_to_gcs(self.config.bucket.clone(), self.token_data_id.clone(), json)
                        .await
                        .ok();
                self.model.set_cdn_json_uri(cdn_json_uri);
            }

            // Commit model to Postgres
            self.commit_to_postgres().await;
        }

        // Deduplicate raw_image_uri
        // Proceed with image optimization of force or if raw_image_uri has not been parsed
        if self.force
            || self.model.get_raw_image_uri().map_or(true, |uri_option| {
                NFTMetadataCrawlerURIsQuery::get_by_raw_image_uri(uri_option, &mut self.conn)
                    .map_or(true, |uri| uri.is_none())
            })
        {
            // Parse raw_image_uri, use token_uri if parsing fails
            let raw_image_uri = self
                .model
                .get_raw_image_uri()
                .unwrap_or(self.model.get_token_uri());
            let img_uri = URIParser::parse(raw_image_uri).unwrap_or(self.model.get_token_uri());

            // Resize and optimize image and animation
            let (image, format) = ImageOptimizer::optimize(img_uri).await.unwrap_or_else(|e| {
                // Increment retry count if image is None
                error!(
                    last_transaction_version = self.last_transaction_version,
                    error = ?e,
                    "[NFT Metadata Crawler] Image optimization failed"
                );
                self.model.increment_image_optimizer_retry_count();
                (vec![], ImageFormat::Png)
            });

            if !image.is_empty() {
                // Save resized and optimized image to GCS
                let cdn_image_uri = write_image_to_gcs(
                    format,
                    self.config.bucket.clone(),
                    self.token_data_id.clone(),
                    image,
                )
                .await
                .ok();
                self.model.set_cdn_image_uri(cdn_image_uri);
            }

            // Commit model to Postgres
            self.commit_to_postgres().await;
        }

        // Deduplicate raw_animation_uri
        // Set raw_animation_uri_option to None if not force and raw_animation_uri already exists
        let mut raw_animation_uri_option = self.model.get_raw_animation_uri();
        if !self.force
            && raw_animation_uri_option.clone().map_or(true, |uri| {
                NFTMetadataCrawlerURIsQuery::get_by_raw_animation_uri(uri, &mut self.conn)
                    .unwrap_or(None)
                    .is_some()
            })
        {
            raw_animation_uri_option = None;
        }

        // If raw_animation_uri_option is None, skip
        if let Some(raw_animation_uri) = raw_animation_uri_option {
            let animation_uri =
                URIParser::parse(raw_animation_uri.clone()).unwrap_or(raw_animation_uri);

            // Resize and optimize animation
            let (animation, format) = ImageOptimizer::optimize(animation_uri)
                .await
                .unwrap_or_else(|e| {
                    // Increment retry count if animation is None
                    error!(
                        last_transaction_version = self.last_transaction_version,
                        error = ?e,
                        "[NFT Metadata Crawler] Animation optimization failed"
                    );
                    self.model.increment_animation_optimizer_retry_count();
                    (vec![], ImageFormat::Png)
                });

            // Save resized and optimized animation to GCS
            if !animation.is_empty() {
                let cdn_animation_uri = write_image_to_gcs(
                    format,
                    self.config.bucket.clone(),
                    self.token_data_id.clone(),
                    animation,
                )
                .await
                .ok();
                self.model.set_cdn_animation_uri(cdn_animation_uri);
            }

            // Commit model to Postgres
            self.commit_to_postgres().await;
        }

        Ok(())
    }

    /// Calls and handles error for upserting to Postgres
    async fn commit_to_postgres(&mut self) {
        todo!();
    }
}
