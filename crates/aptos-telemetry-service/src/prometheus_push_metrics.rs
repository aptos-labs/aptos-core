// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    auth::with_auth,
    constants::MAX_CONTENT_LENGTH,
    context::Context,
    debug, error,
    errors::{MetricsIngestError, ServiceError},
    metrics::METRICS_INGEST_BACKEND_REQUEST_DURATION,
    types::{auth::Claims, common::NodeType},
};
use reqwest::{header::CONTENT_ENCODING, StatusCode};
use tokio::time::Instant;
use warp::{filters::BoxedFilter, hyper::body::Bytes, reject, reply, Filter, Rejection, Reply};

/// TODO: Cleanup after v1 API is ramped up
pub fn metrics_ingest_legacy(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path!("push-metrics")
        .and(warp::post())
        .and(context.clone().filter())
        .and(with_auth(context, vec![
            NodeType::Validator,
            NodeType::ValidatorFullNode,
            NodeType::PublicFullNode,
        ]))
        .and(warp::header::optional(CONTENT_ENCODING.as_str()))
        .and(warp::body::content_length_limit(MAX_CONTENT_LENGTH))
        .and(warp::body::bytes())
        .and_then(handle_metrics_ingest)
        .boxed()
}

pub fn metrics_ingest(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path!("ingest" / "metrics")
        .and(warp::post())
        .and(context.clone().filter())
        .and(with_auth(context, vec![
            NodeType::Validator,
            NodeType::ValidatorFullNode,
            NodeType::PublicFullNode,
            NodeType::UnknownValidator,
            NodeType::UnknownFullNode,
        ]))
        .and(warp::header::optional(CONTENT_ENCODING.as_str()))
        .and(warp::body::content_length_limit(MAX_CONTENT_LENGTH))
        .and(warp::body::bytes())
        .and_then(handle_metrics_ingest)
        .boxed()
}

pub async fn handle_metrics_ingest(
    context: Context,
    claims: Claims,
    encoding: Option<String>,
    metrics_body: Bytes,
) -> anyhow::Result<impl Reply, Rejection> {
    debug!("handling prometheus metrics ingest");

    let extra_labels = claims_to_extra_labels(
        &claims,
        context
            .peer_identities()
            .get(&claims.chain_id)
            .and_then(|peers| peers.get(&claims.peer_id)),
    );

    let client = match claims.node_type {
        NodeType::UnknownValidator | NodeType::UnknownFullNode => {
            &context.metrics_client().untrusted_ingest_metrics_clients
        },
        _ => &context.metrics_client().ingest_metrics_client,
    };

    let start_timer = Instant::now();

    let post_futures = client.iter().map(|(name, client)| async {
        let result = client
            .post_prometheus_metrics(
                metrics_body.clone(),
                extra_labels.clone(),
                encoding.clone().unwrap_or_default(),
            )
            .await;

        match result {
            Ok(res) => {
                METRICS_INGEST_BACKEND_REQUEST_DURATION
                    .with_label_values(&[&claims.peer_id.to_string(), name, res.status().as_str()])
                    .observe(start_timer.elapsed().as_secs_f64());
                if res.status().is_success() {
                    debug!("remote write to victoria metrics succeeded");
                } else {
                    error!(
                        "remote write failed to victoria_metrics for client {}: {}",
                        name.clone(),
                        res.error_for_status().err().unwrap()
                    );
                    return Err(());
                }
            },
            Err(err) => {
                METRICS_INGEST_BACKEND_REQUEST_DURATION
                    .with_label_values(&[name, "Unknown"])
                    .observe(start_timer.elapsed().as_secs_f64());
                error!(
                    "error sending remote write request for client {}: {}",
                    name.clone(),
                    err
                );
                return Err(());
            },
        }
        Ok(())
    });

    #[allow(clippy::unnecessary_fold)]
    if futures::future::join_all(post_futures)
        .await
        .iter()
        .all(|result| result.is_err())
    {
        return Err(reject::custom(ServiceError::internal(
            MetricsIngestError::IngestionError.into(),
        )));
    }

    Ok(reply::with_status(reply::reply(), StatusCode::CREATED))
}

fn claims_to_extra_labels(claims: &Claims, common_name: Option<&String>) -> Vec<String> {
    let chain_name = if claims.chain_id.id() == 3 {
        format!("chain_name={}", claims.chain_id.id())
    } else {
        format!("chain_name={}", claims.chain_id)
    };
    let pod_name = if let Some(common_name) = common_name {
        format!(
            "kubernetes_pod_name=peer_id:{}//{}",
            common_name,
            claims.peer_id.to_hex_literal()
        )
    } else {
        // for community nodes we cannot determine which pod name they run in (or whether they run in k8s at all),
        // so we use the peer id as an approximation/replacement for pod_name
        // This works well with our existing grafana dashboards
        format!(
            "kubernetes_pod_name=peer_id:{}",
            claims.peer_id.to_hex_literal()
        )
    };
    vec![
        format!("role={}", claims.node_type),
        format!("metrics_source={}", "telemetry-service"),
        chain_name,
        format!("namespace={}", "telemetry-service"),
        pod_name,
    ]
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{tests::test_context, MetricsClient};
    use aptos_types::{chain_id::ChainId, PeerId};
    use httpmock::MockServer;
    use reqwest::Url;
    use std::str::FromStr;

    #[test]
    fn verify_labels() {
        let claims = claims_to_extra_labels(
            &super::Claims {
                chain_id: ChainId::new(25),
                peer_id: PeerId::from_str("0x1").unwrap(),
                node_type: NodeType::Validator,
                epoch: 3,
                exp: 123,
                iat: 123,
            },
            Some(&String::from("test_name")),
        );
        assert_eq!(claims, vec![
            "role=validator",
            "metrics_source=telemetry-service",
            "chain_name=25",
            "namespace=telemetry-service",
            "kubernetes_pod_name=peer_id:test_name//0x1",
        ]);

        let claims = claims_to_extra_labels(
            &super::Claims {
                chain_id: ChainId::new(25),
                peer_id: PeerId::from_str("0x1").unwrap(),
                node_type: NodeType::Validator,
                epoch: 3,
                exp: 123,
                iat: 123,
            },
            None,
        );
        assert_eq!(claims, vec![
            "role=validator",
            "metrics_source=telemetry-service",
            "chain_name=25",
            "namespace=telemetry-service",
            "kubernetes_pod_name=peer_id:0x1",
        ]);
    }

    #[tokio::test]
    async fn test_metrics_ingest_all_success() {
        let mut test_context = test_context::new_test_context().await;
        let claims = Claims::test();
        let body = Bytes::from_static(b"hello");

        let server1 = MockServer::start();
        let mock1 = server1.mock(|when, then| {
            when.method("POST").path("/api/v1/import/prometheus");
            then.status(200);
        });

        let server2 = MockServer::start();
        let mock2 = server2.mock(|when, then| {
            when.method("POST").path("/api/v1/import/prometheus");
            then.status(200);
        });

        let clients = test_context.inner.metrics_client_mut();
        clients.ingest_metrics_client.insert(
            "default1".into(),
            MetricsClient::new(Url::parse(&server1.base_url()).unwrap(), "token1".into()),
        );
        clients.ingest_metrics_client.insert(
            "default2".into(),
            MetricsClient::new(Url::parse(&server2.base_url()).unwrap(), "token2".into()),
        );

        let result =
            handle_metrics_ingest(test_context.inner, claims, Some("gzip".into()), body).await;

        mock1.assert();
        mock2.assert();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_metrics_ingest_partial_success() {
        let mut test_context = test_context::new_test_context().await;
        let claims = Claims::test();
        let body = Bytes::from_static(b"hello");

        let server1 = MockServer::start();
        let mock1 = server1.mock(|when, then| {
            when.method("POST").path("/api/v1/import/prometheus");
            then.status(200);
        });

        let server2 = MockServer::start();
        let mock2 = server2.mock(|when, then| {
            when.method("POST").path("/api/v1/import/prometheus");
            then.status(500);
        });

        let clients = test_context.inner.metrics_client_mut();
        clients.ingest_metrics_client.insert(
            "default1".into(),
            MetricsClient::new(Url::parse(&server1.base_url()).unwrap(), "token1".into()),
        );
        clients.ingest_metrics_client.insert(
            "default2".into(),
            MetricsClient::new(Url::parse(&server2.base_url()).unwrap(), "token2".into()),
        );

        let result =
            handle_metrics_ingest(test_context.inner, claims, Some("gzip".into()), body).await;

        mock1.assert();
        mock2.assert_hits(4);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_metrics_ingest_all_failure() {
        let mut test_context = test_context::new_test_context().await;
        let claims = Claims::test();
        let body = Bytes::from_static(b"hello");

        let server1 = MockServer::start();
        let mock1 = server1.mock(|when, then| {
            when.method("POST").path("/api/v1/import/prometheus");
            then.status(500);
        });

        let server2 = MockServer::start();
        let mock2 = server2.mock(|when, then| {
            when.method("POST").path("/api/v1/import/prometheus");
            then.status(401);
        });

        let clients = test_context.inner.metrics_client_mut();
        clients.ingest_metrics_client.insert(
            "default1".into(),
            MetricsClient::new(Url::parse(&server1.base_url()).unwrap(), "token1".into()),
        );
        clients.ingest_metrics_client.insert(
            "default2".into(),
            MetricsClient::new(Url::parse(&server2.base_url()).unwrap(), "token2".into()),
        );

        let result =
            handle_metrics_ingest(test_context.inner, claims, Some("gzip".into()), body).await;

        mock1.assert_hits(4);
        mock2.assert();
        assert!(result.is_err());
    }
}
