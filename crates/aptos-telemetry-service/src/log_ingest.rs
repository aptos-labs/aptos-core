// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    auth::with_auth,
    clients::humio::{
        CHAIN_ID_TAG_NAME, EPOCH_FIELD_NAME, PEER_ID_FIELD_NAME, PEER_ROLE_TAG_NAME,
        RUN_UUID_TAG_NAME,
    },
    constants::{MAX_CONTENT_LENGTH, MAX_DECOMPRESSED_LENGTH},
    context::Context,
    debug, error,
    errors::{LogIngestError, ServiceError},
    metrics::LOG_INGEST_BACKEND_REQUEST_DURATION,
    types::{auth::Claims, common::NodeType, humio::UnstructuredLog},
};
use flate2::bufread::GzDecoder;
use reqwest::{header::CONTENT_ENCODING, StatusCode};
use std::{collections::HashMap, io::Read};
use tokio::time::Instant;
use warp::{filters::BoxedFilter, reject, reply, Buf, Filter, Rejection, Reply};

pub fn log_ingest(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path!("ingest" / "logs")
        .and(warp::post())
        .and(context.clone().filter())
        .and(with_auth(context, vec![
            NodeType::Validator,
            NodeType::ValidatorFullNode,
            NodeType::PublicFullNode,
            NodeType::UnknownFullNode,
            NodeType::UnknownValidator,
        ]))
        .and(warp::header::optional(CONTENT_ENCODING.as_str()))
        .and(warp::body::content_length_limit(MAX_CONTENT_LENGTH))
        .and(warp::body::aggregate())
        .and_then(handle_log_ingest)
        .boxed()
}

pub async fn handle_log_ingest(
    context: Context,
    claims: Claims,
    encoding: Option<String>,
    body: impl Buf,
) -> anyhow::Result<impl Reply, Rejection> {
    debug!("handling log ingest");

    // Apply rate limiting for unknown/untrusted nodes
    let is_unknown = matches!(
        claims.node_type,
        NodeType::Unknown | NodeType::UnknownValidator | NodeType::UnknownFullNode
    );
    if is_unknown && !context.unknown_logs_rate_limiter().check_rate_limit().await {
        debug!(
            "rate limit exceeded for unknown node logs: peer_id={}",
            claims.peer_id
        );
        return Err(reject::custom(ServiceError::too_many_requests(
            LogIngestError::RateLimitExceeded.into(),
        )));
    }

    // Standard log ingestion requires humio_ingest_config to be configured
    let log_clients = context.log_ingest_clients().ok_or_else(|| {
        error!("Standard log ingestion not configured - rejecting request");
        reject::custom(ServiceError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            LogIngestError::IngestionError.into(),
        ))
    })?;

    if let Some(blacklist) = &log_clients.blacklist {
        if blacklist.contains(&claims.peer_id) {
            return Err(reject::custom(ServiceError::forbidden(
                LogIngestError::Forbidden(claims.peer_id).into(),
            )));
        }
    }

    let client = match claims.node_type {
        NodeType::Unknown | NodeType::UnknownValidator | NodeType::UnknownFullNode => {
            &log_clients.unknown_logs_ingest_client
        },
        _ => &log_clients.known_logs_ingest_client,
    };

    let log_messages: Vec<String> = if let Some(encoding) = encoding {
        if encoding.eq_ignore_ascii_case("gzip") {
            let decoder = GzDecoder::new(body.reader());
            // Limit decompressed size to prevent decompression bomb attacks
            let limited_reader = decoder.take(MAX_DECOMPRESSED_LENGTH as u64);
            serde_json::from_reader(limited_reader).map_err(|e| {
                debug!("unable to decode and deserialize body: {}", e);
                ServiceError::bad_request(LogIngestError::UnexpectedPayloadBody.into())
            })?
        } else {
            return Err(reject::custom(ServiceError::bad_request(
                LogIngestError::UnexpectedContentEncoding.into(),
            )));
        }
    } else {
        serde_json::from_reader(body.reader()).map_err(|e| {
            error!("unable to deserialize body: {}", e);
            ServiceError::bad_request(LogIngestError::UnexpectedPayloadBody.into())
        })?
    };

    let mut fields = HashMap::new();
    fields.insert(PEER_ID_FIELD_NAME.into(), claims.peer_id.to_string());
    fields.insert(EPOCH_FIELD_NAME.into(), claims.epoch.to_string());

    let mut tags = HashMap::new();
    let chain_name = if claims.chain_id.id() == 3 {
        format!("{}", claims.chain_id.id())
    } else {
        format!("{}", claims.chain_id)
    };
    tags.insert(CHAIN_ID_TAG_NAME.into(), chain_name);
    tags.insert(PEER_ROLE_TAG_NAME.into(), claims.node_type.to_string());
    tags.insert(RUN_UUID_TAG_NAME.into(), claims.run_uuid.to_string());

    let unstructured_log = UnstructuredLog {
        fields,
        tags,
        messages: log_messages,
    };

    debug!("ingesting to humio: {:?}", unstructured_log);

    let start_timer = Instant::now();

    let res = client.ingest_unstructured_log(unstructured_log).await;

    match res {
        Ok(res) => {
            LOG_INGEST_BACKEND_REQUEST_DURATION
                .with_label_values(&[res.status().as_str()])
                .observe(start_timer.elapsed().as_secs_f64());
            if res.status().is_success() {
                debug!("log ingested into humio succeessfully");
            } else {
                error!(
                    "humio log ingestion failed: {}",
                    res.error_for_status().err().unwrap()
                );
                return Err(reject::custom(ServiceError::bad_request(
                    LogIngestError::IngestionError.into(),
                )));
            }
        },
        Err(err) => {
            LOG_INGEST_BACKEND_REQUEST_DURATION
                .with_label_values(&["Unknown"])
                .observe(start_timer.elapsed().as_secs_f64());
            error!("error sending log ingest request: {}", err);
            return Err(reject::custom(ServiceError::bad_request(
                LogIngestError::IngestionError.into(),
            )));
        },
    }

    Ok(reply::with_status(reply::reply(), StatusCode::CREATED))
}
