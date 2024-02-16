// Copyright © Aptos Foundation

use crate::utils::{
    filter::EventFilter, filter_editor::spawn_filter_editor, stream::spawn_stream, EventModel,
};
use aptos_indexer_grpc_server_framework::RunnableConfig;
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use tokio_tungstenite::connect_async;
use tracing::{error, info};
use url::Url;
use warp::Filter;

/// Config from Event Stream YAML
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EventFilterConfig {
    pub server_port: u16,
    pub num_sec_valid: Option<i64>,
    pub websocket_alive_duration: Option<u64>,
}

#[derive(Clone)]
pub struct FilterContext {
    pub channel: broadcast::Sender<EventModel>,
    pub websocket_alive_duration: u64,
}

/// Handles WebSocket connection from /filter endpoint
async fn handle_websocket(
    websocket: warp::ws::Ws,
    context: Arc<FilterContext>,
) -> Result<impl warp::Reply, warp::Rejection> {
    Ok(websocket.on_upgrade(move |ws| {
        let (tx, rx) = ws.split();
        let filter = Arc::new(Mutex::new(EventFilter::new()));
        let websocket_alive_duration = context.websocket_alive_duration;

        let filter_edit = filter.clone();
        tokio::spawn(async move {
            spawn_filter_editor(rx, filter_edit, websocket_alive_duration).await
        });

        spawn_stream(
            tx,
            filter.clone(),
            context.channel.subscribe(),
            websocket_alive_duration,
        )
    }))
}

#[async_trait::async_trait]
impl RunnableConfig for EventFilterConfig {
    async fn run(&self) -> anyhow::Result<()> {
        info!(
            "[Event Filter] Starting event stream with config: {:?}",
            self
        );

        // Create Event broadcast channel
        // Can use channel size to help with pubsub lagging
        let (broadcast_tx, mut broadcast_rx) = broadcast::channel(10000);

        // Receive all messages with initial Receiver to keep channel open
        tokio::spawn(async move {
            loop {
                broadcast_rx.recv().await.unwrap_or_else(|e| {
                    error!(
                        error = ?e,
                        "[Event Filter] Failed to receive message from channel"
                    );
                    panic!();
                });
            }
        });

        // Create and start ingestor
        let broadcast_tx_write = broadcast_tx.clone();
        tokio::spawn(async move {
            let url =
                Url::parse("ws://localhost:8081/stream").expect("Failed to parse WebSocket URL");

            let (ws_stream, _) = connect_async(url)
                .await
                .expect("Failed to connect to WebSocket");

            let (_, mut read) = ws_stream.split();

            while let Some(message) = read.next().await {
                match message {
                    Ok(msg) => {
                        broadcast_tx_write
                            .send(serde_json::from_str::<EventModel>(&msg.to_string()).unwrap())
                            .unwrap();
                    },
                    Err(e) => {
                        eprintln!("Error receiving message: {}", e);
                        break;
                    },
                }
            }
        });

        // Create web server
        let stream_context = Arc::new(FilterContext {
            channel: broadcast_tx,
            websocket_alive_duration: self.websocket_alive_duration.unwrap_or(30),
        });

        let ws_route = warp::path("filter")
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
