// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#[allow(unused_imports)]
use anyhow::{format_err, Result};
use serde::{Deserialize, Serialize};
use warp::{filters::BoxedFilter, http::Response, Filter, Rejection, Reply};

use crate::metrics::metrics;

#[derive(Deserialize, Serialize)]
struct FailpointConf {
    name: String,
    actions: String,
}

// GET /failpoints?name=str&actions=str
pub fn get_update_failpoint() -> BoxedFilter<(impl Reply,)> {
    warp::path!("set_failpoint")
        .and(warp::get())
        .and(warp::query::<FailpointConf>())
        .and_then(set_failpoint)
        .with(metrics("set_failpoint"))
        .boxed()
}

#[allow(unused_variables)]
#[inline]
async fn set_failpoint(failpoint_conf: FailpointConf) -> Result<impl Reply, Rejection> {
    #[cfg(feature = "failpoints")]
    {
        fail::cfg(&failpoint_conf.name, &failpoint_conf.actions)
            .map_err(|e| warp::reject::reject())?;
        println!(
            "Configured failpoint {} to {}",
            failpoint_conf.name, failpoint_conf.actions
        );
        if true {
            return Ok(Response::builder().body("true"));
        }
    }
    Ok(Response::builder().body("false"))
}
