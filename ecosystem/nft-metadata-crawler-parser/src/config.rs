// Copyright © Aptos Foundation

use crate::{
    utils::{
        counters::{
            GOT_CONNECTION_COUNT, PARSER_FAIL_COUNT, PARSER_INVOCATIONS_COUNT,
            PUBSUB_ACK_SUCCESS_COUNT, SKIP_URI_COUNT, UNABLE_TO_GET_CONNECTION_COUNT,
        },
        database::{check_or_update_chain_id, establish_connection_pool, run_migrations},
    },
    worker::Worker,
};
use aptos_indexer_grpc_server_framework::RunnableConfig;
use bytes::Bytes;
use diesel::{
    r2d2::{ConnectionManager, Pool},
    PgConnection,
};
use google_cloud_storage::client::{Client as GCSClient, ClientConfig as GCSClientConfig};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info, warn};
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
    pub ipfs_auth_key: Option<String>,
    pub max_file_size_bytes: Option<u32>,
    pub image_quality: Option<u8>, // Quality up to 100
    pub max_image_dimensions: Option<u32>,
    pub ack_parsed_uris: Option<bool>,
    pub uri_blacklist: Option<Vec<String>>,
    pub server_port: u16,
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
