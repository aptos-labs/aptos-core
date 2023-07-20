// Copyright © Aptos Foundation

use crate::{
    models::{
        nft_metadata_crawler_uris::NFTMetadataCrawlerURIs,
        nft_metadata_crawler_uris_query::NFTMetadataCrawlerURIsQuery,
    },
    utils::{image_optimizer::ImageOptimizer, json_parser::JSONParser, uri_parser::URIParser},
};
use diesel::{
    r2d2::{ConnectionManager, PooledConnection},
    PgConnection,
};
use image::ImageFormat;
use nft_metadata_crawler_utils::NFTMetadataCrawlerEntry;
use serde_json::Value;
use tracing::{error, info};

/**
 * Stuct that represents a parser for a single entry from queue
 */
#[allow(dead_code)]
pub struct Parser {
    entry: NFTMetadataCrawlerEntry,
    model: NFTMetadataCrawlerURIs,
    bucket: String,
    token: String,
    conn: PooledConnection<ConnectionManager<PgConnection>>,
    cdn_prefix: String,
}

impl Parser {
    pub fn new(
        entry: NFTMetadataCrawlerEntry,
        bucket: String,
        token: String,
        conn: PooledConnection<ConnectionManager<PgConnection>>,
        cdn_prefix: String,
    ) -> Self {
        Self {
            model: NFTMetadataCrawlerURIs::new(entry.token_uri.clone()),
            entry,
            bucket,
            token,
            conn,
            cdn_prefix,
        }
    }

    /**
     * Main parsing flow
     */
    pub async fn parse(&mut self) -> anyhow::Result<()> {
        // Deduplicate token_uri
        // Skip if token_uri already exists and not force
        if self.entry.force
            || NFTMetadataCrawlerURIsQuery::get_by_token_uri(
                self.entry.token_uri.clone(),
                &mut self.conn,
            )?
            .is_none()
        {
            self.log_info("Starting JSON parse");

            // Parse token_uri
            self.log_info("Parsing token_uri for IPFS URI");
            self.model.set_token_uri(self.entry.token_uri.clone());
            let json_uri = URIParser::parse(self.model.get_token_uri());

            // Parse JSON for raw_image_uri and raw_animation_uri
            self.log_info("Parsing JSON");
            let (raw_image_uri, raw_animation_uri, json) = JSONParser::parse(json_uri).await;
            self.model.set_raw_image_uri(raw_image_uri);
            self.model.set_raw_animation_uri(raw_animation_uri);

            // Increment retry count if JSON is None
            if json.is_none() {
                self.log_error("JSON parse failed");
                self.model.increment_json_parser_retry_count()
            }

            // Save parsed JSON to GCS
            self.log_info("Writing JSON to GCS");
            let cdn_json_uri = self.handle_write_json_to_gcs(json).await;
            self.model.set_cdn_json_uri(cdn_json_uri);

            // Commit model to Postgres
            self.log_info("Committing JSON parse to Postgres");
            self.commit_to_postgres().await;
        } else {
            self.log_info("Duplicate token_uri, skipping URI parse");
        }

        // Deduplicate raw_image_uri
        // Skip if raw_image_uri has already been parsed and not force
        if self.entry.force
            || NFTMetadataCrawlerURIsQuery::get_by_raw_image_uri(
                self.model.get_raw_image_uri(),
                &mut self.conn,
            )?
            .is_none()
        {
            self.log_info("Starting image optimization");

            // Parse raw_image_uri, use token_uri if parsing fails
            self.log_info("Parsing raw_image_uri for IPFS");
            let img_uri = URIParser::parse(match self.model.get_raw_image_uri() {
                Some(uri) => uri,
                None => self.model.get_token_uri(),
            });

            // Resize and optimize image and animation
            self.log_info("Optimizing image");
            let image = ImageOptimizer::optimize(Some(img_uri)).await;

            // Increment retry count if image is None
            if image.is_none() {
                self.log_error("Image optimization failed");
                self.model.increment_image_optimizer_retry_count()
            }

            // Save resized and optimized image to GCS
            self.log_info("Writing image to GCS");
            let cdn_image_uri = self.handle_write_image_to_gcs(image).await;
            self.model.set_cdn_image_uri(cdn_image_uri);

            // Commit model to Postgres
            self.log_info("Committing image optimization to Postgres");
            self.commit_to_postgres().await;
        } else {
            self.log_info("Duplicate raw_image_uri, skipping image optimization");
        }

        // Deduplicate raw_animation_uri
        // Skip if raw_animation_uri has already been parsed and not force
        if self.entry.force
            || NFTMetadataCrawlerURIsQuery::get_by_raw_animation_uri(
                self.model.get_raw_animation_uri(),
                &mut self.conn,
            )?
            .is_none()
        {
            self.log_info("Starting animation optimization");

            // Parse raw_animation_uri, use None if parsing fails
            self.log_info("Parsing raw_animation_uri for IPFS");
            let animation_uri = match self.model.get_raw_animation_uri() {
                Some(uri) => Some(URIParser::parse(uri)),
                None => None,
            };

            // Resize and optimize animation
            self.log_info("Optimizing animation");
            let animation = ImageOptimizer::optimize(animation_uri).await;

            // Increment retry count if animation is None
            if animation.is_none() {
                self.log_error("Animation optimization failed");
                self.model.increment_animation_optimizer_retry_count()
            }

            // Save resized and optimized animation to GCS
            self.log_info("Writing animation to GCS");
            let cdn_animation_uri = self.handle_write_image_to_gcs(animation).await;
            self.model.set_cdn_animation_uri(cdn_animation_uri);

            // Commit model to Postgres
            self.log_info("Committing animation optimization to Postgres");
            self.commit_to_postgres().await;
        } else {
            self.log_info("Duplicate raw_animation_uri, skipping animation optimization");
        }

        Ok(())
    }

    /**
     * Calls and handles error for writing JSON to GCS
     */
    async fn handle_write_json_to_gcs(&mut self, _json: Option<Value>) -> Option<String> {
        todo!();
    }

    /**
     * Calls and handles error for writing image to GCS
     */
    async fn handle_write_image_to_gcs(
        &mut self,
        _image: Option<(Vec<u8>, ImageFormat)>,
    ) -> Option<String> {
        todo!();
    }

    /**
     * Calls and handles error for upserting to Postgres
     */
    async fn commit_to_postgres(&mut self) {
        todo!();
    }

    /**
     * Logs info with last_transaction_version
     */
    fn log_info(&self, msg: &str) {
        info!(
            last_transaction_version = self.entry.last_transaction_version,
            msg
        );
    }

    /**
     * Logs error with last_transaction_version
     */
    fn log_error(&self, msg: &str) {
        error!(
            last_transaction_version = self.entry.last_transaction_version,
            msg
        );
    }
}
