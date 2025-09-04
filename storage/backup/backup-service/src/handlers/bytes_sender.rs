// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::handlers::utils::THROUGHPUT_COUNTER;
use velor_metrics_core::IntCounterVecHelper;
use velor_storage_interface::{VelorDbError, Result as DbResult};
use bytes::{BufMut, Bytes, BytesMut};
use serde::Serialize;

type BoxError = Box<dyn std::error::Error + Send + Sync>;
type BytesResult = Result<Bytes, BoxError>;

pub(super) struct BytesSender {
    /// Buffers bytes instead of relying on the channel's backlog to provide backpressure, so
    /// the max pending bytes are more predictable.
    buffer: BytesMut,
    bytes_tx: tokio::sync::mpsc::Sender<BytesResult>,
    endpoint: &'static str,
}

impl BytesSender {
    const MAX_BATCHES: usize = 100;
    #[cfg(not(test))]
    const TARGET_BATCH_SIZE: usize = 10 * 1024;
    #[cfg(test)]
    const TARGET_BATCH_SIZE: usize = 10;

    pub fn new(
        endpoint: &'static str,
    ) -> (Self, tokio_stream::wrappers::ReceiverStream<BytesResult>) {
        let (bytes_tx, bytes_rx) = tokio::sync::mpsc::channel(Self::MAX_BATCHES);

        let myself = Self {
            buffer: BytesMut::new(),
            bytes_tx,
            endpoint,
        };

        let stream = tokio_stream::wrappers::ReceiverStream::new(bytes_rx);

        (myself, stream)
    }

    pub fn send_bytes(&mut self, bytes: Bytes) -> DbResult<()> {
        self.buffer.extend(bytes);

        if self.buffer.len() >= Self::TARGET_BATCH_SIZE {
            self.flush_buffer()?
        }

        Ok(())
    }

    pub fn send_size_prefixed_bcs_bytes<Record: Serialize>(
        &mut self,
        record: Record,
    ) -> DbResult<()> {
        let record_bytes = bcs::to_bytes(&record)?;
        let size_bytes = (record_bytes.len() as u32).to_be_bytes();

        let mut buf = BytesMut::with_capacity(size_bytes.len() + record_bytes.len());
        buf.put_slice(&size_bytes);
        buf.extend(record_bytes);

        self.send_bytes(buf.freeze())
    }

    pub fn flush_buffer(&mut self) -> DbResult<()> {
        let bytes = self.buffer.split().freeze();
        THROUGHPUT_COUNTER.inc_with_by(&[self.endpoint], bytes.len() as u64);

        self.send_res(Ok(bytes))
    }

    pub fn finish(mut self) -> DbResult<()> {
        self.flush_buffer()
    }

    pub fn abort<E: std::error::Error + Send + Sync + 'static>(self, err: E) -> DbResult<()> {
        self.send_res(Err(Box::new(err)))
    }

    pub fn send_res(&self, item: BytesResult) -> DbResult<()> {
        self.bytes_tx
            .blocking_send(item)
            .map_err(|e| VelorDbError::Other(format!("Failed to send to response stream. {e}")))
    }
}
