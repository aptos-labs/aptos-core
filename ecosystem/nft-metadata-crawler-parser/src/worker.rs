// Copyright © Aptos Foundation

use crate::{
    models::{
        nft_metadata_crawler_uris::NFTMetadataCrawlerURIs,
        nft_metadata_crawler_uris_query::NFTMetadataCrawlerURIsQuery,
    },
    utils::{
        constants::{MAX_RETRY_TIME_SECONDS, URI_SKIP_LIST},
        counters::{
            DUPLICATE_RAW_ANIMATION_URI_COUNT, DUPLICATE_RAW_IMAGE_URI_COUNT,
            DUPLICATE_TOKEN_URI_COUNT, GOT_CONNECTION_COUNT, OPTIMIZE_IMAGE_TYPE_COUNT,
            PARSER_FAIL_COUNT, PARSER_INVOCATIONS_COUNT, PARSER_SUCCESSES_COUNT,
            PARSE_URI_TYPE_COUNT, PUBSUB_ACK_SUCCESS_COUNT, PUBSUB_STREAM_RESET_COUNT,
            SKIP_URI_COUNT, UNABLE_TO_GET_CONNECTION_COUNT,
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
use anyhow::Context;
use aptos_indexer_grpc_server_framework::RunnableConfig;
use diesel::{
    r2d2::{ConnectionManager, Pool, PooledConnection},
    PgConnection,
};
use futures::{future::join_all, StreamExt};
use google_cloud_pubsub::{
    client::{Client, ClientConfig},
    subscription::{MessageStream, Subscription},
};
use image::ImageFormat;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::{Duration, Instant};
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

/// Structs to hold config from YAML
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ParserConfig {
    pub google_application_credentials: Option<String>,
    pub bucket: String,
    pub subscription_name: String,
    pub database_url: String,
    pub cdn_prefix: String,
    pub ipfs_prefix: String,
    pub num_parsers: usize,
    pub max_file_size_bytes: u32,
    pub image_quality: u8, // Quality up to 100
    pub ack_parsed_uris: Option<bool>,
}

/// Repeatedly pulls workers from Channel and perform parsing operations
async fn spawn_parser(
    parser_config: ParserConfig,
    pool: Pool<ConnectionManager<PgConnection>>,
    subscription: Subscription,
    ack_parsed_uris: bool,
) {
    let mut db_chain_id = None;
    let mut stream = get_new_subscription_stream(&subscription).await;
    while let Some(msg) = stream.next().await {
        let start_time = Instant::now();
        let pubsub_message = String::from_utf8(msg.message.clone().data).unwrap_or_else(|e| {
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
            if ack_parsed_uris {
                info!(
                    pubsub_message = pubsub_message,
                    time_elapsed = start_time.elapsed().as_secs_f64(),
                    "[NFT Metadata Crawler] Received worker, acking message"
                );
                if let Err(e) = send_ack(&subscription, msg.ack_id()).await {
                    error!(
                        pubsub_message = pubsub_message,
                        error = ?e,
                        "[NFT Metadata Crawler] Resetting stream"
                    );
                    stream = get_new_subscription_stream(&subscription).await;
                }
                PUBSUB_ACK_SUCCESS_COUNT.inc();
                continue;
            }
        }

        // Parse PubSub message
        let parts: Vec<&str> = pubsub_message.split(',').collect();

        // Perform chain id check
        // If chain id is not set, set it
        let mut conn = get_conn(pool.clone());

        let grpc_chain_id = parts[4].parse::<u64>().unwrap_or_else(|e| {
            error!(
                error = ?e,
                "[NFT Metadata Crawler] Failed to parse chain id from PubSub message"
            );
            panic!();
        });
        if let Some(existing_id) = db_chain_id {
            if grpc_chain_id != existing_id {
                error!(
                    chain_id = grpc_chain_id,
                    existing_id = existing_id,
                    "[NFT Metadata Crawler] Stream somehow changed chain id!",
                );
                panic!("[NFT Metadata Crawler] Stream somehow changed chain id!");
            }
        } else {
            db_chain_id = Some(
                check_or_update_chain_id(&mut conn, grpc_chain_id as i64)
                    .expect("Chain id should match"),
            );
        }

        // Spawn worker
        let mut worker = Worker::new(
            parser_config.clone(),
            conn,
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

        // Sends ack to PubSub only if ack_parsed_uris flag is true
        if ack_parsed_uris {
            info!(
                pubsub_message = pubsub_message,
                time_elapsed = start_time.elapsed().as_secs_f64(),
                "[NFT Metadata Crawler] Received worker, acking message"
            );
            if let Err(e) = send_ack(&subscription, msg.ack_id()).await {
                error!(
                    pubsub_message = pubsub_message,
                    error = ?e,
                    "[NFT Metadata Crawler] Resetting stream"
                );
                stream = get_new_subscription_stream(&subscription).await;
                continue;
            }
            PUBSUB_ACK_SUCCESS_COUNT.inc();
        }

        info!(
            pubsub_message = pubsub_message,
            "[NFT Metadata Crawler] Starting worker"
        );

        PARSER_INVOCATIONS_COUNT.inc();
        if let Err(e) = worker.parse().await {
            warn!(
                pubsub_message = pubsub_message,
                error = ?e,
                "[NFT Metadata Crawler] Parsing failed"
            );
            PARSER_FAIL_COUNT.inc();
        } else {
            PARSER_SUCCESSES_COUNT.inc();
        }

        info!(
            pubsub_message = pubsub_message,
            "[NFT Metadata Crawler] Worker finished"
        );
    }
}

/// Returns a new stream from a PubSub subscription
async fn get_new_subscription_stream(subscription: &Subscription) -> MessageStream {
    PUBSUB_STREAM_RESET_COUNT.inc();
    subscription.subscribe(None).await.unwrap_or_else(|e| {
        error!(
            error = ?e,
            "[NFT Metadata Crawler] Failed to get stream from PubSub subscription"
        );
        panic!();
    })
}

/// Sends ack to PubSub, times out after MAX_RETRY_TIME_SECONDS
async fn send_ack(subscription: &Subscription, ack_id: &str) -> anyhow::Result<()> {
    let ack = ack_id.to_string();
    tokio::time::timeout(
        Duration::from_secs(MAX_RETRY_TIME_SECONDS),
        subscription.ack(vec![ack.to_string()]),
    )
    .await?
    .context("Failed to ack message to PubSub")
}

/// Gets a Postgres connection from the pool
fn get_conn(
    pool: Pool<ConnectionManager<PgConnection>>,
) -> PooledConnection<ConnectionManager<PgConnection>> {
    loop {
        match pool.get() {
            Ok(conn) => {
                GOT_CONNECTION_COUNT.inc();
                return conn;
            },
            Err(err) => {
                UNABLE_TO_GET_CONNECTION_COUNT.inc();
                error!(
                    "Could not get DB connection from pool, will retry in {:?}. Err: {:?}",
                    pool.connection_timeout(),
                    err
                );
            },
        };
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

        // Establish gRPC client
        let config = ClientConfig::default()
            .with_auth()
            .await
            .unwrap_or_else(|e| {
                error!(
                    error = ?e,
                    "[NFT Metadata Crawler] Failed to create gRPC client config"
                );
                panic!();
            });
        let client = Client::new(config).await.unwrap_or_else(|e| {
            error!(
                error = ?e,
                "[NFT Metadata Crawler] Failed to create gRPC client"
            );
            panic!();
        });
        let subscription = client.subscription(&self.subscription_name);

        // Spawns workers
        let mut workers: Vec<JoinHandle<()>> = Vec::new();
        for _ in 0..self.num_parsers {
            let worker = tokio::spawn(spawn_parser(
                self.clone(),
                pool.clone(),
                subscription.clone(),
                self.ack_parsed_uris.unwrap_or(false),
            ));

            workers.push(worker);
        }

        join_all(workers).await;
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
    pubsub_message: String,
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
        pubsub_message: String,
        token_data_id: String,
        token_uri: String,
        last_transaction_version: i32,
        last_transaction_timestamp: chrono::NaiveDateTime,
        force: bool,
    ) -> Self {
        let worker = Self {
            config,
            conn,
            pubsub_message,
            model: NFTMetadataCrawlerURIs::new(token_uri.clone()),
            token_data_id,
            token_uri,
            last_transaction_version,
            last_transaction_timestamp,
            force,
        };
        worker.log_info("Created worker");
        worker
    }

    /// Main parsing flow
    pub async fn parse(&mut self) -> anyhow::Result<()> {
        // Deduplicate token_uri
        // Exit if not force or if token_uri has already been parsed
        if !self.force
            && NFTMetadataCrawlerURIsQuery::get_by_token_uri(self.token_uri.clone(), &mut self.conn)
                .is_some()
        {
            self.log_info("Duplicate token_uri found, skipping parse");
            DUPLICATE_TOKEN_URI_COUNT.inc();
            return Ok(());
        }

        // Skip if token_uri contains any of the uris in URI_SKIP_LIST
        if URI_SKIP_LIST
            .iter()
            .any(|&uri| self.token_uri.contains(uri))
        {
            self.log_info("Found match in URI skip list, skipping parse");
            SKIP_URI_COUNT.inc();
            return Ok(());
        }

        // Parse token_uri
        self.log_info("Parsing token_uri");
        let json_uri =
            URIParser::parse(self.config.ipfs_prefix.clone(), self.model.get_token_uri())
                .unwrap_or_else(|_| {
                    self.log_warn("Failed to parse token_uri", None);
                    PARSE_URI_TYPE_COUNT.with_label_values(&["other"]).inc();
                    self.model.get_token_uri()
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
            let cdn_json_uri_result =
                write_json_to_gcs(self.config.bucket.clone(), self.token_data_id.clone(), json)
                    .await;

            if let Err(e) = cdn_json_uri_result.as_ref() {
                self.log_error("Failed to write JSON to GCS", e);
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
        // Since we default to token_uri, this check works if raw_image_uri is null because deduplication for token_uri has already taken place
        if self.force
            || self.model.get_raw_image_uri().map_or(true, |uri_option| {
                match NFTMetadataCrawlerURIsQuery::get_by_raw_image_uri(
                    self.token_uri.clone(),
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
            // Parse raw_image_uri, use token_uri if parsing fails
            self.log_info("Parsing raw_image_uri");
            let raw_image_uri = self
                .model
                .get_raw_image_uri()
                .unwrap_or(self.model.get_token_uri());
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
                    self.token_data_id.clone(),
                    image,
                )
                .await;

                if let Err(e) = cdn_image_uri_result.as_ref() {
                    self.log_error("Failed to write image to GCS", e);
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
                    self.token_uri.clone(),
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
                    self.token_data_id.clone(),
                    animation,
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

        Ok(())
    }

    fn log_info(&self, message: &str) {
        info!(
            pubsub_message = self.pubsub_message,
            token_data_id = self.token_data_id,
            token_uri = self.token_uri,
            last_transaction_version = self.last_transaction_version,
            last_transaction_timestamp = self.last_transaction_timestamp.to_string(),
            "[NFT Metadata Crawler] {}",
            message
        );
    }

    fn log_warn(&self, message: &str, e: Option<&anyhow::Error>) {
        warn!(
            pubsub_message = self.pubsub_message,
            token_data_id = self.token_data_id,
            token_uri = self.token_uri,
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
            token_data_id = self.token_data_id,
            token_uri = self.token_uri,
            last_transaction_version = self.last_transaction_version,
            last_transaction_timestamp = self.last_transaction_timestamp.to_string(),
            error = ?e,
            "[NFT Metadata Crawler] {}",
            message
        );
        panic!();
    }
}
