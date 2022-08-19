// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::increment_log_ingest_too_large_by;
use crate::sender::TelemetrySender;
use aptos_config::config::NodeConfig;
use aptos_logger::prelude::*;
use aptos_types::chain_id::ChainId;
use futures::channel::mpsc;
use futures::StreamExt;
use std::time::{Duration, Instant};

/// Buffered
pub(crate) struct TelemetryLogSender {
    sender: TelemetrySender,
    batch: Vec<String>,
    max_bytes: usize,
    current_bytes: usize,
    max_batch_time: Duration,
    batch_start_time: Instant,
}

impl TelemetryLogSender {
    pub fn new(base_url: &str, chain_id: ChainId, node_config: &NodeConfig) -> Self {
        Self {
            sender: TelemetrySender::new(base_url, chain_id, node_config),
            batch: Vec::new(),
            max_bytes: 128 * 1024,
            current_bytes: 0,
            max_batch_time: Duration::from_secs(1),
            batch_start_time: Instant::now(),
        }
    }

    pub async fn handle_next_log(&mut self, log: String) {
        if log.len() > self.max_bytes {
            warn!("Log ignored, size: {}", log.len());
            increment_log_ingest_too_large_by(1);
            return;
        }

        self.current_bytes += log.len();
        self.batch.push(log);

        if self.current_bytes > self.max_bytes
            || self.batch_start_time.elapsed() > self.max_batch_time
        {
            let batch: Vec<_> = self.batch.drain(..).collect();
            self.current_bytes = 0;
            self.batch_start_time = Instant::now();

            self.sender.send_logs(batch).await;
        }
    }

    pub async fn start(mut self, mut rx: mpsc::Receiver<String>) {
        loop {
            tokio::select! {
                Some(log) = rx.next() => {
                    self.handle_next_log(log).await;
                }
            }
        }
    }
}
