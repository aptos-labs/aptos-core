// Copyright © Aptos Foundation

use crate::{
    models::{
        nft_metadata_crawler_uris::NFTMetadataCrawlerURIs,
        nft_metadata_crawler_uris_query::NFTMetadataCrawlerURIsQuery,
    },
    utils::{
        constants::URI_SKIP_LIST,
        counters::{
            DUPLICATE_ASSET_URI_COUNT, DUPLICATE_RAW_ANIMATION_URI_COUNT,
            DUPLICATE_RAW_IMAGE_URI_COUNT, GOT_CONNECTION_COUNT, OPTIMIZE_IMAGE_TYPE_COUNT,
            PARSER_FAIL_COUNT, PARSER_INVOCATIONS_COUNT, PARSER_SUCCESSES_COUNT,
            PARSE_URI_TYPE_COUNT, PUBSUB_ACK_SUCCESS_COUNT, SKIP_URI_COUNT,
            UNABLE_TO_GET_CONNECTION_COUNT,
        },
        database::{
            check_or_update_chain_id, establish_connection_pool, run_migrations, upsert_uris,
        },
        gcs::{write_image_to_gcs, write_json_to_gcs},
        image_optimizer::ImageOptimizer,
        json_parser::JSONParser,
        uri_parser::URIParser,
    },
};
use aptos_indexer_grpc_server_framework::RunnableConfig;
use bytes::Bytes;
use diesel::{
    r2d2::{ConnectionManager, Pool, PooledConnection},
    PgConnection,
};
use google_cloud_storage::client::{Client as GCSClient, ClientConfig as GCSClientConfig};
use image::ImageFormat;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tracing::{error, info, warn};
use url::Url;
use warp::Filter;

/// Structs to hold config from YAML
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ParserConfig {
    pub google_application_credentials: Option<String>,
    pub bucket: String,
    pub database_url: String,
    pub cdn_prefix: String,
    pub ipfs_prefix: String,
    pub max_file_size_bytes: u32,
    pub image_quality: u8, // Quality up to 100
    pub ack_parsed_uris: Option<bool>,
    pub server_port: u16,
}

/// Struct to hold context required for parsing
#[derive(Clone)]
pub struct ServerContext {
    pub parser_config: ParserConfig,
    pub pool: Pool<ConnectionManager<PgConnection>>,
    pub gcs_client: GCSClient,
}

/// Repeatedly pulls workers from Channel and perform parsing operations
async fn spawn_parser(
    parser_config: ParserConfig,
    msg_base64: Bytes,
    pool: Pool<ConnectionManager<PgConnection>>,
    gcs_client: GCSClient,
) {
    PARSER_INVOCATIONS_COUNT.inc();
    let pubsub_message = String::from_utf8(msg_base64.to_vec()).unwrap_or_else(|e| {
        error!(
            error = ?e,
            "[NFT Metadata Crawler] Failed to parse PubSub message"
        );
        panic!();
    });

    info!(
        pubsub_message = pubsub_message,
        "[NFT Metadata Crawler] Received message from PubSub"
    );

    // Skips message if it does not have 5 commas (likely malformed URI)
    if pubsub_message.matches(',').count() != 5 {
        // Sends ack to PubSub only if ack_parsed_uris flag is true
        info!("[NFT Metadata Crawler] More than 5 commas, skipping message");
        SKIP_URI_COUNT.with_label_values(&["invalid"]).inc();
        return;
    }

    // Parse PubSub message
    let parts: Vec<&str> = pubsub_message.split(',').collect();

    // Perform chain id check
    // If chain id is not set, set it
    let mut conn = pool.get().unwrap_or_else(|e| {
        error!(
                pubsub_message = pubsub_message,
                error = ?e,
                "[NFT Metadata Crawler] Failed to get DB connection from pool");
        UNABLE_TO_GET_CONNECTION_COUNT.inc();
        panic!();
    });
    GOT_CONNECTION_COUNT.inc();

    let grpc_chain_id = parts[4].parse::<u64>().unwrap_or_else(|e| {
        error!(
            error = ?e,
            "[NFT Metadata Crawler] Failed to parse chain id from PubSub message"
        );
        panic!();
    });

    // Panic if chain id of PubSub message does not match chain id in DB
    check_or_update_chain_id(&mut conn, grpc_chain_id as i64).expect("Chain id should match");

    // Spawn worker
    let mut worker = Worker::new(
            parser_config.clone(),
            conn,
        gcs_client.clone(),
            pubsub_message.clone(),
            parts[0].to_string(),
            parts[1].to_string(),
            parts[2].to_string().parse().unwrap_or_else(|e|{
                error!(
                    error = ?e,
                    "[NFT Metadata Crawler] Failed to parse last transaction version from PubSub message"
                );
                panic!();
            }),
            chrono::NaiveDateTime::parse_from_str(parts[3], "%Y-%m-%d %H:%M:%S %Z").unwrap_or(
                chrono::NaiveDateTime::parse_from_str(parts[3], "%Y-%m-%d %H:%M:%S%.f %Z").unwrap_or_else(
                    |e| {
                        error!(
                            error = ?e,
                            "[NFT Metadata Crawler] Failed to parse timestamp from PubSub message"
                        );
                        panic!();
                    },
                ),
            ),
            parts[5].parse::<bool>().unwrap_or(false),
        );

    info!(
        pubsub_message = pubsub_message,
        "[NFT Metadata Crawler] Starting worker"
    );

    if let Err(e) = worker.parse().await {
        warn!(
            pubsub_message = pubsub_message,
            error = ?e,
            "[NFT Metadata Crawler] Parsing failed"
        );
        PARSER_FAIL_COUNT.inc();
    }

    info!(
        pubsub_message = pubsub_message,
        "[NFT Metadata Crawler] Worker finished"
    );
}

/// Handles calling parser for the root endpoint
async fn handle_root(
    msg: Bytes,
    context: Arc<ServerContext>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let to_ack = context.parser_config.ack_parsed_uris.unwrap_or(false);

    // Use spawn_blocking to run the function on a separate thread.
    let _ = tokio::spawn(spawn_parser(
        context.parser_config.clone(),
        msg,
        context.pool.clone(),
        context.gcs_client.clone(),
    ))
    .await;

    if !to_ack {
        return Ok(warp::reply::with_status(
            warp::reply(),
            warp::http::StatusCode::BAD_REQUEST,
        ));
    }

    PUBSUB_ACK_SUCCESS_COUNT.inc();
    Ok(warp::reply::with_status(
        warp::reply(),
        warp::http::StatusCode::OK,
    ))
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
        let pool = establish_connection_pool(self.database_url.clone());
        info!("[NFT Metadata Crawler] Database connection successful");

        info!("[NFT Metadata Crawler] Running migrations");
        run_migrations(&pool);
        info!("[NFT Metadata Crawler] Finished migrations");

        if let Some(google_application_credentials) = self.google_application_credentials.clone() {
            std::env::set_var(
                "GOOGLE_APPLICATION_CREDENTIALS",
                google_application_credentials,
            );
        }

        // Establish GCS client
        let gcs_config = GCSClientConfig::default()
            .with_auth()
            .await
            .unwrap_or_else(|e| {
                error!(
                    error = ?e,
                    "[NFT Metadata Crawler] Failed to create gRPC client config"
                );
                panic!();
            });

        // Create request context
        let context = Arc::new(ServerContext {
            parser_config: self.clone(),
            pool,
            gcs_client: GCSClient::new(gcs_config),
        });

        // Create web server
        let route = warp::post()
            .and(warp::path::end())
            .and(warp::body::bytes())
            .and(warp::any().map(move || context.clone()))
            .and_then(handle_root);
        warp::serve(route)
            .run(([0, 0, 0, 0], self.server_port))
            .await;
        Ok(())
    }

    fn get_server_name(&self) -> String {
        "parser".to_string()
    }
}

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
        if !self.force
            && NFTMetadataCrawlerURIsQuery::get_by_asset_uri(self.asset_uri.clone(), &mut self.conn)
                .is_some()
        {
            self.log_info("Duplicate asset_uri found, skipping parse");
            DUPLICATE_ASSET_URI_COUNT.inc();
            return Ok(());
        }

        // Skip if asset_uri contains any of the uris in URI_SKIP_LIST
        if URI_SKIP_LIST
            .iter()
            .any(|&uri| self.asset_uri.contains(uri))
        {
            self.log_info("Found match in URI skip list, skipping parse");
            SKIP_URI_COUNT.with_label_values(&["blacklist"]).inc();
            return Ok(());
        }

        // Skip if asset_uri is not a valid URI
        if Url::parse(&self.asset_uri).is_err() {
            self.log_info("URI is invalid, skipping parse");
            SKIP_URI_COUNT.with_label_values(&["invalid"]).inc();
            return Ok(());
        }

        // Parse asset_uri
        self.log_info("Parsing asset_uri");
        let json_uri =
            URIParser::parse(self.config.ipfs_prefix.clone(), self.model.get_asset_uri())
                .unwrap_or_else(|_| {
                    self.log_warn("Failed to parse asset_uri", None);
                    PARSE_URI_TYPE_COUNT.with_label_values(&["other"]).inc();
                    self.model.get_asset_uri()
                });

        // Parse JSON for raw_image_uri and raw_animation_uri
        self.log_info("Starting JSON parsing");
        let (raw_image_uri, raw_animation_uri, json) =
            JSONParser::parse(json_uri, self.config.max_file_size_bytes)
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
                json,
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
        if let Err(e) = upsert_uris(&mut self.conn, self.model.clone()) {
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
            let img_uri = URIParser::parse(self.config.ipfs_prefix.clone(), raw_image_uri.clone())
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
                self.config.max_file_size_bytes,
                self.config.image_quality,
            )
            .await
            .unwrap_or_else(|e| {
                // Increment retry count if image is None
                self.log_warn("Image optimization failed", Some(&e));
                self.model.increment_image_optimizer_retry_count();
                (vec![], ImageFormat::Png)
            });

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
        if let Err(e) = upsert_uris(&mut self.conn, self.model.clone()) {
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
            let animation_uri =
                URIParser::parse(self.config.ipfs_prefix.clone(), raw_animation_uri.clone())
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
                self.config.max_file_size_bytes,
                self.config.image_quality,
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
        if let Err(e) = upsert_uris(&mut self.conn, self.model.clone()) {
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
