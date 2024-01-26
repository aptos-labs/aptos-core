// Copyright © Aptos Foundation

use bytes::Bytes;
use chrono::Duration;
use tokio::sync::broadcast;
use tracing::{error, info, warn};

use crate::EventMessage;

#[derive(Clone)]
pub struct Ingestor {
    channel: broadcast::Sender<EventMessage>,
    chain_id: i64,
    num_sec_valid: i64,
}

impl Ingestor {
    pub fn new(
        channel: broadcast::Sender<EventMessage>,
        chain_id: i64,
        num_sec_valid: i64,
    ) -> Self {
        Self {
            channel,
            chain_id,
            num_sec_valid,
        }
    }

    pub async fn run(&self, msg_base64: Bytes) -> anyhow::Result<()> {
        let pubsub_message = self.parse_pubsub_message(msg_base64)?;
        info!(
            pubsub_message = pubsub_message.to_string(),
            "[Event Stream] Received message from PubSub"
        );

        if let Err(e) = self.validate_pubsub_message(&pubsub_message) {
            warn!(
                pubsub_message = pubsub_message.to_string(),
                error = ?e,
                "[Event Stream] Failed to validate message"
            );
            return Ok(());
        }

        self.channel
            .send(pubsub_message.clone())
            .unwrap_or_else(|e| {
                error!(
                    pubsub_message = pubsub_message.to_string(),
                    error = ?e,
                    "[Event Stream] Failed to broadcast message"
                );
                panic!();
            });

        info!(
            pubsub_message = pubsub_message.to_string(),
            "[Event Stream] Broadcasted message"
        );
        Ok(())
    }

    fn validate_pubsub_message(&self, event_message: &EventMessage) -> anyhow::Result<()> {
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

    fn parse_pubsub_message(&self, msg_base64: Bytes) -> anyhow::Result<EventMessage> {
        let pubsub_message = String::from_utf8(msg_base64.to_vec())?;
        let event_message = serde_json::from_str::<EventMessage>(&pubsub_message)?;
        Ok(event_message)
    }
}
