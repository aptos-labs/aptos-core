// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    auth::with_auth,
    constants::IP_ADDRESS_KEY,
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
use aptos_types::PeerId;
use gcp_bigquery_client::model::table_data_insert_all_request::TableDataInsertAllRequest;
use serde_json::json;
use std::{str::FromStr, time::Duration};
use tokio::time::Instant;
use warp::{filters::BoxedFilter, hyper::StatusCode, reject, reply, Filter, Rejection, Reply};

pub fn custom_event_ingest(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path!("ingest" / "custom-event")
        .and(warp::post())
        .and(context.clone().filter())
        .and(with_auth(context, vec![
            NodeType::Validator,
            NodeType::ValidatorFullNode,
            NodeType::PublicFullNode,
            NodeType::Unknown,
            NodeType::UnknownValidator,
            NodeType::UnknownFullNode,
        ]))
        .and(warp::body::json())
        .and(warp::header::optional("X-Forwarded-For"))
        .and_then(handle_custom_event)
        .boxed()
}

fn validate_custom_event_body(
    claims: &Claims,
    body: &TelemetryDump,
) -> anyhow::Result<(), Rejection> {
    let body_peer_id = PeerId::from_str(&body.user_id).map_err(|_| {
        reject::custom(ServiceError::bad_request(
            CustomEventIngestError::InvalidEvent(body.user_id.clone(), claims.peer_id).into(),
        ))
    })?;
    if body_peer_id != claims.peer_id {
        return Err(reject::custom(ServiceError::bad_request(
            CustomEventIngestError::InvalidEvent(body.user_id.clone(), claims.peer_id).into(),
        )));
    }

    if body.events.is_empty() {
        return Err(reject::custom(ServiceError::bad_request(
            CustomEventIngestError::EmptyPayload.into(),
        )));
    }

    Ok(())
}

pub(crate) async fn handle_custom_event(
    context: Context,
    claims: Claims,
    mut body: TelemetryDump,
    forwarded_for: Option<String>,
) -> anyhow::Result<impl Reply, Rejection> {
    validate_custom_event_body(&claims, &body)?;

    let mut insert_request = TableDataInsertAllRequest::new();

    let client_ip = forwarded_for
        .as_ref()
        .and_then(|xff| xff.split(',').next())
        .unwrap_or("UNKNOWN");

    let telemetry_event = &mut body.events[0];
    telemetry_event
        .params
        .insert(IP_ADDRESS_KEY.into(), client_ip.into());

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

#[cfg(test)]
mod test {
    use super::validate_custom_event_body;
    use crate::types::{
        auth::Claims,
        common::NodeType,
        telemetry::{TelemetryDump, TelemetryEvent},
    };
    use aptos_types::{chain_id::ChainId, PeerId};
    use claims::assert_ok;
    use std::collections::BTreeMap;
    use uuid::Uuid;

    #[test]
    fn test_validate_custom_event_body() {
        let claims = Claims {
            chain_id: ChainId::test(),
            peer_id: PeerId::from_hex_literal("0x1234").unwrap(),
            node_type: NodeType::Validator,
            epoch: 1,
            exp: 100,
            iat: 200,
            run_uuid: Uuid::new_v4(),
        };

        let body = TelemetryDump {
            client_id: String::new(),
            user_id: String::from("0x1"),
            timestamp_micros: String::new(),
            events: Vec::new(),
        };
        assert_eq!(format!("{:?}", validate_custom_event_body(&claims, &body)), "Err(Rejection(ServiceError { http_code: 400, error_code: CustomEventIngestError(InvalidEvent(\"0x1\", 0000000000000000000000000000000000000000000000000000000000001234)) }))");

        let body = TelemetryDump {
            client_id: String::new(),
            user_id: String::from("0x1234"),
            timestamp_micros: String::new(),
            events: vec![TelemetryEvent {
                name: "test".into(),
                params: BTreeMap::new(),
            }],
        };
        assert_ok!(validate_custom_event_body(&claims, &body));

        let body = TelemetryDump {
            client_id: String::new(),
            user_id: String::from("1234"),
            timestamp_micros: String::new(),
            events: vec![TelemetryEvent {
                name: "test".into(),
                params: BTreeMap::new(),
            }],
        };
        assert_ok!(validate_custom_event_body(&claims, &body));

        let body = TelemetryDump {
            client_id: String::new(),
            user_id: String::from("0x00001234"),
            timestamp_micros: String::new(),
            events: vec![TelemetryEvent {
                name: "test".into(),
                params: BTreeMap::new(),
            }],
        };
        assert_ok!(validate_custom_event_body(&claims, &body));

        let body = TelemetryDump {
            client_id: String::new(),
            user_id: String::from("00001234"),
            timestamp_micros: String::new(),
            events: vec![TelemetryEvent {
                name: "test".into(),
                params: BTreeMap::new(),
            }],
        };
        assert_ok!(validate_custom_event_body(&claims, &body));
    }
}
