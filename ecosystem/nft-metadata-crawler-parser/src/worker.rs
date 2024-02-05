// Copyright © Aptos Foundation

use crate::{
    config::ParserConfig,
    models::{
        nft_metadata_crawler_uris::NFTMetadataCrawlerURIs,
        nft_metadata_crawler_uris_query::NFTMetadataCrawlerURIsQuery,
    },
    utils::{
        constants::{
            DEFAULT_IMAGE_QUALITY, DEFAULT_MAX_FILE_SIZE_BYTES, DEFAULT_MAX_IMAGE_DIMENSIONS,
        },
        counters::{
            DUPLICATE_ASSET_URI_COUNT, DUPLICATE_RAW_ANIMATION_URI_COUNT,
            DUPLICATE_RAW_IMAGE_URI_COUNT, OPTIMIZE_IMAGE_TYPE_COUNT, PARSER_SUCCESSES_COUNT,
            PARSE_URI_TYPE_COUNT, SKIP_URI_COUNT,
        },
        database::upsert_uris,
        gcs::{write_image_to_gcs, write_json_to_gcs},
        image_optimizer::ImageOptimizer,
        json_parser::JSONParser,
        uri_parser::URIParser,
    },
};
use diesel::{
    r2d2::{ConnectionManager, PooledConnection},
    PgConnection,
};
use google_cloud_storage::client::Client as GCSClient;
use image::ImageFormat;
use serde_json::Value;
use tracing::{error, info, warn};
use url::Url;

/// Stuct that represents a parser for a single entry from queue
pub struct Worker {
    config: ParserConfig,
    conn: PooledConnection<ConnectionManager<PgConnection>>,
    gcs_client: GCSClient,
    pubsub_message: String,
    model: NFTMetadataCrawlerURIs,
    asset_data_id: String,
    asset_uri: String,
    last_transaction_version: i32,
    last_transaction_timestamp: chrono::NaiveDateTime,
    force: bool,
}

impl Worker {
    pub fn new(
        config: ParserConfig,
        conn: PooledConnection<ConnectionManager<PgConnection>>,
        gcs_client: GCSClient,
        pubsub_message: String,
        asset_data_id: String,
        asset_uri: String,
        last_transaction_version: i32,
        last_transaction_timestamp: chrono::NaiveDateTime,
        force: bool,
    ) -> Self {
        let worker = Self {
            config,
            conn,
            gcs_client,
            pubsub_message,
            model: NFTMetadataCrawlerURIs::new(asset_uri.clone()),
            asset_data_id,
            asset_uri,
            last_transaction_version,
            last_transaction_timestamp,
            force,
        };
        worker.log_info("Created worker");
        worker
    }

    /// Main parsing flow
    pub async fn parse(&mut self) -> anyhow::Result<()> {
        // Deduplicate asset_uri
        // Exit if not force or if asset_uri has already been parsed
        let prev_model =
            NFTMetadataCrawlerURIsQuery::get_by_asset_uri(self.asset_uri.clone(), &mut self.conn);
        if !self.force && prev_model.is_some() {
            self.log_info("Duplicate asset_uri found, skipping parse");
            DUPLICATE_ASSET_URI_COUNT.inc();
            return Ok(());
        }

        if prev_model.unwrap_or_default().do_not_parse {
            self.log_info("do_not_parse is true, skipping parse");
            return Ok(());
        }

        // Skip if asset_uri contains any of the uris in URI_SKIP_LIST
        if let Some(blacklist) = self.config.uri_blacklist.clone() {
            if blacklist.iter().any(|uri| self.asset_uri.contains(uri)) {
                self.log_info("Found match in URI skip list, skipping parse");
                SKIP_URI_COUNT.with_label_values(&["blacklist"]).inc();
                return Ok(());
            }
        }

        // Skip if asset_uri is not a valid URI
        if Url::parse(&self.asset_uri).is_err() {
            self.log_info("URI is invalid, skipping parse, marking as do_not_parse");
            self.model.set_do_not_parse(true);
            SKIP_URI_COUNT.with_label_values(&["invalid"]).inc();
            if let Err(e) = upsert_uris(&mut self.conn, &self.model) {
                self.log_error("Commit to Postgres failed", &e);
            }
            return Ok(());
        }

        // Parse asset_uri
        self.log_info("Parsing asset_uri");
        let json_uri = URIParser::parse(
            self.config.ipfs_prefix.clone(),
            self.model.get_asset_uri(),
            self.config.ipfs_auth_key.clone(),
        )
        .unwrap_or_else(|_| {
            self.log_warn("Failed to parse asset_uri", None);
            PARSE_URI_TYPE_COUNT.with_label_values(&["other"]).inc();
            self.model.get_asset_uri()
        });

        // Parse JSON for raw_image_uri and raw_animation_uri
        self.log_info("Starting JSON parsing");
        let (raw_image_uri, raw_animation_uri, json) = JSONParser::parse(
            json_uri,
            self.config
                .max_file_size_bytes
                .unwrap_or(DEFAULT_MAX_FILE_SIZE_BYTES),
        )
        .await
        .unwrap_or_else(|e| {
            // Increment retry count if JSON parsing fails
            self.log_warn("JSON parsing failed", Some(&e));
            self.model.increment_json_parser_retry_count();
            (None, None, Value::Null)
        });

        self.model.set_raw_image_uri(raw_image_uri);
        self.model.set_raw_animation_uri(raw_animation_uri);

        // Save parsed JSON to GCS
        if json != Value::Null {
            self.log_info("Writing JSON to GCS");
            let cdn_json_uri_result = write_json_to_gcs(
                self.config.bucket.clone(),
                self.asset_data_id.clone(),
                &json,
                &self.gcs_client,
            )
            .await;

            if let Err(e) = cdn_json_uri_result.as_ref() {
                self.log_warn(
                    "Failed to write JSON to GCS, maybe upload timed out?",
                    Some(e),
                );
            }

            let cdn_json_uri = cdn_json_uri_result
                .map(|value| format!("{}{}", self.config.cdn_prefix, value))
                .ok();
            self.model.set_cdn_json_uri(cdn_json_uri);
        }

        // Commit model to Postgres
        self.log_info("Committing JSON to Postgres");
        if let Err(e) = upsert_uris(&mut self.conn, &self.model) {
            self.log_error("Commit to Postgres failed", &e);
        }

        // Deduplicate raw_image_uri
        // Proceed with image optimization of force or if raw_image_uri has not been parsed
        // Since we default to asset_uri, this check works if raw_image_uri is null because deduplication for asset_uri has already taken place
        if self.force
            || self.model.get_raw_image_uri().map_or(true, |uri_option| {
                match NFTMetadataCrawlerURIsQuery::get_by_raw_image_uri(
                    self.asset_uri.clone(),
                    uri_option,
                    &mut self.conn,
                ) {
                    Some(uris) => {
                        self.log_info("Duplicate raw_image_uri found");
                        DUPLICATE_RAW_IMAGE_URI_COUNT.inc();
                        self.model.set_cdn_image_uri(uris.cdn_image_uri);
                        false
                    },
                    None => true,
                }
            })
        {
            // Parse raw_image_uri, use asset_uri if parsing fails
            self.log_info("Parsing raw_image_uri");
            let raw_image_uri = self
                .model
                .get_raw_image_uri()
                .unwrap_or(self.model.get_asset_uri());
            let img_uri = URIParser::parse(
                self.config.ipfs_prefix.clone(),
                raw_image_uri.clone(),
                self.config.ipfs_auth_key.clone(),
            )
            .unwrap_or_else(|_| {
                self.log_warn("Failed to parse raw_image_uri", None);
                PARSE_URI_TYPE_COUNT.with_label_values(&["other"]).inc();
                raw_image_uri
            });

            // Resize and optimize image
            self.log_info("Starting image optimization");
            OPTIMIZE_IMAGE_TYPE_COUNT
                .with_label_values(&["image"])
                .inc();
            let (image, format) = ImageOptimizer::optimize(
                img_uri,
                self.config
                    .max_file_size_bytes
                    .unwrap_or(DEFAULT_MAX_FILE_SIZE_BYTES),
                self.config.image_quality.unwrap_or(DEFAULT_IMAGE_QUALITY),
                self.config
                    .max_image_dimensions
                    .unwrap_or(DEFAULT_MAX_IMAGE_DIMENSIONS),
            )
            .await
            .unwrap_or_else(|e| {
                // Increment retry count if image is None
                self.log_warn("Image optimization failed", Some(&e));
                self.model.increment_image_optimizer_retry_count();
                (vec![], ImageFormat::Png)
            });

            if image.is_empty() && json == Value::Null {
                self.log_info("Image and JSON are empty, skipping parse, marking as do_not_parse");
                self.model.set_do_not_parse(true);
                SKIP_URI_COUNT.with_label_values(&["empty"]).inc();
                if let Err(e) = upsert_uris(&mut self.conn, &self.model) {
                    self.log_error("Commit to Postgres failed", &e);
                }
                return Ok(());
            }

            // Save resized and optimized image to GCS
            if !image.is_empty() {
                self.log_info("Writing image to GCS");
                let cdn_image_uri_result = write_image_to_gcs(
                    format,
                    self.config.bucket.clone(),
                    self.asset_data_id.clone(),
                    image,
                    &self.gcs_client,
                )
                .await;

                if let Err(e) = cdn_image_uri_result.as_ref() {
                    self.log_warn(
                        "Failed to write image to GCS, maybe upload timed out?",
                        Some(e),
                    );
                }

                let cdn_image_uri = cdn_image_uri_result
                    .map(|value| format!("{}{}", self.config.cdn_prefix, value))
                    .ok();
                self.model.set_cdn_image_uri(cdn_image_uri);
            }
        }

        // Commit model to Postgres
        self.log_info("Committing image to Postgres");
        if let Err(e) = upsert_uris(&mut self.conn, &self.model) {
            self.log_error("Commit to Postgres failed", &e);
        }

        // Deduplicate raw_animation_uri
        // Set raw_animation_uri_option to None if not force and raw_animation_uri already exists
        let mut raw_animation_uri_option = self.model.get_raw_animation_uri();
        if !self.force
            && raw_animation_uri_option.clone().map_or(true, |uri| {
                match NFTMetadataCrawlerURIsQuery::get_by_raw_animation_uri(
                    self.asset_uri.clone(),
                    uri,
                    &mut self.conn,
                ) {
                    Some(uris) => {
                        self.log_info("Duplicate raw_animation_uri found");
                        DUPLICATE_RAW_ANIMATION_URI_COUNT.inc();
                        self.model.set_cdn_animation_uri(uris.cdn_animation_uri);
                        true
                    },
                    None => true,
                }
            })
        {
            raw_animation_uri_option = None;
        }

        // If raw_animation_uri_option is None, skip
        if let Some(raw_animation_uri) = raw_animation_uri_option {
            self.log_info("Starting animation optimization");
            let animation_uri = URIParser::parse(
                self.config.ipfs_prefix.clone(),
                raw_animation_uri.clone(),
                self.config.ipfs_auth_key.clone(),
            )
            .unwrap_or_else(|_| {
                self.log_warn("Failed to parse raw_animation_uri", None);
                PARSE_URI_TYPE_COUNT.with_label_values(&["other"]).inc();
                raw_animation_uri
            });

            // Resize and optimize animation
            self.log_info("Starting animation optimization");
            OPTIMIZE_IMAGE_TYPE_COUNT
                .with_label_values(&["animation"])
                .inc();
            let (animation, format) = ImageOptimizer::optimize(
                animation_uri,
                self.config
                    .max_file_size_bytes
                    .unwrap_or(DEFAULT_MAX_FILE_SIZE_BYTES),
                self.config.image_quality.unwrap_or(DEFAULT_IMAGE_QUALITY),
                self.config
                    .max_image_dimensions
                    .unwrap_or(DEFAULT_MAX_IMAGE_DIMENSIONS),
            )
            .await
            .unwrap_or_else(|e| {
                // Increment retry count if animation is None
                self.log_warn("Animation optimization failed", Some(&e));
                self.model.increment_animation_optimizer_retry_count();
                (vec![], ImageFormat::Png)
            });

            // Save resized and optimized animation to GCS
            if !animation.is_empty() {
                self.log_info("Writing animation to GCS");
                let cdn_animation_uri_result = write_image_to_gcs(
                    format,
                    self.config.bucket.clone(),
                    self.asset_data_id.clone(),
                    animation,
                    &self.gcs_client,
                )
                .await;

                if let Err(e) = cdn_animation_uri_result.as_ref() {
                    self.log_error("Failed to write animation to GCS", e);
                }

                let cdn_animation_uri = cdn_animation_uri_result
                    .map(|value| format!("{}{}", self.config.cdn_prefix, value))
                    .ok();
                self.model.set_cdn_animation_uri(cdn_animation_uri);
            }
        }

        // Commit model to Postgres
        self.log_info("Committing animation to Postgres");
        if let Err(e) = upsert_uris(&mut self.conn, &self.model) {
            self.log_error("Commit to Postgres failed", &e);
        }

        PARSER_SUCCESSES_COUNT.inc();
        Ok(())
    }

    fn log_info(&self, message: &str) {
        info!(
            pubsub_message = self.pubsub_message,
            asset_data_id = self.asset_data_id,
            asset_uri = self.asset_uri,
            last_transaction_version = self.last_transaction_version,
            last_transaction_timestamp = self.last_transaction_timestamp.to_string(),
            "[NFT Metadata Crawler] {}",
            message
        );
    }

    fn log_warn(&self, message: &str, e: Option<&anyhow::Error>) {
        warn!(
            pubsub_message = self.pubsub_message,
            asset_data_id = self.asset_data_id,
            asset_uri = self.asset_uri,
            last_transaction_version = self.last_transaction_version,
            last_transaction_timestamp = self.last_transaction_timestamp.to_string(),
            error = ?e,
            "[NFT Metadata Crawler] {}",
            message
        );
    }

    fn log_error(&self, message: &str, e: &anyhow::Error) {
        error!(
            pubsub_message = self.pubsub_message,
            asset_data_id = self.asset_data_id,
            asset_uri = self.asset_uri,
            last_transaction_version = self.last_transaction_version,
            last_transaction_timestamp = self.last_transaction_timestamp.to_string(),
            error = ?e,
            "[NFT Metadata Crawler] {}",
            message
        );
    }
}
