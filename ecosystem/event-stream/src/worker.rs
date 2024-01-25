// Copyright © Aptos Foundation

use crate::utils::{
    counters::{
        EVENT_RECEIVED_COUNT, GOT_CONNECTION_COUNT, PUBSUB_ACK_SUCCESS_COUNT,
        UNABLE_TO_GET_CONNECTION_COUNT,
    },
    database::{check_or_update_chain_id, establish_connection_pool, run_migrations},
};
use aptos_indexer_grpc_server_framework::RunnableConfig;
use bytes::Bytes;
use diesel::{
    r2d2::{ConnectionManager, Pool},
    PgConnection,
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{error, info};
use warp::{filters::ws::WebSocket, Filter};

/// Structs to hold config from YAML
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EventStreamConfig {
    pub database_url: String,
    pub ack_parsed_uris: Option<bool>,
    pub server_port: u16,
}

/// Struct to hold context required for event ingesetion
#[derive(Clone)]
pub struct IngestionContext {
    pub channel: broadcast::Sender<String>,
    pub event_stream_config: EventStreamConfig,
    pub pool: Pool<ConnectionManager<PgConnection>>,
}

/// Performs validation checks and adds event to channel
async fn spawn_ingestor(
    channel: broadcast::Sender<String>,
    msg_base64: Bytes,
    pool: Pool<ConnectionManager<PgConnection>>,
) {
    EVENT_RECEIVED_COUNT.inc();
    let pubsub_message = String::from_utf8(msg_base64.to_vec()).unwrap_or_else(|e| {
        error!(
            error = ?e,
            "[Event Stream] Failed to parse PubSub message"
        );
        panic!();
    });

    info!(
        pubsub_message = pubsub_message,
        "[Event Stream] Received message from PubSub"
    );

    // Perform chain id check
    // If chain id is not set, set it
    let mut conn = pool.get().unwrap_or_else(|e| {
        error!(
                pubsub_message = pubsub_message,
                error = ?e,
                "[Event Stream] Failed to get DB connection from pool");
        UNABLE_TO_GET_CONNECTION_COUNT.inc();
        panic!();
    });
    GOT_CONNECTION_COUNT.inc();

    let grpc_chain_id = 3; // TODO: Get chain id from PubSub message

    // Panic if chain id of PubSub message does not match chain id in DB
    check_or_update_chain_id(&mut conn, grpc_chain_id as i64).expect("Chain id should match");

    // TODO: Broadcast message
    channel.send(pubsub_message.clone()).unwrap_or_else(|e| {
        error!(
            pubsub_message = pubsub_message,
            error = ?e,
            "[Event Stream] Failed to broadcast message"
        );
        panic!();
    });

    info!(
        pubsub_message = pubsub_message,
        "[Event Stream] Worker finished"
    );
}

/// Handles ingestion from the root endpoint
async fn handle_root(
    msg: Bytes,
    context: Arc<IngestionContext>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let to_ack = context.event_stream_config.ack_parsed_uris.unwrap_or(false);

    // Use spawn_blocking to run the function on a separate thread.
    let _ = tokio::spawn(spawn_ingestor(
        context.channel.clone(),
        msg,
        context.pool.clone(),
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

/// Maintains websocket connection and sends messages from channel
async fn spawn_stream(ws: WebSocket, channel: Arc<broadcast::Sender<String>>) {
    let (mut tx, _) = ws.split();
    let mut channel = channel.subscribe();

    loop {
        let msg = channel.recv().await.unwrap_or_else(|e| {
            error!(
                error = ?e,
                "[Event Stream] Failed to receive message from channel"
            );
            panic!();
        });
        tx.send(warp::ws::Message::text(msg)).await.unwrap();
    }
}

/// Handles websocket connection from stream endpoint
async fn handle_websocket(
    websocket: warp::ws::Ws,
    channel: Arc<broadcast::Sender<String>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    Ok(websocket.on_upgrade(move |ws| spawn_stream(ws, channel.clone())))
}

#[async_trait::async_trait]
impl RunnableConfig for EventStreamConfig {
    /// Main driver function that establishes a connection to Pubsub and parses the Pubsub entries in parallel
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

        // Create event channel
        let (tx, _) = broadcast::channel::<String>(100);

        // Create request context
        let context = Arc::new(IngestionContext {
            channel: tx.clone(),
            event_stream_config: self.clone(),
            pool,
        });

        let tx = Arc::new(tx);

        // Create web server
        let route = warp::post()
            .and(warp::path::end())
            .and(warp::body::bytes())
            .and(warp::any().map(move || context.clone()))
            .and_then(handle_root);

        let ws_route = warp::path("stream")
            .and(warp::ws())
            .and(warp::any().map(move || tx.clone()))
            .and_then(handle_websocket);

        warp::serve(route.or(ws_route))
            .run(([0, 0, 0, 0], self.server_port))
            .await;
        Ok(())
    }

    fn get_server_name(&self) -> String {
        "parser".to_string()
    }
}
