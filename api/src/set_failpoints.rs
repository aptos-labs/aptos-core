// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#[allow(unused_imports)]
use anyhow::{format_err, Result};
use serde::{Deserialize, Serialize};

#[cfg(feature = "failpoints")]
use crate::metrics::metrics;
#[cfg(feature = "failpoints")]
use warp::{filters::BoxedFilter, http::Response, Filter, Rejection, Reply};

#[derive(Deserialize, Serialize)]
struct FailpointConf {
    name: String,
    actions: String,
}

// GET /failpoints?name=str&actions=str
#[cfg(feature = "failpoints")]
pub fn set_failpoint() -> BoxedFilter<(impl Reply,)> {
    warp::path!("set_failpoint")
        .and(warp::get())
        .and(warp::query::<FailpointConf>())
        .and_then(handle_set_failpoint)
        .with(metrics("set_failpoint"))
        .boxed()
}

#[allow(unused_variables)]
#[inline]
#[cfg(feature = "failpoints")]
async fn handle_set_failpoint(failpoint_conf: FailpointConf) -> Result<impl Reply, Rejection> {
    fail::cfg(&failpoint_conf.name, &failpoint_conf.actions).map_err(|e| warp::reject::reject())?;
    println!(
        "Configured failpoint {} to {}",
        failpoint_conf.name, failpoint_conf.actions
    );
    return Ok(Response::builder().body(""));
}
