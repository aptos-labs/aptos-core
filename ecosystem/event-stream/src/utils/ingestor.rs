// Copyright © Aptos Foundation

use crate::utils::{
    counters::{EVENT_RECEIVED_COUNT, PUBSUB_STREAM_RESET_COUNT},
    event_message::{PubSubEventMessage, StreamEventMessage},
};
use chrono::Duration;
use futures::StreamExt;
use google_cloud_pubsub::subscription::{MessageStream, Subscription};
use tokio::sync::broadcast;
use tracing::{error, info, warn};

#[derive(Clone)]
pub struct Ingestor {
    channel: broadcast::Sender<StreamEventMessage>,
    chain_id: i64,
    num_sec_valid: i64,
    subscription: Subscription,
}

impl Ingestor {
    pub fn new(
        channel: broadcast::Sender<StreamEventMessage>,
        chain_id: i64,
        num_sec_valid: i64,
        subscription: Subscription,
    ) -> Self {
        Self {
            channel,
            chain_id,
            num_sec_valid,
            subscription,
        }
    }

    pub async fn run(&self) {
        let mut stream = self.get_new_subscription_stream().await;
        while let Some(msg) = stream.next().await {
            let pubsub_message = String::from_utf8(msg.message.data.to_vec()).unwrap_or_else(|e| {
                error!(
                    error = ?e,
                    "[Event Stream] Failed to decode PubSub message"
                );
                panic!();
            });

            info!(
                pubsub_message = pubsub_message.to_string(),
                "[Event Stream] Received message from PubSub"
            );
            EVENT_RECEIVED_COUNT.inc();

            let pubsub_message = self
                .parse_pubsub_message(&pubsub_message)
                .unwrap_or_else(|e| {
                    error!(
                        error = ?e,
                        "[Event Stream] Failed to parse PubSub message"
                    );
                    panic!();
                });

            if let Err(e) = msg.ack().await {
                warn!(
                    pubsub_message = pubsub_message.to_string(),
                    error = ?e,
                    "[Event Stream] Resetting stream"
                );
                stream = self.get_new_subscription_stream().await;
                continue;
            }

            if let Err(e) = self.validate_pubsub_message(&pubsub_message) {
                warn!(
                    pubsub_message = pubsub_message.to_string(),
                    error = ?e,
                    "[Event Stream] Failed to validate message"
                );
                continue;
            }

            let stream_messages = StreamEventMessage::list_from_pubsub(&pubsub_message);
            for stream_message in stream_messages {
                self.channel
                    .send(stream_message.clone())
                    .unwrap_or_else(|e| {
                        error!(
                            pubsub_message = pubsub_message.to_string(),
                            stream_message = stream_message.to_string(),
                            error = ?e,
                            "[Event Stream] Failed to broadcast message"
                        );
                        panic!();
                    });
                info!(
                    stream_message = stream_message.to_string(),
                    "[Event Stream] Broadcasted message"
                );
            }
        }
    }

    /// Returns a new stream from a PubSub subscription
    async fn get_new_subscription_stream(&self) -> MessageStream {
        PUBSUB_STREAM_RESET_COUNT.inc();
        self.subscription.subscribe(None).await.unwrap_or_else(|e| {
            error!(
                error = ?e,
                "[Event Stream] Failed to get stream from PubSub subscription"
            );
            panic!();
        })
    }

    fn validate_pubsub_message(&self, event_message: &PubSubEventMessage) -> anyhow::Result<()> {
        if event_message.chain_id != self.chain_id {
            error!(
                chain_id = event_message.chain_id,
                expected_chain_id = self.chain_id,
                pubsub_message = event_message.to_string(),
                "[Event Stream] Chain ID mismatch"
            );
            panic!();
        }

        let now = chrono::Utc::now().naive_utc();
        let event_time = chrono::NaiveDateTime::parse_from_str(
            &event_message.timestamp,
            "%Y-%m-%d %H:%M:%S %Z",
        )?;

        let duration = event_time.signed_duration_since(now);
        if duration < Duration::seconds(-self.num_sec_valid) {
            return Err(anyhow::anyhow!(
                "Event timestamp is too far in the past: {}",
                event_message.timestamp
            ));
        }

        Ok(())
    }

    fn parse_pubsub_message(&self, pubsub_message: &str) -> anyhow::Result<PubSubEventMessage> {
        let event_message = serde_json::from_str::<PubSubEventMessage>(pubsub_message)?;
        Ok(event_message)
    }
}
