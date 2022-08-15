// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::context::Context;
use aptos_logger::{debug, error};
use warp::{filters::BoxedFilter, reject, reply, Filter, Rejection, Reply, hyper::body::Bytes};

pub fn metrics_ingest(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path!("metrics-ingest")
        .and(warp::post())
        .and(context.filter())
        .and(warp::body::bytes())
        .and_then(handle_metrics_ingest)
        .boxed()
}

pub async fn handle_metrics_ingest(
    context: Context,
    body: Bytes,
) -> anyhow::Result<impl Reply, Rejection> {
    let client = reqwest::Client::new();

    let res = client.post("http://localhost:9090/api/v1/write").body(body).send().await;

    match res {
        Ok(res) => {
            if res.status().is_success() {
                debug!("remote write succeeded");
            } else {
                error!("remote write failed {}", res.error_for_status().err().unwrap());
            }
        },
        Err(err) => {
            error!("error sending request {}", err);
        },
    }
    

    Ok(reply::reply())
}
