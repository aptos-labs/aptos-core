// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::Server,
    utils::{
        counters::{
            GOT_CONNECTION_COUNT, PARSER_FAIL_COUNT, PARSER_INVOCATIONS_COUNT,
            PUBSUB_ACK_SUCCESS_COUNT, SKIP_URI_COUNT, UNABLE_TO_GET_CONNECTION_COUNT,
        },
        database::check_or_update_chain_id,
    },
};
use axum::{http::StatusCode, response::Response, routing::post, Router};
use bytes::Bytes;
use config::ParserConfig;
use diesel::{
    r2d2::{ConnectionManager, Pool},
    PgConnection,
};
use google_cloud_storage::client::{Client as GCSClient, ClientConfig as GCSClientConfig};
use std::sync::Arc;
use tracing::{error, info, warn};
use worker::Worker;

pub mod config;
mod worker;

/// Struct to hold context required for parsing
#[derive(Clone)]
pub struct ParserContext {
    pub parser_config: Arc<ParserConfig>,
    pub pool: Pool<ConnectionManager<PgConnection>>,
    pub gcs_client: Arc<GCSClient>,
}

impl ParserContext {
    pub async fn new(
        parser_config: ParserConfig,
        pool: Pool<ConnectionManager<PgConnection>>,
    ) -> Self {
        if let Some(google_application_credentials) = &parser_config.google_application_credentials
        {
            info!(
                "[NFT Metadata Crawler] Google Application Credentials path found, setting env var"
            );
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

        Self {
            parser_config: Arc::new(parser_config),
            pool,
            gcs_client: Arc::new(GCSClient::new(gcs_config)),
        }
    }

    /// Repeatedly pulls workers from Channel and perform parsing operations
    async fn spawn_parser(&self, msg_base64: Bytes) {
        PARSER_INVOCATIONS_COUNT.inc();
        let pubsub_message = String::from_utf8(msg_base64.to_vec())
            .unwrap_or_else(|e| {
                error!(
                    error = ?e,
                    "[NFT Metadata Crawler] Failed to parse PubSub message"
                );
                panic!();
            })
            .replace('\u{0000}', "")
            .replace("\\u0000", "");

        info!(
            pubsub_message = pubsub_message,
            "[NFT Metadata Crawler] Received message from PubSub"
        );

        // Skips message if it does not have 5 commas (likely malformed URI)
        if pubsub_message.matches(',').count() != 5 {
            // Sends ack to PubSub only if ack_parsed_uris flag is true
            info!(
                pubsub_message = pubsub_message,
                "[NFT Metadata Crawler] Number of commans != 5, skipping message"
            );
            SKIP_URI_COUNT.with_label_values(&["invalid"]).inc();
            return;
        }

        // Parse PubSub message
        let parts: Vec<&str> = pubsub_message.split(',').collect();

        // Perform chain id check
        // If chain id is not set, set it
        let mut conn = self.pool.get().unwrap_or_else(|e| {
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
                pubsub_message = pubsub_message,
                error = ?e,
                "[NFT Metadata Crawler] Failed to parse chain id from PubSub message"
            );
            panic!();
        });

        // Panic if chain id of PubSub message does not match chain id in DB
        check_or_update_chain_id(&mut conn, grpc_chain_id as i64).expect("Chain id should match");

        // Spawn worker
        let last_transaction_version = parts[2].to_string().parse().unwrap_or_else(|e| {
            error!(
                pubsub_message = pubsub_message,
                error = ?e,
                "[NFT Metadata Crawler] Failed to parse last transaction version from PubSub message"
            );
            panic!();
        });

        let last_transaction_timestamp =
            chrono::NaiveDateTime::parse_from_str(parts[3], "%Y-%m-%d %H:%M:%S %Z").unwrap_or(
                chrono::NaiveDateTime::parse_from_str(parts[3], "%Y-%m-%d %H:%M:%S%.f %Z")
                    .unwrap_or_else(|e| {
                        error!(
                            pubsub_message = pubsub_message,
                            error = ?e,
                            "[NFT Metadata Crawler] Failed to parse timestamp from PubSub message"
                        );
                        panic!();
                    }),
            );

        let mut worker = Worker::new(
            self.parser_config.clone(),
            conn,
            self.parser_config.max_num_parse_retries,
            self.gcs_client.clone(),
            &pubsub_message,
            parts[0],
            parts[1],
            last_transaction_version,
            last_transaction_timestamp,
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
}

impl Server for ParserContext {
    fn build_router(&self) -> Router {
        let self_arc = Arc::new(self.clone());
        Router::new().route(
            "/",
            post(|bytes| async move {
                self_arc.spawn_parser(bytes).await;

                if !self_arc.parser_config.ack_parsed_uris {
                    return Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .body("".to_string())
                        .unwrap();
                }

                PUBSUB_ACK_SUCCESS_COUNT.inc();
                Response::builder()
                    .status(StatusCode::OK)
                    .body("".to_string())
                    .unwrap()
            }),
        )
    }
}
