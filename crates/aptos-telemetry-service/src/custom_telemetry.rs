// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

use crate::{
    auth::with_auth,
    context::Context,
    types::{auth::Claims, telemetry::TelemetryDump},
};
use aptos_config::config::PeerRole;
use aptos_logger::{debug, error};
use gcp_bigquery_client::model::table_data_insert_all_request::TableDataInsertAllRequest;
use serde_json::json;
use warp::{filters::BoxedFilter, reject, reply, Filter, Rejection, Reply};

pub fn custom_telemetry(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path!("telemetry")
        .and(warp::post())
        .and(context.clone().filter())
        .and(with_auth(
            context,
            vec![PeerRole::Validator, PeerRole::Unknown],
        ))
        .and(warp::body::json())
        .and_then(handle_custom_telemetry)
        .boxed()
}

pub async fn handle_custom_telemetry(
    context: Context,
    claims: Claims,
    body: TelemetryDump,
) -> anyhow::Result<impl Reply, Rejection> {
    if body.user_id != claims.peer_id.to_string() {
        return Err(reject::reject());
    }

    let mut insert_request = TableDataInsertAllRequest::new();

    let telemetry_event = &body.events[0];
    let event_params: Vec<serde_json::Value> = telemetry_event
        .params
        .iter()
        .map(|(k, v)| {
            json!({
                "key": k,
                "value": v
            })
        })
        .collect();

    let duration = Duration::from_micros(
        body.timestamp_micros
            .as_str()
            .parse::<u64>()
            .map_err(|_| reject::reject())?,
    );

    let row = json!({
        "event_name": telemetry_event.name,
        "event_timestamp": duration.as_secs(),
        "event_params": event_params,
    });

    insert_request
        .add_row(None, &row)
        .map_err(|_| reject::reject())?;

    context
        .gcp_bq_client
        .unwrap()
        .tabledata()
        .insert_all(
            context.gcp_bq_config.project_id.as_str(),
            context.gcp_bq_config.dataset_id.as_str(),
            context.gcp_bq_config.table_id.as_str(),
            insert_request,
        )
        .await
        .map_err(|e| {
            error!("Error due to {}", e);
            reject::reject()
        })?;

    debug!("inject succeeded {:?}", &row);
    Ok(reply::reply())
}
