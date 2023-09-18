// Copyright © Aptos Foundation

use prometheus::{HistogramVec, register_histogram_vec};
use once_cell::sync::Lazy;
use crate::exponential_buckets;

pub static SHARDING_V3_SPAN_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        // metric name
        "sharding_v3_span_seconds",
        // metric description
        "TBD",
        &["name"],
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    )
        .unwrap()
});
