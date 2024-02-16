// Copyright © Aptos Foundation

use crate::utils::filter::EventFilter;
use futures::{stream::SplitStream, StreamExt};
use std::{sync::Arc, time::Duration};
use tokio::{sync::Mutex, time};
use tracing::{error, info};
use warp::filters::ws::WebSocket;

pub struct FilterEditor {
    rx: SplitStream<WebSocket>,
    filter: Arc<Mutex<EventFilter>>,
    websocket_alive_duration: u64,
}

impl FilterEditor {
    pub fn new(
        rx: SplitStream<WebSocket>,
        filter: Arc<Mutex<EventFilter>>,
        websocket_alive_duration: u64,
    ) -> Self {
        info!("Received WebSocket connection");
        Self {
            rx,
            filter,
            websocket_alive_duration,
        }
    }

    /// Maintains websocket connection and sends messages from channel
    pub async fn run(&mut self) {
        let sleep = time::sleep(Duration::from_secs(self.websocket_alive_duration));
        tokio::pin!(sleep);

        loop {
            tokio::select! {
                msg = self.rx.next() => {
                    if let Some(msg) = msg {
                        let mut filter = self.filter.lock().await;
                        let policy = msg.unwrap_or_else(|e| {
                            error!(
                                error = ?e,
                                "[Event Stream] Failed to receive message from channel"
                            );
                            panic!();
                        });
                        let policy = policy.to_str().unwrap_or_else(|e|{
                            error!(
                                error = ?e,
                                "[Event Stream] Failed to convert message to string"
                            );
                            panic!();
                        }).split(",").collect::<Vec<&str>>();
                        match policy[0] {
                            "account" => {
                                match policy[1] {
                                    "add" => {
                                        filter.accounts.insert(policy[2].to_string());
                                    }
                                    "remove" => {
                                        filter.accounts.remove(policy[2]);
                                    }
                                    _ => {
                                        error!(
                                            "[Event Stream] Invalid filter command: {}",
                                            policy[1]
                                        );
                                    }
                                }
                            }
                            "type" => {
                                match policy[1] {
                                    "add" => {
                                        filter.types.insert(policy[2].to_string());
                                    }
                                    "remove" => {
                                        filter.types.remove(policy[2]);
                                    }
                                    _ => {
                                        error!(
                                            "[Event Stream] Invalid filter command: {}",
                                            policy[1]
                                        );
                                    }
                                }
                            }
                            _ => {
                                error!(
                                    "[Event Stream] Invalid filter type: {}",
                                    policy[0]
                                );
                            }
                        }
                    }
                },
                _ = &mut sleep => {
                    break;
                }
            }
        }
    }
}

pub async fn spawn_filter_editor(
    rx: SplitStream<WebSocket>,
    filter: Arc<Mutex<EventFilter>>,
    websocket_alive_duration: u64,
) {
    let mut filter = FilterEditor::new(rx, filter, websocket_alive_duration);
    filter.run().await;
}
