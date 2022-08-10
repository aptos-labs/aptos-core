// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

use crate::{
    auth::with_auth,
    context::Context,
    error,
    types::auth::{Claims, TelemetryDump},
};
use aptos_config::config::PeerRole;
use aptos_logger::{error, info};
use gcp_bigquery_client::model::table_data_insert_all_request::TableDataInsertAllRequest;
use serde_json::json;
use warp::{filters::BoxedFilter, reject, reply, Filter, Rejection, Reply};

pub fn custom_event(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path!("custom_event")
        .and(warp::post())
        .and(context.clone().filter())
        .and(with_auth(
            context,
            vec![PeerRole::Validator, PeerRole::Unknown],
        ))
        .and(warp::body::json())
        .and_then(handle_custom_event)
        .boxed()
}

pub async fn handle_custom_event(
    context: Context,
    _claims: Claims,
    body: TelemetryDump,
) -> anyhow::Result<impl Reply, Rejection> {
    let mut insert_request = TableDataInsertAllRequest::new();

    if body.events.is_empty() {
        return Err(reject::custom(error::Error::InvalidCustomEvent));
    }

    let telemetry_event = body.events[0].clone();
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
        .add_row(None, row.clone())
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
            reject::custom(error::Error::GCPInsertError)
        })?;

    info!("insert succeeded {:?}", row.to_string());

    Ok(reply::reply())
}
