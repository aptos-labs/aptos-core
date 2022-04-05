// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::{ensure, Result};
use std::{
    ops::Sub,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use storage_interface::DbReader;
use warp::{filters::BoxedFilter, reject, Filter, Reply};

// HealthCheckParams is optional params for different layer's health check.
// If no param is provided, server return 200 by default to indicate HTTP server is running health.
#[derive(serde::Deserialize)]
struct HealthCheckParams {
    // Health check returns 200 when this param is provided and meet the following condition:
    //   server latest ledger info timestamp >= server current time timestamp - duration_secs
    pub duration_secs: Option<u64>,
}

#[derive(Debug)]
struct HealthCheckError;
impl reject::Reject for HealthCheckError {}

pub fn health_check_route(health_aptos_db: Arc<dyn DbReader>) -> BoxedFilter<(impl Reply,)> {
    warp::path!("-" / "healthy")
        .and(warp::path::end())
        .and(warp::query().map(move |params: HealthCheckParams| params))
        .and(warp::any().map(move || health_aptos_db.clone()))
        .and(warp::any().map(SystemTime::now))
        .and_then(health_check)
        .boxed()
}

async fn health_check(
    params: HealthCheckParams,
    db: Arc<dyn DbReader>,
    now: SystemTime,
) -> Result<Box<dyn warp::Reply>, warp::Rejection> {
    if let Some(duration) = params.duration_secs {
        let ledger_info = db
            .get_latest_ledger_info()
            .map_err(|_| reject::custom(HealthCheckError))?;
        let timestamp = ledger_info.ledger_info().timestamp_usecs();

        check_latest_ledger_info_timestamp(duration, timestamp, now)
            .map_err(|_| reject::custom(HealthCheckError))?;
    }
    Ok(Box::new("aptos-node:ok"))
}

pub fn check_latest_ledger_info_timestamp(
    duration_sec: u64,
    timestamp_usecs: u64,
    now: SystemTime,
) -> Result<()> {
    let timestamp = Duration::from_micros(timestamp_usecs);
    let expectation = now
        .sub(Duration::from_secs(duration_sec))
        .duration_since(UNIX_EPOCH)?;
    ensure!(timestamp >= expectation);
    Ok(())
}
