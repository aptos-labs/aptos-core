// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::increment_log_ingest_too_large_by;
use crate::sender::TelemetrySender;
use aptos_logger::prelude::*;
use aptos_logger::telemetry_log_writer::TelemetryLog;
use futures::channel::mpsc;
use futures::StreamExt;
use std::time::Duration;
use tokio::time::interval;
use tokio_stream::wrappers::IntervalStream;

const MAX_BYTES: usize = 128 * 1024;
const MAX_BATCH_TIME: Duration = Duration::from_secs(5);

/// Buffered
pub(crate) struct TelemetryLogSender {
    sender: TelemetrySender,
    batch: Vec<String>,
    max_bytes: usize,
    current_bytes: usize,
}

impl TelemetryLogSender {
    pub fn new(sender: TelemetrySender) -> Self {
        Self {
            // TODO: use an existing sender?
            sender,
            batch: Vec::new(),
            max_bytes: MAX_BYTES,
            current_bytes: 0,
        }
    }

    fn drain_batch(&mut self) -> Vec<String> {
        let batch: Vec<_> = self.batch.drain(..).collect();
        self.current_bytes = 0;
        batch
    }

    pub(crate) fn add_to_batch(&mut self, log: String) -> Option<Vec<String>> {
        if log.len() > self.max_bytes {
            warn!("Log ignored, size: {}", log.len());
            increment_log_ingest_too_large_by(1);
            return None;
        }

        self.current_bytes += log.len();
        self.batch.push(log);

        if self.current_bytes > self.max_bytes {
            return Some(self.drain_batch());
        }
        None
    }

    pub async fn handle_next_log(&mut self, log: TelemetryLog) {
        match log {
            TelemetryLog::Log(log) => {
                if let Some(batch) = self.add_to_batch(log) {
                    self.sender.try_send_logs(batch).await;
                }
            }
            TelemetryLog::Flush(tx) => {
                self.flush_batch().await;
                let _ = tx.send(());
            }
        }
    }

    pub async fn flush_batch(&mut self) {
        if !self.batch.is_empty() {
            let drained = self.drain_batch();
            self.sender.try_send_logs(drained).await;
        }
    }

    pub async fn start(mut self, mut rx: mpsc::Receiver<TelemetryLog>) {
        debug!("Started Telemetry Log Sender");
        let mut interval = IntervalStream::new(interval(MAX_BATCH_TIME)).fuse();

        loop {
            ::futures::select! {
                log = rx.select_next_some() => {
                    self.handle_next_log(log).await;
                },
                _ = interval.select_next_some() => {
                    self.flush_batch().await;
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::sender::TelemetrySender;
    use crate::telemetry_log_sender::{TelemetryLogSender, MAX_BYTES};
    use aptos_config::config::NodeConfig;
    use aptos_types::chain_id::ChainId;

    #[tokio::test]
    async fn test_add_to_batch() {
        let telemetry_sender =
            TelemetrySender::new("test".to_string(), ChainId::test(), &NodeConfig::default());
        let mut sender = TelemetryLogSender::new(telemetry_sender);

        for _i in 0..2 {
            // Large batch should not be allowed
            let batch = sender.add_to_batch("a".repeat(MAX_BYTES + 1));
            assert!(batch.is_none());

            // Batch is flushed before reaching size
            let to_send = vec!["test"];
            let batch = sender.add_to_batch(to_send[0].to_string());
            assert!(batch.is_none());
            let batch = sender.drain_batch();
            assert_eq!(batch.len(), 1);
            assert_eq!(batch, to_send);

            // Create batch that reaches max bytes
            let bytes_per_string = 11;
            let num_strings = (MAX_BYTES + 1) / bytes_per_string
                + if (MAX_BYTES + 1) % bytes_per_string == 0 {
                    0
                } else {
                    1
                };
            let to_send: Vec<_> = (0..num_strings).map(|i| format!("{:11}", i)).collect();
            to_send.iter().enumerate().for_each(|(i, s)| {
                // Large batch should not be allowed
                let batch = sender.add_to_batch("a".repeat(MAX_BYTES + 1));
                assert!(batch.is_none());

                let batch = sender.add_to_batch(s.clone());
                if i == (num_strings - 1) {
                    assert!(batch.is_some());
                    assert_eq!(batch.unwrap(), to_send);
                } else {
                    assert!(batch.is_none());
                }
            })
        }
    }
}
