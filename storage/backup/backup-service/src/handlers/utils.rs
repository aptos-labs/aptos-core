// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_db::backup::backup_handler::BackupHandler;
use aptos_logger::prelude::*;
use aptos_metrics_core::{
    register_histogram_vec, register_int_counter_vec, HistogramVec, IntCounterHelper, IntCounterVec,
};
use aptos_storage_interface::{AptosDbError, Result};
use bytes::{BufMut, Bytes, BytesMut};
use hyper::Body;
use once_cell::sync::Lazy;
use serde::Serialize;
use std::{convert::Infallible, future::Future};
use warp::{reply::Response, Rejection, Reply};

pub(super) static LATENCY_HISTOGRAM: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_backup_service_latency_s",
        "Backup service endpoint latency.",
        &["endpoint", "status"]
    )
    .unwrap()
});

pub(super) static THROUGHPUT_COUNTER: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_backup_service_sent_bytes",
        "Backup service throughput in bytes.",
        &["endpoint"]
    )
    .unwrap()
});

pub(super) fn reply_with_bcs_bytes<R: Serialize>(
    endpoint: &str,
    record: &R,
) -> Result<Box<dyn Reply>> {
    let bytes = bcs::to_bytes(record)?;
    THROUGHPUT_COUNTER
        .with_label_values(&[endpoint])
        .inc_by(bytes.len() as u64);
    Ok(Box::new(bytes))
}

#[must_use]
pub(super) struct BytesSender {
    buffer: BytesMut,
    batch_tx: Option<tokio::sync::mpsc::Sender<Bytes>>,
    sender_task: Option<tokio::task::JoinHandle<(hyper::body::Sender, Result<()>)>>,
}

impl BytesSender {
    fn new(endpoint: &'static str, mut inner: hyper::body::Sender) -> Self {
        let (batch_tx, mut batch_rx) = tokio::sync::mpsc::channel::<Bytes>(100);

        let sender_task = tokio::spawn(async move {
            let res = async {
                while let Some(batch) = Self::recv_some(&mut batch_rx).await {
                    let n_bytes = batch.len();

                    inner.send_data(batch).await.into_db_res()?;

                    THROUGHPUT_COUNTER.inc_with_by(&[endpoint], n_bytes as u64);
                }
                Ok(())
            }
            .await;

            (inner, res)
        });

        Self {
            buffer: BytesMut::new(),
            batch_tx: Some(batch_tx),
            sender_task: Some(sender_task),
        }
    }

    async fn recv_some(rx: &mut tokio::sync::mpsc::Receiver<Bytes>) -> Option<Bytes> {
        let mut buf = BytesMut::new();

        while let Ok(bytes) = rx.try_recv() {
            buf.put(bytes);
        }

        if buf.is_empty() {
            rx.recv().await
        } else {
            Some(buf.freeze())
        }
    }

    async fn flush_buffer(&mut self) -> Result<()> {
        self.batch_tx
            .as_mut()
            .expect("Batch sender gone.")
            .send(self.buffer.split().freeze())
            .await
            .into_db_res()
    }

    async fn send_data(&mut self, bytes: &[u8]) -> Result<()> {
        let sender_task = self.sender_task.as_ref().expect("Sender task gone.");

        if sender_task.is_finished() {
            return Err(AptosDbError::Other(
                "Sender task finished unexpectedly.".to_string(),
            ));
        }

        self.buffer.put_slice(bytes);

        const TARGET_BATCH_SIZE: usize = if cfg!(test) { 10 } else { 1024 * 1024 };
        if self.buffer.len() >= TARGET_BATCH_SIZE {
            self.flush_buffer().await?
        }

        Ok(())
    }

    async fn finish_impl(&mut self, abort: bool) -> Result<()> {
        let mut ret = Ok(());

        if !abort {
            ret = self.flush_buffer().await
        }

        // drop sender to inform the sending task to quit
        self.batch_tx.take().unwrap();

        let (inner, res) = self
            .sender_task
            .take()
            .unwrap()
            .await
            .expect("Sender task panicked.");

        ret = ret.and(res);

        if abort || ret.is_err() {
            inner.abort();
        }

        ret
    }

    async fn finish(mut self) -> Result<()> {
        self.finish_impl(false).await
    }

    async fn abort(mut self) {
        // ignore error
        let _ = self.finish_impl(true).await;
    }
}

pub(super) fn reply_with_async_channel_writer<G, F>(
    backup_handler: &BackupHandler,
    endpoint: &'static str,
    get_channel_writer: G,
) -> Box<dyn Reply>
where
    G: FnOnce(BackupHandler, BytesSender) -> F,
    F: Future<Output = ()> + Send + 'static,
{
    let (sender, body) = Body::channel();
    let sender = BytesSender::new(endpoint, sender);
    let bh = backup_handler.clone();
    tokio::spawn(get_channel_writer(bh, sender));

    Box::new(Response::new(body))
}

pub(super) async fn send_size_prefixed_bcs_bytes<I, R>(iter_res: Result<I>, mut sender: BytesSender)
where
    I: Iterator<Item = Result<R>>,
    R: Serialize,
{
    match send_size_prefixed_bcs_bytes_impl(iter_res, &mut sender).await {
        Ok(()) => {
            if let Err(e) = sender.finish().await {
                warn!("Failed to finish http body: {:?}", e);
            }
        },
        Err(e) => {
            warn!("Failed writing http body: {:?}", e);
            sender.abort().await;
        },
    }
}

async fn send_size_prefixed_bcs_bytes_impl<I, R>(
    iter_res: Result<I>,
    sender: &mut BytesSender,
) -> Result<()>
where
    I: Iterator<Item = Result<R>>,
    R: Serialize,
{
    for record_res in iter_res? {
        let record = record_res?;
        let record_bytes = bcs::to_bytes(&record)?;
        let size_bytes = (record_bytes.len() as u32).to_be_bytes();

        sender.send_data(&size_bytes).await?;
        sender.send_data(&record_bytes).await?;
    }

    Ok(())
}

pub(super) fn size_prefixed_bcs_bytes<R: Serialize>(record: &R) -> Result<Bytes> {
    let record_bytes = bcs::to_bytes(&record)?;
    let size_bytes = (record_bytes.len() as u32).to_be_bytes();

    let mut buf = BytesMut::with_capacity(size_bytes.len() + record_bytes.len());
    buf.put_slice(&size_bytes);
    buf.extend(record_bytes);

    Ok(buf.freeze())
}

/// Return 500 on any error raised by the request handler.
pub(super) fn unwrap_or_500(result: Result<Box<dyn Reply>>) -> Box<dyn Reply> {
    match result {
        Ok(resp) => resp,
        Err(e) => {
            warn!("Request handler exception: {:#}", e);
            Box::new(warp::http::StatusCode::INTERNAL_SERVER_ERROR)
        },
    }
}

/// Return 400 on any rejections (parameter parsing errors).
pub(super) async fn handle_rejection(err: Rejection) -> Result<impl Reply, Infallible> {
    warn!("bad request: {:?}", err);
    Ok(warp::http::StatusCode::BAD_REQUEST)
}

trait IntoDbResult<T> {
    fn into_db_res(self) -> Result<T>;
}

impl<T, E: std::error::Error> IntoDbResult<T> for std::result::Result<T, E> {
    fn into_db_res(self) -> Result<T> {
        self.map_err(|e| AptosDbError::Other(e.to_string()))
    }
}
