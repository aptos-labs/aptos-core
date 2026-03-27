// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    auth::authorize_request,
    constants::IP_ADDRESS_KEY,
    context::Context,
    debug, error,
    errors::{json_rejection_to_service_error, CustomEventIngestError, ServiceError},
    metrics::BIG_QUERY_BACKEND_REQUEST_DURATION,
    types::{
        auth::Claims,
        common::{EventIdentity, NodeType},
        telemetry::{BigQueryRow, TelemetryDump},
    },
};
use anyhow::anyhow;
use aptos_types::PeerId;
use axum::{
    extract::{rejection::JsonRejection, Extension, Json},
    http::{HeaderMap, StatusCode},
};
use gcp_bigquery_client::model::table_data_insert_all_request::TableDataInsertAllRequest;
use serde_json::json;
use std::{str::FromStr, time::Duration};
use tokio::time::Instant;

pub async fn post_custom_event(
    Extension(context): Extension<Context>,
    headers: HeaderMap,
    body: Result<Json<TelemetryDump>, JsonRejection>,
) -> Result<StatusCode, ServiceError> {
    let Json(mut body) = body.map_err(json_rejection_to_service_error)?;
    let claims = authorize_request(&context, &headers, &[
        NodeType::Validator,
        NodeType::ValidatorFullNode,
        NodeType::PublicFullNode,
        NodeType::Unknown,
        NodeType::UnknownValidator,
        NodeType::UnknownFullNode,
    ])
    .await?;
    let forwarded_for = headers
        .get("X-Forwarded-For")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    handle_custom_event(context, claims, &mut body, forwarded_for).await
}

fn validate_custom_event_body(claims: &Claims, body: &TelemetryDump) -> Result<(), ServiceError> {
    let body_peer_id = PeerId::from_str(&body.user_id).map_err(|_| {
        ServiceError::bad_request(
            CustomEventIngestError::InvalidEvent(body.user_id.clone(), claims.peer_id).into(),
        )
    })?;
    if body_peer_id != claims.peer_id {
        return Err(ServiceError::bad_request(
            CustomEventIngestError::InvalidEvent(body.user_id.clone(), claims.peer_id).into(),
        ));
    }

    if body.events.is_empty() {
        return Err(ServiceError::bad_request(
            CustomEventIngestError::EmptyPayload.into(),
        ));
    }

    Ok(())
}

pub(crate) async fn handle_custom_event(
    context: Context,
    claims: Claims,
    body: &mut TelemetryDump,
    forwarded_for: Option<String>,
) -> Result<StatusCode, ServiceError> {
    validate_custom_event_body(&claims, body)?;

    let is_unknown = matches!(
        claims.node_type,
        NodeType::Unknown | NodeType::UnknownValidator | NodeType::UnknownFullNode
    );
    if is_unknown && !context.unknown_logs_rate_limiter().check_rate_limit().await {
        debug!(
            "rate limit exceeded for unknown node events: peer_id={}",
            claims.peer_id
        );
        return Err(ServiceError::too_many_requests(
            CustomEventIngestError::RateLimitExceeded.into(),
        ));
    }

    let mut insert_request = TableDataInsertAllRequest::new();

    let client_ip = forwarded_for
        .as_ref()
        .and_then(|xff| xff.split(',').next())
        .unwrap_or("UNKNOWN");

    let duration =
        Duration::from_micros(body.timestamp_micros.as_str().parse::<u64>().map_err(|_| {
            ServiceError::bad_request(
                CustomEventIngestError::InvalidTimestamp(body.timestamp_micros.clone()).into(),
            )
        })?);

    let event_identity = EventIdentity::from(claims);

    for telemetry_event in &mut body.events {
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

        let row = BigQueryRow {
            event_identity: event_identity.clone(),
            event_name: telemetry_event.name.clone(),
            event_timestamp: duration.as_secs(),
            event_params,
        };

        insert_request.add_row(None, &row).map_err(|e| {
            error!("unable to create row: {}", e);
            ServiceError::internal(CustomEventIngestError::from(e).into())
        })?;
    }

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

    debug!("inserted {} events successfully", body.events.len());

    Ok(StatusCode::CREATED)
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
        assert_eq!(format!("{:?}", validate_custom_event_body(&claims, &body)), "Err(ServiceError { http_code: 400, error_code: CustomEventIngestError(InvalidEvent(\"0x1\", 0000000000000000000000000000000000000000000000000000000000001234)) })");

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
