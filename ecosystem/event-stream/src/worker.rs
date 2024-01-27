// Copyright © Aptos Foundation

use crate::utils::{
    database::{check_or_update_chain_id, establish_connection_pool, run_migrations},
    event_message::StreamEventMessage,
    ingestor::Ingestor,
    stream::spawn_stream,
};
use aptos_indexer_grpc_server_framework::RunnableConfig;
use google_cloud_pubsub::client::{Client, ClientConfig};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{error, info};
use warp::Filter;

/// Config from Event Stream YAML
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EventStreamConfig {
    pub database_url: String,
    pub ack_parsed_uris: Option<bool>,
    pub server_port: u16,
    pub chain_id: i64,
    pub num_sec_valid: Option<i64>,
    pub websocket_alive_duration: Option<u64>,
    pub google_application_credentials: Option<String>,
    pub subscription_name: String,
}

#[derive(Clone)]
pub struct StreamContext {
    pub channel: broadcast::Sender<StreamEventMessage>,
    pub websocket_alive_duration: u64,
}

/// Handles WebSocket connection from /stream endpoint
async fn handle_websocket(
    websocket: warp::ws::Ws,
    context: Arc<StreamContext>,
) -> Result<impl warp::Reply, warp::Rejection> {
    Ok(websocket.on_upgrade(move |ws| {
        spawn_stream(
            ws,
            context.channel.clone(),
            context.websocket_alive_duration,
        )
    }))
}

#[async_trait::async_trait]
impl RunnableConfig for EventStreamConfig {
    async fn run(&self) -> anyhow::Result<()> {
        info!(
            "[Event Stream] Starting event stream with config: {:?}",
            self
        );

        info!("[Event Stream] Connecting to database");
        let pool = establish_connection_pool(&self.database_url);
        info!("[Event Stream] Database connection successful");

        info!("[Event Stream] Running migrations");
        run_migrations(&pool);
        info!("[Event Stream] Finished migrations");

        if let Some(google_application_credentials) = &self.google_application_credentials {
            std::env::set_var(
                "GOOGLE_APPLICATION_CREDENTIALS",
                google_application_credentials,
            );
        }

        // Establish PubSub client
        let pubsub_config = ClientConfig::default()
            .with_auth()
            .await
            .unwrap_or_else(|e| {
                error!(
                    error = ?e,
                    "[Event Stream] Failed to create PubSub client config"
                );
                panic!();
            });
        let pubsub_client = Client::new(pubsub_config).await.unwrap_or_else(|e| {
            error!(
                error = ?e,
                "[Event Stream] Failed to create PubSub client"
            );
            panic!();
        });
        let subscription = pubsub_client.subscription(&self.subscription_name);

        // Panic if chain id of config does not match chain id of database
        // Perform chain id check
        // If chain id is not set, set it
        info!("[Event Stream] Checking if chain id is correct");
        let mut conn = pool.get().unwrap_or_else(|e| {
            error!(
                error = ?e,
                "[Event Stream] Failed to get DB connection from pool");
            panic!();
        });
        check_or_update_chain_id(&mut conn, self.chain_id).unwrap_or_else(|e| {
            error!(error = ?e, "[Event Stream] Chain id should match");
            panic!();
        });
        info!(
            chain_id = self.chain_id,
            "[Event Stream] Chain id matches! Continue to stream...",
        );

        // Create Event broadcast channel
        let (tx, _rx) = broadcast::channel::<StreamEventMessage>(100);

        // Create and start ingestor
        let ingestor = Ingestor::new(
            tx.clone(),
            self.chain_id,
            self.num_sec_valid.unwrap_or(30),
            subscription,
        );
        tokio::spawn(async move {
            ingestor.run().await;
        });

        // Create web server
        let stream_context = Arc::new(StreamContext {
            channel: tx,
            websocket_alive_duration: self.websocket_alive_duration.unwrap_or(30),
        });

        let ws_route = warp::path("stream")
            .and(warp::ws())
            .and(warp::any().map(move || stream_context.clone()))
            .and_then(handle_websocket);

        warp::serve(ws_route)
            .run(([0, 0, 0, 0], self.server_port))
            .await;
        Ok(())
    }

    fn get_server_name(&self) -> String {
        "event_stream".to_string()
    }
}
