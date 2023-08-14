// Copyright © Aptos Foundation

use crate::{
    models::{
        nft_metadata_crawler_uris::NFTMetadataCrawlerURIs,
        nft_metadata_crawler_uris_query::NFTMetadataCrawlerURIsQuery,
    },
    utils::{
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
use chrono::NaiveDateTime;
use crossbeam_channel::{bounded, Receiver, Sender};
use diesel::{
    r2d2::{ConnectionManager, Pool, PooledConnection},
    PgConnection,
};
use futures::StreamExt;
use google_cloud_pubsub::{
    client::{Client, ClientConfig},
    subscription::Subscription,
};
use image::ImageFormat;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{sync::Arc, time::Duration};
use tokio::{
    sync::{Mutex, Semaphore},
    task::JoinHandle,
    time::sleep,
};
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

/// Subscribes to PubSub and sends URIs to Channel
/// - Creates an infinite loop that pulls `msgs_per_pull` entries from PubSub
/// - Parses each entry into a `Worker` and sends to Channel
async fn consume_pubsub_entries_to_channel_loop(
    parser_config: ParserConfig,
    sender: Sender<(Worker, String)>,
    subscription: Subscription,
    pool: Pool<ConnectionManager<PgConnection>>,
) -> anyhow::Result<()> {
    let mut db_chain_id = None;
    let mut stream = subscription.subscribe(None).await?;
    while let Some(msg) = stream.next().await {
        // Parse metadata from Pubsub message and create worker
        let ack = msg.ack_id();
        let entry_string = String::from_utf8(msg.message.clone().data)?;
        let parts: Vec<&str> = entry_string.split(',').collect();

        let mut conn = pool.get()?;
        let grpc_chain_id = parts[4].parse::<u64>()?;

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

        let worker = Worker::new(
            parser_config.clone(),
            conn,
            parts[0].to_string(),
            parts[1].to_string(),
            parts[2].to_string().parse()?,
            NaiveDateTime::parse_from_str(parts[3], "%Y-%m-%d %H:%M:%S %Z").unwrap_or(
                NaiveDateTime::parse_from_str(parts[3], "%Y-%m-%d %H:%M:%S%.f %Z")?,
            ),
            parts[5].parse::<bool>().unwrap_or(false),
        );

        // Send worker to channel
        sender.send((worker, ack.to_string())).unwrap_or_else(|e| {
            error!(
                error = ?e,
                "[NFT Metadata Crawler] Failed to send PubSub entry to channel"
            );
            panic!();
        });
    }

    Ok(())
}

/// Repeatedly pulls workers from Channel and perform parsing operations
async fn spawn_parser(
    semaphore: Arc<Semaphore>,
    receiver: Arc<Mutex<Receiver<(Worker, String)>>>,
    subscription: Subscription,
    release: bool,
) -> anyhow::Result<()> {
    loop {
        let _ = semaphore.acquire().await?;

        // Pulls worker from Channel
        let (mut worker, ack) = receiver.lock().await.recv()?;
        worker.parse().await?;

        // Sends ack to PubSub only if running on release mode
        if release {
            info!(
                token_data_id = worker.token_data_id,
                token_uri = worker.token_uri,
                last_transaction_version = worker.last_transaction_version,
                force = worker.force,
                "[NFT Metadata Crawler] Acking message"
            );
            subscription.ack(vec![ack]).await?;
        }

        sleep(Duration::from_millis(500)).await;
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
        let config = ClientConfig::default().with_auth().await?;
        let client = Client::new(config).await?;
        let subscription = client.subscription(&self.subscription_name);

        // Create workers
        let (sender, receiver) = bounded::<(Worker, String)>(2 * self.num_parsers);
        let receiver = Arc::new(Mutex::new(receiver));
        let semaphore = Arc::new(Semaphore::new(self.num_parsers));

        // Spawn producer
        let producer = tokio::spawn(consume_pubsub_entries_to_channel_loop(
            self.clone(),
            sender,
            subscription.clone(),
            pool,
        ));

        // Spawns workers
        let mut workers: Vec<JoinHandle<anyhow::Result<()>>> = Vec::new();
        for _ in 0..self.num_parsers {
            let worker = tokio::spawn(spawn_parser(
                Arc::clone(&semaphore),
                Arc::clone(&receiver),
                subscription.clone(),
                self.ack_parsed_uris.unwrap_or(false),
            ));

            workers.push(worker);
        }

        match producer.await {
            Ok(_) => (),
            Err(e) => error!("[NFT Metadata Crawler] Producer error: {:?}", e),
        }

        for worker in workers {
            match worker.await {
                Ok(_) => (),
                Err(e) => error!("[NFT Metadata Crawler] Worker error: {:?}", e),
            }
        }
        Ok(())
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
            token_data_id = self.token_data_id,
            token_uri = self.token_uri,
            last_transaction_version = self.last_transaction_version,
            force = self.force,
            "[NFT Metadata Crawler] Starting worker"
        );

        // Deduplicate token_uri
        // Exit if not force or if token_uri has already been parsed
        if !self.force
            && NFTMetadataCrawlerURIsQuery::get_by_token_uri(
                self.token_uri.clone(),
                &mut self.conn,
            )?
            .is_some()
        {
            return Ok(());
        }

        // Parse token_uri
        let json_uri =
            URIParser::parse(self.config.ipfs_prefix.clone(), self.model.get_token_uri())
                .unwrap_or(self.model.get_token_uri());

        // Parse JSON for raw_image_uri and raw_animation_uri
        let (raw_image_uri, raw_animation_uri, json) =
            JSONParser::parse(json_uri, self.config.max_file_size_bytes)
                .await
                .unwrap_or_else(|e| {
                    // Increment retry count if JSON parsing fails
                    warn!(
                        token_data_id=self.token_data_id,
                        token_uri=self.token_uri,
                        last_transaction_version = self.last_transaction_version,
                        force = self.force,
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
            let cdn_json_uri_result =
                write_json_to_gcs(self.config.bucket.clone(), self.token_data_id.clone(), json)
                    .await;

            if let Err(e) = cdn_json_uri_result.as_ref() {
                error!(
                    token_data_id=self.token_data_id,
                    token_uri=self.token_uri,
                    last_transaction_version = self.last_transaction_version,
                    force = self.force,
                    error = ?e,
                    "[NFT Metadata Crawler] Failed to write JSON to GCS"
                );
                panic!();
            }

            let cdn_json_uri = cdn_json_uri_result
                .map(|value| format!("{}{}", self.config.cdn_prefix, value))
                .ok();
            self.model.set_cdn_json_uri(cdn_json_uri);
        }

        // Commit model to Postgres
        if let Err(e) = upsert_uris(&mut self.conn, self.model.clone()) {
            error!(
                token_data_id=self.token_data_id,
                token_uri=self.token_uri,
                last_transaction_version = self.last_transaction_version,
                force = self.force,
                error = ?e,
                "[NFT Metadata Crawler] Commit to Postgres failed"
            );
            panic!();
        }

        // Deduplicate raw_image_uri
        // Proceed with image optimization of force or if raw_image_uri has not been parsed
        // Since we default to token_uri, this check works if raw_image_uri is null because deduplication for token_uri has already taken place
        if self.force
            || self.model.get_raw_image_uri().map_or(true, |uri_option| {
                NFTMetadataCrawlerURIsQuery::get_by_raw_image_uri(
                    self.token_uri.clone(),
                    uri_option,
                    &mut self.conn,
                )
                .map_or(true, |uri| match uri {
                    Some(uris) => {
                        self.model.set_cdn_image_uri(uris.cdn_image_uri);
                        false
                    },
                    None => true,
                })
            })
        {
            // Parse raw_image_uri, use token_uri if parsing fails
            let raw_image_uri = self
                .model
                .get_raw_image_uri()
                .unwrap_or(self.model.get_token_uri());
            let img_uri = URIParser::parse(self.config.ipfs_prefix.clone(), raw_image_uri.clone())
                .unwrap_or(raw_image_uri);

            // Resize and optimize image and animation
            let (image, format) = ImageOptimizer::optimize(
                img_uri,
                self.config.max_file_size_bytes,
                self.config.image_quality,
            )
            .await
            .unwrap_or_else(|e| {
                // Increment retry count if image is None
                warn!(
                    token_data_id=self.token_data_id,
                    token_uri=self.token_uri,
                    last_transaction_version = self.last_transaction_version,
                    force = self.force,
                    error = ?e,
                    "[NFT Metadata Crawler] Image optimization failed"
                );
                self.model.increment_image_optimizer_retry_count();
                (vec![], ImageFormat::Png)
            });

            if !image.is_empty() {
                // Save resized and optimized image to GCS
                let cdn_image_uri_result = write_image_to_gcs(
                    format,
                    self.config.bucket.clone(),
                    self.token_data_id.clone(),
                    image,
                )
                .await;

                if let Err(e) = cdn_image_uri_result.as_ref() {
                    error!(
                        token_data_id=self.token_data_id,
                        token_uri=self.token_uri,
                        last_transaction_version = self.last_transaction_version,
                        force = self.force,
                        error = ?e,
                        "[NFT Metadata Crawler] Failed to write image to GCS"
                    );
                    panic!();
                }

                let cdn_image_uri = cdn_image_uri_result
                    .map(|value| format!("{}{}", self.config.cdn_prefix, value))
                    .ok();
                self.model.set_cdn_image_uri(cdn_image_uri);
            }
        }

        // Commit model to Postgres
        if let Err(e) = upsert_uris(&mut self.conn, self.model.clone()) {
            error!(
                token_data_id=self.token_data_id,
                token_uri=self.token_uri,
                last_transaction_version = self.last_transaction_version,
                force = self.force,
                error = ?e,
                "[NFT Metadata Crawler] Commit to Postgres failed"
            );
            panic!();
        }

        // Deduplicate raw_animation_uri
        // Set raw_animation_uri_option to None if not force and raw_animation_uri already exists
        let mut raw_animation_uri_option = self.model.get_raw_animation_uri();
        if !self.force
            && raw_animation_uri_option.clone().map_or(true, |uri| {
                NFTMetadataCrawlerURIsQuery::get_by_raw_animation_uri(
                    self.token_uri.clone(),
                    uri,
                    &mut self.conn,
                )
                .map_or(true, |uri| match uri {
                    Some(uris) => {
                        self.model.set_cdn_animation_uri(uris.cdn_animation_uri);
                        true
                    },
                    None => true,
                })
            })
        {
            raw_animation_uri_option = None;
        }

        // If raw_animation_uri_option is None, skip
        if let Some(raw_animation_uri) = raw_animation_uri_option {
            let animation_uri =
                URIParser::parse(self.config.ipfs_prefix.clone(), raw_animation_uri.clone())
                    .unwrap_or(raw_animation_uri);

            // Resize and optimize animation
            let (animation, format) = ImageOptimizer::optimize(
                animation_uri,
                self.config.max_file_size_bytes,
                self.config.image_quality,
            )
            .await
            .unwrap_or_else(|e| {
                // Increment retry count if animation is None
                warn!(
                    token_data_id=self.token_data_id,
                    token_uri=self.token_uri,
                    last_transaction_version = self.last_transaction_version,
                    force = self.force,
                    error = ?e,
                    "[NFT Metadata Crawler] Animation optimization failed"
                );
                self.model.increment_animation_optimizer_retry_count();
                (vec![], ImageFormat::Png)
            });

            // Save resized and optimized animation to GCS
            if !animation.is_empty() {
                let cdn_animation_uri_result = write_image_to_gcs(
                    format,
                    self.config.bucket.clone(),
                    self.token_data_id.clone(),
                    animation,
                )
                .await;

                if let Err(e) = cdn_animation_uri_result.as_ref() {
                    error!(
                        token_data_id=self.token_data_id,
                        token_uri=self.token_uri,
                        last_transaction_version = self.last_transaction_version,
                        force = self.force,
                        error = ?e,
                        "[NFT Metadata Crawler] Failed to write animation to GCS"
                    );
                    panic!();
                }

                let cdn_animation_uri = cdn_animation_uri_result
                    .map(|value| format!("{}{}", self.config.cdn_prefix, value))
                    .ok();
                self.model.set_cdn_animation_uri(cdn_animation_uri);
            }
        }

        // Commit model to Postgres
        if let Err(e) = upsert_uris(&mut self.conn, self.model.clone()) {
            error!(
                token_data_id=self.token_data_id,
                token_uri=self.token_uri,
                last_transaction_version = self.last_transaction_version,
                force = self.force,
                error = ?e,
                "[NFT Metadata Crawler] Commit to Postgres failed"
            );
            panic!();
        }

        Ok(())
    }
}
