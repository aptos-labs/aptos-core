// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::context::Context;
#[cfg(feature = "failpoints")]
use crate::metrics::metrics;
#[allow(unused_imports)]
use anyhow::{format_err, Result};
#[cfg(feature = "failpoints")]
use aptos_logger::prelude::*;
use poem::{
    handler,
    web::{Data, Query},
};
use serde::{Deserialize, Serialize};
#[cfg(feature = "failpoints")]
use warp::{filters::BoxedFilter, http::Response, Filter, Rejection, Reply};

#[derive(Deserialize, Serialize)]
pub struct FailpointConf {
    name: String,
    actions: String,
}

// GET /set_failpoint?name=str&actions=str
#[cfg(feature = "failpoints")]
pub fn set_failpoint(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path!("set_failpoint")
        .and(warp::get())
        .and(warp::query::<FailpointConf>())
        .and(context.filter())
        .and_then(handle_set_failpoint)
        .with(metrics("set_failpoint"))
        .boxed()
}

#[allow(unused_variables)]
#[inline]
#[cfg(feature = "failpoints")]
async fn handle_set_failpoint(
    failpoint_conf: FailpointConf,
    context: Context,
) -> Result<impl Reply, Rejection> {
    if context.failpoints_enabled() {
        fail::cfg(&failpoint_conf.name, &failpoint_conf.actions)
            .map_err(|e| warp::reject::reject())?;
        info!(
            "Configured failpoint {} to {}",
            failpoint_conf.name, failpoint_conf.actions
        );
        Ok(Response::builder().body(""))
    } else {
        Err(warp::reject::reject())
    }
}

#[cfg(feature = "failpoints")]
#[handler]
pub fn set_failpoint_poem(
    context: Data<&std::sync::Arc<Context>>,
    Query(failpoint_conf): Query<FailpointConf>,
) -> poem::Result<String> {
    if context.failpoints_enabled() {
        fail::cfg(&failpoint_conf.name, &failpoint_conf.actions)
            .map_err(|e| poem::Error::from(anyhow::anyhow!(e)))?;
        info!(
            "Configured failpoint {} to {}",
            failpoint_conf.name, failpoint_conf.actions
        );
        Ok(format!("Set failpoint {}", failpoint_conf.name))
    } else {
        Err(poem::Error::from(anyhow::anyhow!(
            "Failpoints are not enabled at a config level"
        )))
    }
}

#[allow(unused_variables)]
#[cfg(not(feature = "failpoints"))]
#[handler]
pub fn set_failpoint_poem(
    context: Data<&std::sync::Arc<Context>>,
    Query(failpoint_conf): Query<FailpointConf>,
) -> poem::Result<String> {
    Err(poem::Error::from(anyhow::anyhow!(
        "Failpoints are not enabled at a feature level"
    )))
}
