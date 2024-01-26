// Copyright © Aptos Foundation

use crate::{
    utils::{
        counters::EVENT_RECEIVED_COUNT,
        database::{check_or_update_chain_id, establish_connection_pool, run_migrations},
        ingestor::Ingestor,
        stream::spawn_stream,
    },
    EventMessage,
};
use aptos_indexer_grpc_server_framework::RunnableConfig;
use bytes::Bytes;
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
}

/// Context required for event ingesetion
#[derive(Clone)]
pub struct IngestionContext {
    pub event_stream_config: EventStreamConfig,
    pub ingestor: Ingestor,
}

/// Handles PubSub ingestion from root endpoint
async fn handle_root(
    msg: Bytes,
    context: Arc<IngestionContext>,
) -> Result<impl warp::Reply, warp::Rejection> {
    EVENT_RECEIVED_COUNT.inc();
    context.ingestor.run(msg).await.unwrap_or_else(|e| {
        error!(
            error = ?e,
            "[Event Stream] Failed to run ingestor"
        );
        panic!();
    });

    let to_ack = context.event_stream_config.ack_parsed_uris.unwrap_or(false);
    if !to_ack {
        return Ok(warp::reply::with_status(
            warp::reply(),
            warp::http::StatusCode::BAD_REQUEST,
        ));
    }

    Ok(warp::reply::with_status(
        warp::reply(),
        warp::http::StatusCode::OK,
    ))
}

#[derive(Clone)]
pub struct StreamContext {
    pub channel: broadcast::Sender<EventMessage>,
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
        let pool = establish_connection_pool(self.database_url.clone());
        info!("[Event Stream] Database connection successful");

        info!("[Event Stream] Running migrations");
        run_migrations(&pool);
        info!("[Event Stream] Finished migrations");

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
        let (tx, _rx) = broadcast::channel::<EventMessage>(100);

        // Create web server
        let ingestion_context = Arc::new(IngestionContext {
            event_stream_config: self.clone(),
            ingestor: Ingestor::new(tx.clone(), self.chain_id, self.num_sec_valid.unwrap_or(30)),
        });

        let stream_context = Arc::new(StreamContext {
            channel: tx.clone(),
            websocket_alive_duration: self.websocket_alive_duration.unwrap_or(30),
        });

        let route = warp::post()
            .and(warp::path::end())
            .and(warp::body::bytes())
            .and(warp::any().map(move || ingestion_context.clone()))
            .and_then(handle_root);

        let ws_route = warp::path("stream")
            .and(warp::ws())
            .and(warp::any().map(move || stream_context.clone()))
            .and_then(handle_websocket);

        warp::serve(route.or(ws_route))
            .run(([0, 0, 0, 0], self.server_port))
            .await;
        Ok(())
    }

    fn get_server_name(&self) -> String {
        "event_stream".to_string()
    }
}
