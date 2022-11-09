// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

use crate::{
    auth::with_auth,
    context::Context,
    debug, error,
    errors::{CustomEventIngestError, ServiceError},
    metrics::BIG_QUERY_BACKEND_REQUEST_DURATION,
    types::{
        auth::Claims,
        common::{EventIdentity, NodeType},
        telemetry::{BigQueryRow, TelemetryDump},
    },
};

use anyhow::anyhow;
use gcp_bigquery_client::model::table_data_insert_all_request::TableDataInsertAllRequest;
use serde_json::json;
use tokio::time::Instant;
use warp::{filters::BoxedFilter, hyper::StatusCode, reject, reply, Filter, Rejection, Reply};

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
        return Err(reject::custom(ServiceError::bad_request(
            CustomEventIngestError::InvalidEvent(body.user_id, claims.peer_id).into(),
        )));
    }

    if body.events.is_empty() {
        return Err(reject::custom(ServiceError::bad_request(
            CustomEventIngestError::EmptyPayload.into(),
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

    let duration =
        Duration::from_micros(body.timestamp_micros.as_str().parse::<u64>().map_err(|_| {
            ServiceError::bad_request(
                CustomEventIngestError::InvalidTimestamp(body.timestamp_micros).into(),
            )
        })?);

    let row = BigQueryRow {
        event_identity: EventIdentity::from(claims),
        event_name: telemetry_event.name.clone(),
        event_timestamp: duration.as_secs(),
        event_params,
    };

    insert_request.add_row(None, &row).map_err(|e| {
        error!("unable to create row: {}", e);
        ServiceError::internal(CustomEventIngestError::from(e).into())
    })?;

    let start_timer = Instant::now();

    context
        .bigquery_client()
        .ok_or_else(|| {
            error!("big query client is not configured");
            ServiceError::internal(
                CustomEventIngestError::from(anyhow!("BQ client is not configured")).into(),
            )
        })?
        .insert_all(insert_request)
        .await
        .map_err(|e| {
            BIG_QUERY_BACKEND_REQUEST_DURATION
                .with_label_values(&["request_error"])
                .observe(start_timer.elapsed().as_millis() as f64);
            error!("unable to insert row into bigquery: {}", e);
            ServiceError::internal(CustomEventIngestError::from(e).into())
        })
        .and_then(|result| {
            if let Some(err) = result.insert_errors {
                BIG_QUERY_BACKEND_REQUEST_DURATION
                    .with_label_values(&["insert_error"])
                    .observe(start_timer.elapsed().as_secs_f64());
                Err(ServiceError::bad_request(
                    CustomEventIngestError::from(err[0].clone()).into(),
                ))
            } else {
                BIG_QUERY_BACKEND_REQUEST_DURATION
                    .with_label_values(&["success"])
                    .observe(start_timer.elapsed().as_secs_f64());
                Ok(result)
            }
        })?;

    debug!("row inserted succeefully: {:?}", &row);

    Ok(reply::with_status(reply::reply(), StatusCode::CREATED))
}
