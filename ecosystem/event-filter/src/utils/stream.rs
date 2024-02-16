// Copyright © Aptos Foundation

use crate::utils::{filter::EventFilter, EventModel};
use futures::{stream::SplitSink, SinkExt};
use std::{sync::Arc, time::Duration};
use tokio::{
    sync::{broadcast, Mutex},
    time,
};
use tracing::{error, info};
use warp::filters::ws::{Message, WebSocket};

pub struct Stream {
    tx: SplitSink<WebSocket, Message>,
    filter: Arc<Mutex<EventFilter>>,
    channel: broadcast::Receiver<EventModel>,
    websocket_alive_duration: u64,
}

impl Stream {
    pub fn new(
        tx: SplitSink<WebSocket, Message>,
        filter: Arc<Mutex<EventFilter>>,
        channel: broadcast::Receiver<EventModel>,
        websocket_alive_duration: u64,
    ) -> Self {
        info!("Received WebSocket connection");
        Self {
            tx,
            filter,
            channel,
            websocket_alive_duration,
        }
    }

    /// Maintains websocket connection and sends messages from channel
    pub async fn run(&mut self) {
        let sleep = time::sleep(Duration::from_secs(self.websocket_alive_duration));
        tokio::pin!(sleep);

        loop {
            tokio::select! {
                event = self.channel.recv() => {
                    let event = event.unwrap_or_else(|e| {
                        error!(
                            error = ?e,
                            "[Event Stream] Failed to receive message from channel"
                        );
                        panic!();
                    });

                    let filter = self.filter.lock().await;
                    if filter.accounts.contains(&event.account_address) || filter.types.contains(&event.type_) {
                        self.tx
                            .send(warp::ws::Message::text(serde_json::to_string(&event).unwrap_or_default()))
                            .await
                            .unwrap();
                    }
                },
                _ = &mut sleep => {
                    break;
                }
            }
        }
    }
}

pub async fn spawn_stream(
    tx: SplitSink<WebSocket, Message>,
    filter: Arc<Mutex<EventFilter>>,
    channel: broadcast::Receiver<EventModel>,
    websocket_alive_duration: u64,
) {
    let mut stream = Stream::new(tx, filter, channel, websocket_alive_duration);
    stream.run().await;
}
