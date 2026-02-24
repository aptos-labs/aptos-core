// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

/// Storage Provider Data Ingestion
///
/// This module handles data ingestion from authenticated storage providers.
/// The data flows to separate sinks configured specifically for storage providers
/// to avoid mixing with node telemetry data.
use crate::{
    clients::humio::{PEER_ID_FIELD_NAME, PEER_ROLE_TAG_NAME},
    constants::MAX_DECOMPRESSED_LENGTH,
    context::Context,
    custom_contract_auth::with_custom_contract_auth,
    debug, error,
    errors::{CustomEventIngestError, LogIngestError, ServiceError, ServiceErrorCode},
    metrics::{record_custom_contract_error, CustomContractEndpoint, CustomContractErrorType},
    types::{
        common::EventIdentity, common::NodeType, humio::UnstructuredLog, telemetry::TelemetryDump,
    },
    warn,
};
use aptos_types::{chain_id::ChainId, PeerId};
use flate2::read::GzDecoder;
use gcp_bigquery_client::model::table_data_insert_all_request::TableDataInsertAllRequest;
use std::{collections::HashMap, io::Read};
use uuid::Uuid;
use warp::{filters::BoxedFilter, hyper::body::Bytes, reject, reply, Filter, Rejection, Reply};

/// Custom contract metrics ingest endpoint
pub fn metrics_ingest(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path!("custom-contract" / String / "ingest" / "metrics")
        .and(warp::post())
        .and(context.clone().filter())
        .and(with_custom_contract_auth(context.clone()))
        .and(warp::header::optional("content-encoding"))
        .and(warp::body::bytes())
        .and_then(handle_metrics_ingest)
        .boxed()
}

/// Handle custom contract metrics ingestion
async fn handle_metrics_ingest(
    contract_name: String,
    context: Context,
    (jwt_contract_name, peer_id, chain_id, is_trusted): (String, PeerId, ChainId, bool),
    content_encoding: Option<String>,
    body: Bytes,
) -> Result<impl Reply, Rejection> {
    // Verify the JWT was issued for this specific contract (prevents cross-contract token reuse)
    if jwt_contract_name != contract_name {
        error!(
            "contract name mismatch: JWT issued for '{}', request for '{}'",
            jwt_contract_name, contract_name
        );
        record_custom_contract_error(
            &contract_name,
            CustomContractEndpoint::MetricsIngest,
            CustomContractErrorType::TokenMismatch,
        );
        return Err(reject::custom(ServiceError::forbidden(
            ServiceErrorCode::CustomContractAuthError(
                format!(
                    "token issued for '{}' cannot be used for '{}'",
                    jwt_contract_name, contract_name
                ),
                chain_id,
            ),
        )));
    }

    // Check if the peer is blacklisted for this contract
    if let Some(instance) = context.get_custom_contract(&contract_name) {
        if instance.is_peer_blacklisted(&peer_id) {
            debug!(
                "peer_id {} is blacklisted from custom contract '{}' metrics",
                peer_id, contract_name
            );
            record_custom_contract_error(
                &contract_name,
                CustomContractEndpoint::MetricsIngest,
                CustomContractErrorType::NotInAllowlist,
            );
            return Err(reject::custom(ServiceError::forbidden(
                ServiceErrorCode::CustomContractAuthError(
                    format!("peer_id {} is blacklisted from this contract", peer_id),
                    chain_id,
                ),
            )));
        }
    }

    // Apply rate limiting for untrusted nodes (metrics)
    // Check per-contract metrics rate limiter first, then fall back to global metrics rate limiter
    if !is_trusted {
        let contract_rate_limited = !context
            .contract_metrics_rate_limiters()
            .check_rate_limit(&contract_name)
            .await;

        // If no per-contract limiter exists, use global metrics rate limiter
        let use_global = !context
            .contract_metrics_rate_limiters()
            .has_limiter(&contract_name)
            .await;
        let global_rate_limited = use_global
            && !context
                .unknown_metrics_rate_limiter()
                .check_rate_limit()
                .await;

        if contract_rate_limited || global_rate_limited {
            debug!(
                "rate limit exceeded for untrusted custom contract '{}' metrics: peer_id={}",
                contract_name, peer_id
            );
            record_custom_contract_error(
                &contract_name,
                CustomContractEndpoint::MetricsIngest,
                CustomContractErrorType::RateLimitExceeded,
            );
            return Err(reject::custom(ServiceError::too_many_requests(
                ServiceErrorCode::CustomContractAuthError(
                    "rate limit exceeded for untrusted telemetry".to_string(),
                    chain_id,
                ),
            )));
        }
    }

    debug!(
        "received custom contract '{}' metrics from peer_id: {}, chain_id: {}, is_trusted: {}, body length: {}",
        contract_name,
        peer_id,
        chain_id,
        is_trusted,
        body.len()
    );

    // Get the custom contract instance
    let instance = context.get_custom_contract(&contract_name).ok_or_else(|| {
        error!("custom contract '{}' not configured", contract_name);
        record_custom_contract_error(
            &contract_name,
            CustomContractEndpoint::MetricsIngest,
            CustomContractErrorType::ContractNotConfigured,
        );
        reject::custom(ServiceError::internal(
            LogIngestError::IngestionError.into(),
        ))
    })?;

    // Get the appropriate metrics clients based on whether node is trusted (allowlisted)
    // Unknown/untrusted nodes go to untrusted_metrics_sinks if configured, else regular sinks
    let metrics_clients = instance.get_metrics_clients(is_trusted);

    // Prepare extra labels for metrics - include contract name, node_type_name, and trust status
    // Format: name=value (no quotes - Victoria Metrics extra_label format)
    let node_type = &instance.node_type_name;
    let trust_label = if is_trusted { "trusted" } else { "untrusted" };

    // Build kubernetes_pod_name label matching standard telemetry behavior:
    // - If peer_identity is configured: "peer_id:{identity}//{peer_id_hex}"
    // - Otherwise: "peer_id:{peer_id_hex}"
    let pod_name = if let Some(identity) = instance.get_peer_identity(&chain_id, &peer_id) {
        format!(
            "kubernetes_pod_name=peer_id:{}//{}",
            identity,
            peer_id.to_hex_literal()
        )
    } else {
        format!("kubernetes_pod_name=peer_id:{}", peer_id.to_hex_literal())
    };

    let extra_labels = vec![
        format!("peer_id={}", peer_id),
        format!("node_type={}", node_type),
        format!("contract_name={}", contract_name),
        format!("trust_status={}", trust_label),
        pod_name,
    ];

    // Determine encoding
    let encoding = content_encoding.unwrap_or_else(|| "identity".to_string());

    // Send metrics to all configured sinks for this custom contract
    for (name, client) in metrics_clients {
        debug!(
            "forwarding custom contract '{}' metrics to {} sink '{}' (url: {})",
            contract_name,
            trust_label,
            name,
            client.base_url()
        );
        match client
            .post_prometheus_metrics(body.clone(), extra_labels.clone(), encoding.clone())
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                if !status.is_success() {
                    let body = resp.text().await.unwrap_or_default();
                    warn!(
                        "metrics sink '{}' returned non-success status: {} body: '{}' for contract '{}'",
                        name, status, body, contract_name
                    );
                } else {
                    debug!(
                        "metrics sink '{}' returned success status: {} for contract '{}'",
                        name, status, contract_name
                    );
                }
            },
            Err(e) => {
                warn!("failed to forward metrics to sink '{}': {}", name, e);
            },
        }
    }

    Ok(reply::reply())
}

/// Custom contract logs ingest endpoint
pub fn log_ingest(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path!("custom-contract" / String / "ingest" / "logs")
        .and(warp::post())
        .and(context.clone().filter())
        .and(with_custom_contract_auth(context.clone()))
        .and(warp::header::optional("content-encoding"))
        .and(warp::body::bytes())
        .and_then(handle_log_ingest)
        .boxed()
}

/// Handle custom contract log ingestion
async fn handle_log_ingest(
    contract_name: String,
    context: Context,
    (jwt_contract_name, peer_id, chain_id, is_trusted): (String, PeerId, ChainId, bool),
    content_encoding: Option<String>,
    body: Bytes,
) -> Result<impl Reply, Rejection> {
    // Verify the JWT was issued for this specific contract (prevents cross-contract token reuse)
    if jwt_contract_name != contract_name {
        error!(
            "contract name mismatch: JWT issued for '{}', request for '{}'",
            jwt_contract_name, contract_name
        );
        record_custom_contract_error(
            &contract_name,
            CustomContractEndpoint::LogsIngest,
            CustomContractErrorType::TokenMismatch,
        );
        return Err(reject::custom(ServiceError::forbidden(
            ServiceErrorCode::CustomContractAuthError(
                format!(
                    "token issued for '{}' cannot be used for '{}'",
                    jwt_contract_name, contract_name
                ),
                chain_id,
            ),
        )));
    }

    // Check if the peer is blacklisted for this contract
    if let Some(instance) = context.get_custom_contract(&contract_name) {
        if instance.is_peer_blacklisted(&peer_id) {
            debug!(
                "peer_id {} is blacklisted from custom contract '{}' logs",
                peer_id, contract_name
            );
            record_custom_contract_error(
                &contract_name,
                CustomContractEndpoint::LogsIngest,
                CustomContractErrorType::NotInAllowlist,
            );
            return Err(reject::custom(ServiceError::forbidden(
                ServiceErrorCode::CustomContractAuthError(
                    format!("peer_id {} is blacklisted from this contract", peer_id),
                    chain_id,
                ),
            )));
        }
    }

    // Apply rate limiting for untrusted nodes (logs)
    // Check per-contract logs rate limiter first, then fall back to global logs rate limiter
    if !is_trusted {
        let contract_rate_limited = !context
            .contract_logs_rate_limiters()
            .check_rate_limit(&contract_name)
            .await;

        let use_global = !context
            .contract_logs_rate_limiters()
            .has_limiter(&contract_name)
            .await;
        let global_rate_limited =
            use_global && !context.unknown_logs_rate_limiter().check_rate_limit().await;

        if contract_rate_limited || global_rate_limited {
            debug!(
                "rate limit exceeded for untrusted custom contract '{}' logs: peer_id={}",
                contract_name, peer_id
            );
            record_custom_contract_error(
                &contract_name,
                CustomContractEndpoint::LogsIngest,
                CustomContractErrorType::RateLimitExceeded,
            );
            return Err(reject::custom(ServiceError::too_many_requests(
                ServiceErrorCode::CustomContractAuthError(
                    "rate limit exceeded for untrusted telemetry".to_string(),
                    chain_id,
                ),
            )));
        }
    }

    debug!(
        "received custom contract '{}' logs from peer_id: {}, chain_id: {}, is_trusted: {}, body length: {}",
        contract_name,
        peer_id,
        chain_id,
        is_trusted,
        body.len()
    );

    // Decode the body if gzip encoded (with size limit to prevent decompression bombs)
    let log_data = if content_encoding.as_deref() == Some("gzip") {
        let decoder = GzDecoder::new(&body[..]);
        // Limit decompressed size to prevent decompression bomb attacks
        let mut limited_decoder = decoder.take(MAX_DECOMPRESSED_LENGTH as u64);
        let mut decompressed = Vec::new();
        limited_decoder
            .read_to_end(&mut decompressed)
            .map_err(|_| {
                record_custom_contract_error(
                    &contract_name,
                    CustomContractEndpoint::LogsIngest,
                    CustomContractErrorType::InvalidPayload,
                );
                reject::custom(ServiceError::bad_request(
                    LogIngestError::UnexpectedContentEncoding.into(),
                ))
            })?;
        decompressed
    } else {
        body.to_vec()
    };

    // Parse the log batch
    let log_batch: Vec<String> = serde_json::from_slice(&log_data).map_err(|_| {
        record_custom_contract_error(
            &contract_name,
            CustomContractEndpoint::LogsIngest,
            CustomContractErrorType::InvalidPayload,
        );
        reject::custom(ServiceError::bad_request(
            LogIngestError::UnexpectedPayloadBody.into(),
        ))
    })?;

    debug!(
        "custom contract '{}' log batch size: {} from peer_id: {}, is_trusted: {}",
        contract_name,
        log_batch.len(),
        peer_id,
        is_trusted
    );

    // Get the custom contract instance
    let instance = context.get_custom_contract(&contract_name).ok_or_else(|| {
        error!("custom contract '{}' not configured", contract_name);
        record_custom_contract_error(
            &contract_name,
            CustomContractEndpoint::LogsIngest,
            CustomContractErrorType::ContractNotConfigured,
        );
        reject::custom(ServiceError::internal(
            LogIngestError::IngestionError.into(),
        ))
    })?;

    // Get the appropriate log client based on whether node is trusted (allowlisted)
    // Unknown/untrusted nodes go to untrusted_logs_sink if configured, else regular sink
    let log_client = instance.get_logs_client(is_trusted).ok_or_else(|| {
        debug!(
            "custom contract '{}' log client not configured",
            contract_name
        );
        record_custom_contract_error(
            &contract_name,
            CustomContractEndpoint::LogsIngest,
            CustomContractErrorType::IngestionFailed,
        );
        reject::custom(ServiceError::internal(
            LogIngestError::IngestionError.into(),
        ))
    })?;

    // Prepare unstructured log with custom contract metadata
    let mut fields = HashMap::new();
    fields.insert(PEER_ID_FIELD_NAME.into(), peer_id.to_string());

    // Get the node type from the contract instance config, marked as unknown if not trusted
    let node_type = if is_trusted {
        NodeType::Custom(instance.node_type_name.clone())
    } else {
        NodeType::CustomUnknown(instance.node_type_name.clone())
    };

    let trust_label = if is_trusted { "trusted" } else { "untrusted" };

    let mut tags = HashMap::new();
    tags.insert(PEER_ROLE_TAG_NAME.into(), node_type.as_str());
    tags.insert("contract_name".into(), contract_name.clone());
    tags.insert("trust_status".into(), trust_label.into());

    let unstructured_log = UnstructuredLog {
        fields,
        tags,
        messages: log_batch,
    };

    // Forward logs to the appropriate custom contract sink
    log_client
        .ingest_unstructured_log(unstructured_log)
        .await
        .map_err(|e| {
            debug!(
                "failed to ingest custom contract '{}' logs: {}",
                contract_name, e
            );
            record_custom_contract_error(
                &contract_name,
                CustomContractEndpoint::LogsIngest,
                CustomContractErrorType::IngestionFailed,
            );
            reject::custom(ServiceError::internal(
                LogIngestError::IngestionError.into(),
            ))
        })?;

    Ok(reply::reply())
}

/// Custom contract custom event ingest endpoint
pub fn custom_event_ingest(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path!("custom-contract" / String / "ingest" / "custom-event")
        .and(warp::post())
        .and(context.clone().filter())
        .and(with_custom_contract_auth(context.clone()))
        .and(warp::body::json())
        .and_then(handle_custom_event_ingest)
        .boxed()
}

/// Handle custom contract custom event ingestion
async fn handle_custom_event_ingest(
    contract_name: String,
    context: Context,
    (jwt_contract_name, peer_id, chain_id, is_trusted): (String, PeerId, ChainId, bool),
    body: TelemetryDump,
) -> Result<impl Reply, Rejection> {
    // Verify the JWT was issued for this specific contract (prevents cross-contract token reuse)
    if jwt_contract_name != contract_name {
        error!(
            "contract name mismatch: JWT issued for '{}', request for '{}'",
            jwt_contract_name, contract_name
        );
        record_custom_contract_error(
            &contract_name,
            CustomContractEndpoint::EventsIngest,
            CustomContractErrorType::TokenMismatch,
        );
        return Err(reject::custom(ServiceError::forbidden(
            ServiceErrorCode::CustomContractAuthError(
                format!(
                    "token issued for '{}' cannot be used for '{}'",
                    jwt_contract_name, contract_name
                ),
                chain_id,
            ),
        )));
    }

    // Check if the peer is blacklisted for this contract
    if let Some(instance) = context.get_custom_contract(&contract_name) {
        if instance.is_peer_blacklisted(&peer_id) {
            debug!(
                "peer_id {} is blacklisted from custom contract '{}' events",
                peer_id, contract_name
            );
            record_custom_contract_error(
                &contract_name,
                CustomContractEndpoint::EventsIngest,
                CustomContractErrorType::NotInAllowlist,
            );
            return Err(reject::custom(ServiceError::forbidden(
                ServiceErrorCode::CustomContractAuthError(
                    format!("peer_id {} is blacklisted from this contract", peer_id),
                    chain_id,
                ),
            )));
        }
    }

    // Apply rate limiting for untrusted nodes (events use logs rate limiter)
    // Check per-contract logs rate limiter first, then fall back to global logs rate limiter
    if !is_trusted {
        let contract_rate_limited = !context
            .contract_logs_rate_limiters()
            .check_rate_limit(&contract_name)
            .await;

        let use_global = !context
            .contract_logs_rate_limiters()
            .has_limiter(&contract_name)
            .await;
        let global_rate_limited =
            use_global && !context.unknown_logs_rate_limiter().check_rate_limit().await;

        if contract_rate_limited || global_rate_limited {
            debug!(
                "rate limit exceeded for untrusted custom contract '{}' events: peer_id={}",
                contract_name, peer_id
            );
            record_custom_contract_error(
                &contract_name,
                CustomContractEndpoint::EventsIngest,
                CustomContractErrorType::RateLimitExceeded,
            );
            return Err(reject::custom(ServiceError::too_many_requests(
                ServiceErrorCode::CustomContractAuthError(
                    "rate limit exceeded for untrusted telemetry".to_string(),
                    chain_id,
                ),
            )));
        }
    }

    debug!(
        "received custom contract '{}' custom event from peer_id: {}, chain_id: {}, is_trusted: {}, events: {}",
        contract_name,
        peer_id,
        chain_id,
        is_trusted,
        body.events.len()
    );

    // Validate the user_id matches the peer_id (parse to handle different string formats)
    let body_peer_id = PeerId::from_hex_literal(&body.user_id).map_err(|_| {
        record_custom_contract_error(
            &contract_name,
            CustomContractEndpoint::EventsIngest,
            CustomContractErrorType::InvalidPayload,
        );
        reject::custom(ServiceError::bad_request(
            CustomEventIngestError::InvalidEvent(body.user_id.clone(), peer_id).into(),
        ))
    })?;
    if body_peer_id != peer_id {
        record_custom_contract_error(
            &contract_name,
            CustomContractEndpoint::EventsIngest,
            CustomContractErrorType::InvalidPayload,
        );
        return Err(reject::custom(ServiceError::bad_request(
            CustomEventIngestError::InvalidEvent(body.user_id.clone(), peer_id).into(),
        )));
    }

    // Validate there are events
    if body.events.is_empty() {
        record_custom_contract_error(
            &contract_name,
            CustomContractEndpoint::EventsIngest,
            CustomContractErrorType::InvalidPayload,
        );
        return Err(reject::custom(ServiceError::bad_request(
            CustomEventIngestError::EmptyPayload.into(),
        )));
    }

    // Parse timestamp
    let event_timestamp: u64 = body.timestamp_micros.parse().map_err(|_| {
        record_custom_contract_error(
            &contract_name,
            CustomContractEndpoint::EventsIngest,
            CustomContractErrorType::InvalidPayload,
        );
        reject::custom(ServiceError::bad_request(
            CustomEventIngestError::InvalidTimestamp(body.timestamp_micros.clone()).into(),
        ))
    })?;

    // Get the custom contract instance
    let instance = context.get_custom_contract(&contract_name).ok_or_else(|| {
        error!("custom contract '{}' not configured", contract_name);
        record_custom_contract_error(
            &contract_name,
            CustomContractEndpoint::EventsIngest,
            CustomContractErrorType::ContractNotConfigured,
        );
        reject::custom(ServiceError::internal(
            CustomEventIngestError::EmptyPayload.into(),
        ))
    })?;

    // Get the BigQuery client for this custom contract
    // Note: BigQuery events don't have separate trusted/untrusted sinks yet
    // but we include trust_status as an event parameter for filtering
    if let Some(bq_client) = &instance.bigquery_client {
        use crate::types::telemetry::BigQueryRow;

        // Get the node type from the contract instance config, marked as unknown if not trusted
        let node_type = if is_trusted {
            NodeType::Custom(instance.node_type_name.clone())
        } else {
            NodeType::CustomUnknown(instance.node_type_name.clone())
        };

        let trust_label = if is_trusted { "trusted" } else { "untrusted" };

        // Create event identity for custom contract client using chain_id from JWT claims
        let event_identity = EventIdentity {
            peer_id,
            chain_id,
            role_type: node_type,
            epoch: 0,
            uuid: Uuid::new_v4(),
        };

        // Convert events to BigQuery rows and build insert request
        let mut insert_request = TableDataInsertAllRequest::new();

        for event in body.events {
            // Add contract_name and trust_status to event params
            let mut event_params: Vec<serde_json::Value> = event
                .params
                .into_iter()
                .map(|(key, value)| {
                    serde_json::json!({
                        "key": key,
                        "value": {"string_value": value}
                    })
                })
                .collect();
            // Append contract_name and trust_status as additional parameters
            event_params.push(serde_json::json!({
                "key": "contract_name",
                "value": {"string_value": contract_name.clone()}
            }));
            event_params.push(serde_json::json!({
                "key": "trust_status",
                "value": {"string_value": trust_label}
            }));

            let row = BigQueryRow {
                event_identity: event_identity.clone(),
                event_name: event.name,
                event_timestamp,
                event_params,
            };

            insert_request.add_row(None, &row).map_err(|e| {
                error!("unable to create BigQuery row: {}", e);
                record_custom_contract_error(
                    &contract_name,
                    CustomContractEndpoint::EventsIngest,
                    CustomContractErrorType::IngestionFailed,
                );
                reject::custom(ServiceError::internal(
                    CustomEventIngestError::from(e).into(),
                ))
            })?;
        }

        // Insert into BigQuery
        bq_client.insert_all(insert_request).await.map_err(|e| {
            error!("BigQuery insert failed: {}", e);
            record_custom_contract_error(
                &contract_name,
                CustomContractEndpoint::EventsIngest,
                CustomContractErrorType::IngestionFailed,
            );
            reject::custom(ServiceError::internal(
                CustomEventIngestError::from(e).into(),
            ))
        })?;
    }

    Ok(reply::reply())
}
