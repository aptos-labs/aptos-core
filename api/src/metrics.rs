// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use diem_metrics::{register_histogram_vec, HistogramVec};

use once_cell::sync::Lazy;
use warp::log::{custom, Info, Log};

static HISTOGRAM: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "diem_api_requests",
        "API requests latency grouped by method, operation_id and status",
        &["method", "operation_id", "status"]
    )
    .unwrap()
});

static RESPONSE_STATUS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "diem_api_response_status",
        "API requests latency grouped by status code only",
        &["status"]
    )
    .unwrap()
});

// Record metrics by method, operation_id and status.
// The operation_id is the id for the request handler.
// Should use same `operationId` defined in `openapi.yaml` whenever possible.
pub fn metrics(operation_id: &'static str) -> Log<impl Fn(Info) + Copy> {
    let func = move |info: Info| {
        HISTOGRAM
            .with_label_values(&[
                info.method().to_string().as_str(),
                operation_id,
                info.status().as_u16().to_string().as_str(),
            ])
            .observe(info.elapsed().as_secs_f64());
    };
    custom(func)
}

// Record metrics by response status.
// This is for understanding the overview of responses in case server
// is overloaded by unknown reason.
pub fn status_metrics() -> Log<impl Fn(Info) + Copy> {
    let func = move |info: Info| {
        RESPONSE_STATUS
            .with_label_values(&[info.status().as_u16().to_string().as_str()])
            .observe(info.elapsed().as_secs_f64());
    };
    custom(func)
}
