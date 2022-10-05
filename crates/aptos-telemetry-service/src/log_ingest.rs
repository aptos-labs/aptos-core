// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    auth::with_auth,
    clients::humio::{CHAIN_ID_TAG_NAME, EPOCH_FIELD_NAME, PEER_ID_FIELD_NAME, PEER_ROLE_TAG_NAME},
    constants::MAX_CONTENT_LENGTH,
    context::Context,
    error::ServiceError,
    types::{auth::Claims, common::NodeType, humio::UnstructuredLog},
};
use flate2::bufread::GzDecoder;
use reqwest::{header::CONTENT_ENCODING, StatusCode};
use std::collections::HashMap;
use tracing::{debug, error};
use warp::{filters::BoxedFilter, reject, reply, Buf, Filter, Rejection, Reply};

/// TODO: Cleanup after v1 API is ramped up
pub fn log_ingest_legacy(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path!("log_ingest")
        .and(warp::post())
        .and(context.clone().filter())
        .and(with_auth(
            context,
            vec![
                NodeType::Validator,
                NodeType::ValidatorFullNode,
                NodeType::PublicFullNode,
            ],
        ))
        .and(warp::header::optional(CONTENT_ENCODING.as_str()))
        .and(warp::body::content_length_limit(MAX_CONTENT_LENGTH))
        .and(warp::body::aggregate())
        .and_then(handle_log_ingest)
        .boxed()
}

pub fn log_ingest(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path!("ingest" / "logs")
        .and(warp::post())
        .and(context.clone().filter())
        .and(with_auth(
            context,
            vec![
                NodeType::Validator,
                NodeType::ValidatorFullNode,
                NodeType::PublicFullNode,
            ],
        ))
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

    let log_messages: Vec<String> = if let Some(encoding) = encoding {
        if encoding.eq_ignore_ascii_case("gzip") {
            let decoder = GzDecoder::new(body.reader());
            serde_json::from_reader(decoder).map_err(|e| {
                debug!("unable to decode and deserialize body: {}", e);
                ServiceError::bad_request("Unexpected payload body. Payload should be an array of strings possibly in gzip format.")
            })?
        } else {
            return Err(reject::custom(ServiceError::bad_request(
                "Unexpected content encoding. Supported encodings are: gzip.",
            )));
        }
    } else {
        serde_json::from_reader(body.reader()).map_err(|e| {
            error!("unable to deserialize body: {}", e);
            ServiceError::bad_request("Unexpected payload body. Payload should be an array of strings possibly in gzip format")
        })?
    };

    let mut fields = HashMap::new();
    fields.insert(PEER_ID_FIELD_NAME.into(), claims.peer_id.to_string());
    fields.insert(EPOCH_FIELD_NAME.into(), claims.epoch.to_string());

    let mut tags = HashMap::new();
    tags.insert(CHAIN_ID_TAG_NAME.into(), claims.chain_id.to_string());
    tags.insert(PEER_ROLE_TAG_NAME.into(), claims.node_type.to_string());

    let unstructured_log = UnstructuredLog {
        fields,
        tags,
        messages: log_messages,
    };

    debug!("ingesting to humio: {:?}", unstructured_log);

    let res = context
        .humio_client()
        .ingest_unstructured_log(unstructured_log)
        .await;

    match res {
        Ok(res) => {
            if res.status().is_success() {
                debug!("log ingested into humio succeessfully");
            } else {
                error!(
                    "humio log ingestion failed: {}",
                    res.error_for_status().err().unwrap()
                );
            }
        }
        Err(err) => {
            error!("error sending log ingest request: {}", err);
        }
    }

    Ok(reply::with_status(reply::reply(), StatusCode::CREATED))
}
