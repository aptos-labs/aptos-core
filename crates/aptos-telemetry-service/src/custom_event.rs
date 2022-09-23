// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

use crate::{
    auth::with_auth,
    context::Context,
    error::ServiceError,
    types::{
        auth::Claims,
        common::{EventIdentity, NodeType},
        telemetry::{BigQueryRow, TelemetryDump},
    },
};
use anyhow::anyhow;
use gcp_bigquery_client::model::table_data_insert_all_request::TableDataInsertAllRequest;
use serde_json::json;
use tracing::{debug, error};
use warp::{filters::BoxedFilter, reject, reply, Filter, Rejection, Reply};

/// TODO: Cleanup after v1 API is ramped up
pub fn custom_event_legacy(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path!("custom_event")
        .and(warp::post())
        .and(context.clone().filter())
        .and(with_auth(
            context,
            vec![
                NodeType::Validator,
                NodeType::ValidatorFullNode,
                NodeType::PublicFullNode,
                NodeType::Unknown,
            ],
        ))
        .and(warp::body::json())
        .and_then(handle_custom_event)
        .boxed()
}

pub fn custom_event_ingest(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path!("ingest" / "custom-event")
        .and(warp::post())
        .and(context.clone().filter())
        .and(with_auth(
            context,
            vec![
                NodeType::Validator,
                NodeType::ValidatorFullNode,
                NodeType::PublicFullNode,
                NodeType::Unknown,
            ],
        ))
        .and(warp::body::json())
        .and_then(handle_custom_event)
        .boxed()
}

pub(crate) async fn handle_custom_event(
    context: Context,
    claims: Claims,
    body: TelemetryDump,
) -> anyhow::Result<impl Reply, Rejection> {
    if !body
        .user_id
        .eq_ignore_ascii_case(&claims.peer_id.to_string())
    {
        return Err(reject::custom(ServiceError::bad_request(format!(
            "user_id {} in event does not match peer_id {}",
            body.user_id, claims.peer_id
        ))));
    }

    if body.events.is_empty() {
        return Err(reject::custom(ServiceError::bad_request(
            "no events found in payload",
        )));
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
            .map_err(|_| ServiceError::bad_request("unable to parse timestamp"))?,
    );

    let row = BigQueryRow {
        event_identity: EventIdentity::from(claims),
        event_name: telemetry_event.name.clone(),
        event_timestamp: duration.as_secs(),
        event_params,
    };

    insert_request.add_row(None, &row).map_err(|e| {
        error!("unable to create row: {}", e);
        ServiceError::from(anyhow!("unable to insert row into bigquery"))
    })?;

    context
        .bigquery_client()
        .ok_or_else(|| {
            error!("big query client is not configured");
            ServiceError::from(anyhow!("unable to insert row into bigquery"))
        })?
        .insert_all(insert_request)
        .await
        .map_err(|e| {
            error!("unable to insert row into bigquery: {}", e);
            ServiceError::from(anyhow!("unable to insert row into bigquery"))
        })?;

    debug!("row inserted succeefully: {:?}", &row);

    Ok(reply::reply())
}
