// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{auth::with_auth, context::Context, types::auth::Claims};
use aptos_config::config::PeerRole;
use aptos_logger::{debug, error};
use reqwest::StatusCode;
use warp::{filters::BoxedFilter, hyper::body::Bytes, reply, Filter, Rejection, Reply};

pub fn metrics_ingest(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path!("push-metrics")
        .and(warp::post())
        .and(context.clone().filter())
        .and(with_auth(
            context,
            vec![PeerRole::Validator, PeerRole::ValidatorFullNode],
        ))
        .and(warp::body::bytes())
        .and_then(handle_metrics_ingest)
        .boxed()
}

pub async fn handle_metrics_ingest(
    context: Context,
    claims: Claims,
    metrics_body: Bytes,
) -> anyhow::Result<impl Reply, Rejection> {
    let extra_labels = vec![
        format!("peer_id={}", claims.peer_id),
        format!("peer_role={}", claims.peer_role.as_str()),
        format!("chain_name={}", claims.chain_id),
        format!("namespace={}", "telemetry-service"),
        format!(
            "kubernetes_pod_name={}/{}",
            claims.peer_role.as_str(),
            claims.peer_id
        ),
        format!("role={}", claims.peer_role.as_str()),
    ];

    let res = context
        .victoria_metrics_client
        .unwrap()
        .post_prometheus_metrics(metrics_body, extra_labels)
        .await;

    match res {
        Ok(res) => {
            if res.status().is_success() {
                debug!("remote write to victoria metrics succeeded");
            } else {
                error!(
                    "remote write failed to victoria_metrics: {}",
                    res.error_for_status().err().unwrap()
                );
            }
        }
        Err(err) => {
            error!("error sending remote write request: {}", err);
        }
    }

    Ok(reply::with_status(reply::reply(), StatusCode::CREATED))
}
