// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::handlers::bytes_sender;
use aptos_db::{backup::backup_handler::BackupHandler, metrics::BACKUP_TIMER};
use aptos_logger::prelude::*;
use aptos_metrics_core::{
    HistogramVec, IntCounterVec, TimerHelper, register_histogram_vec, register_int_counter_vec,
};
use aptos_storage_interface::Result as DbResult;
use hyper::Body;
use once_cell::sync::Lazy;
use serde::Serialize;
use std::convert::Infallible;
use warp::{Rejection, Reply, reply::Response};

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
) -> DbResult<Box<dyn Reply>> {
    let bytes = bcs::to_bytes(record)?;
    THROUGHPUT_COUNTER
        .with_label_values(&[endpoint])
        .inc_by(bytes.len() as u64);
    Ok(Box::new(bytes))
}

pub(super) fn reply_with_bytes_sender<F>(
    backup_handler: &BackupHandler,
    endpoint: &'static str,
    f: F,
) -> Box<dyn Reply>
where
    F: FnOnce(BackupHandler, &mut bytes_sender::BytesSender) -> DbResult<()> + Send + 'static,
{
    let (sender, stream) = bytes_sender::BytesSender::new(endpoint);

    // spawn and forget, error propagates through the `stream: TryStream<_>`
    let bh = backup_handler.clone();
    let _join_handle = tokio::task::spawn_blocking(move || {
        let _timer =
            BACKUP_TIMER.timer_with(&[&format!("backup_service_bytes_sender_{}", endpoint)]);
        abort_on_error(f)(bh, sender)
    });

    Box::new(Response::new(Body::wrap_stream(stream)))
}

pub(super) fn abort_on_error<F>(
    f: F,
) -> impl FnOnce(BackupHandler, bytes_sender::BytesSender) + Send + 'static
where
    F: FnOnce(BackupHandler, &mut bytes_sender::BytesSender) -> DbResult<()> + Send + 'static,
{
    move |bh: BackupHandler, mut sender: bytes_sender::BytesSender| {
        // ignore error from finish() and abort()
        let _res = match f(bh, &mut sender) {
            Ok(()) => sender.finish(),
            Err(e) => sender.abort(e),
        };
    }
}

/// Return 500 on any error raised by the request handler.
pub(super) fn unwrap_or_500(result: DbResult<Box<dyn Reply>>) -> Box<dyn Reply> {
    match result {
        Ok(resp) => resp,
        Err(e) => {
            warn!("Request handler exception: {:#}", e);
            Box::new(warp::http::StatusCode::INTERNAL_SERVER_ERROR)
        },
    }
}

/// Return 400 on any rejections (parameter parsing errors).
pub(super) async fn handle_rejection(err: Rejection) -> DbResult<impl Reply, Infallible> {
    warn!("bad request: {:?}", err);
    Ok(warp::http::StatusCode::BAD_REQUEST)
}
