// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{create_k8s_client, K8sApi, ReadWrite, Result};
use again::RetryPolicy;
use anyhow::{anyhow, bail};
use k8s_openapi::api::core::v1::Secret;
use log::{info, warn};
use once_cell::sync::Lazy;
use prometheus_http_query::{
    response::{PromqlResult, Sample},
    Client as PrometheusClient,
};
use reqwest::{header, Client as HttpClient};
use std::{collections::BTreeMap, sync::Arc, time::Duration};

static PROMETHEUS_RETRY_POLICY: Lazy<RetryPolicy> = Lazy::new(|| {
    RetryPolicy::exponential(Duration::from_millis(125))
        .with_max_retries(3)
        .with_jitter(true)
});

pub async fn get_prometheus_client() -> Result<PrometheusClient> {
    // read from the environment
    let kube_client = create_k8s_client().await?;
    let secrets_api = Arc::new(K8sApi::<Secret>::from_client(
        kube_client,
        Some("default".to_string()),
    ));
    create_prometheus_client_from_environment(secrets_api).await
}

async fn create_prometheus_client_from_environment(
    secrets_api: Arc<dyn ReadWrite<Secret>>,
) -> Result<PrometheusClient> {
    let prom_url_env = std::env::var("PROMETHEUS_URL");
    let prom_token_env = std::env::var("PROMETHEUS_TOKEN");

    let (prom_url, prom_token) = match (prom_url_env.clone(), prom_token_env) {
        // if both variables are provided, use them, otherwise try inferring from environment
        (Ok(url), Ok(token)) => {
            info!("Creating prometheus client from environment variables");
            (url, Some(token))
        },
        _ => {
            // try reading a cluster-local secret
            match secrets_api.get("prometheus-read-only").await {
                Ok(secret) => {
                    if let Some(data) = secret.data {
                        let prom_url_k8s_secret = data.get("url");
                        let prom_token_k8s_secret = data.get("token");
                        match (prom_url_k8s_secret, prom_token_k8s_secret) {
                            (Some(url), Some(token)) => {
                                info!("Creating prometheus client from kubernetes secret");
                                (
                                    String::from_utf8(url.0.clone()).unwrap(),
                                    Some(String::from_utf8(token.0.clone()).unwrap()),
                                )
                            },
                            _ => {
                                bail!("Failed to read prometheus-read-only url and token");
                            },
                        }
                    } else {
                        bail!("Failed to read prometheus-read-only secret data");
                    }
                },
                Err(e) => {
                    // There's no remote prometheus secret setup. Try reading from a local prometheus backend
                    info!("Failed to get prometheus-read-only secret: {}", e);
                    info!("Creating prometheus client from local");
                    // Try reading from remote prometheus first, otherwise assume it's local
                    if let Ok(prom_url_env) = prom_url_env {
                        (prom_url_env, None)
                    } else {
                        ("http://127.0.0.1:9090".to_string(), None)
                    }
                },
            }
        },
    };

    // add auth header if specified
    let mut headers = header::HeaderMap::new();
    if let Some(token) = prom_token {
        if let Ok(mut auth_value) =
            header::HeaderValue::from_str(format!("Bearer {}", token.as_str()).as_str())
        {
            auth_value.set_sensitive(true);
            headers.insert(header::AUTHORIZATION, auth_value);
        } else {
            bail!("Invalid prometheus token");
        }
    }

    let client = HttpClient::builder().default_headers(headers).build()?;
    match PrometheusClient::from(client, &prom_url) {
        Ok(c) => Ok(c),
        Err(e) => bail!("Failed to create client {}", e),
    }
}

/// Constructs a new query given a query and labels to add to each metric in the query
/// NOTE: for complex queries with many metric names, it is required to use an empty label selector "{}" to denote where the labels will go
pub fn construct_query_with_extra_labels(
    query: &str,
    labels_map: &BTreeMap<String, String>,
) -> String {
    // edit the query string to insert swarm metadata
    let mut new_query = "".to_string();

    let mut labels_strs = vec![];
    for (k, v) in labels_map {
        labels_strs.push(format!(r#"{}="{}""#, k, v));
    }
    let labels = labels_strs.join(",");

    let parts: Vec<&str> = query.split_inclusive('{').collect();
    if parts.len() == 1 {
        // no labels in query
        format!("{}{{{}}}", query, labels)
    } else {
        let mut parts_iter = parts.into_iter();
        let prev = parts_iter.next();
        new_query.push_str(prev.unwrap());

        for part in parts_iter {
            if part.starts_with('}') {
                // assume no collisions in Forge namespace
                new_query.push_str(&labels);
            } else {
                // assume no collisions in Forge namespace
                new_query.push_str(&labels);
                new_query.push(',');
            }
            new_query.push_str(part);
        }
        new_query
    }
}

pub async fn query_with_metadata(
    prom_client: &PrometheusClient,
    query: &str,
    time: Option<i64>,
    timeout: Option<i64>,
    labels_map: &BTreeMap<String, String>,
) -> Result<PromqlResult> {
    let new_query = construct_query_with_extra_labels(query, labels_map);
    let new_query_ref = &new_query;
    PROMETHEUS_RETRY_POLICY
        .retry(move || prom_client.query(new_query_ref, time, timeout))
        .await
        .map_err(|e| anyhow!("Failed to query prometheus for {}: {}", query, e))
}

pub async fn query_range_with_metadata(
    prom_client: &PrometheusClient,
    query: &str,
    start_time: i64,
    end_time: i64,
    internal_secs: f64,
    timeout: Option<i64>,
    labels_map: &BTreeMap<String, String>,
) -> Result<Vec<Sample>> {
    let new_query = construct_query_with_extra_labels(query, labels_map);
    let new_query_ref = &new_query;
    let r = PROMETHEUS_RETRY_POLICY
        .retry(move || {
            prom_client.query_range(new_query_ref, start_time, end_time, internal_secs, timeout)
        })
        .await
        .map_err(|e| {
            anyhow!(
                "Failed to query prometheus {}. start={}, end={}, query={}",
                e,
                start_time,
                end_time,
                new_query
            )
        })?;
    let range = r.as_range().ok_or_else(|| {
        anyhow!(
            "Failed to get range from prometheus response. start={}, end={}, query={}",
            start_time,
            end_time,
            new_query
        )
    })?;
    if range.is_empty() {
        warn!(
            "Missing data for start={}, end={}, query={}",
            start_time, end_time, new_query
        );
        return Ok(Vec::new());
    }
    if range.len() > 1 {
        bail!(
            "Expected only one range vector from prometheus, received {} ({:?}). start={}, end={}, query={}",
            range.len(),
            range,
            start_time,
            end_time,
            new_query
        );
    }
    Ok(range
        .first()
        .unwrap() // safe because we checked length above
        .samples()
        .to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MockK8sResourceApi;
    use k8s_openapi::ByteString;
    use kube::api::ObjectMeta;
    use prometheus_http_query::Error as PrometheusError;
    use std::{
        env,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[tokio::test]
    async fn test_create_client_secret() {
        let secret_api = Arc::new(MockK8sResourceApi::from_resource(Secret {
            metadata: ObjectMeta {
                name: Some("prometheus-read-only".to_string()),
                ..ObjectMeta::default()
            },
            data: Some(BTreeMap::from([
                (
                    "url".to_string(),
                    ByteString("http://prometheus.site".to_string().into_bytes()),
                ),
                (
                    "token".to_string(),
                    ByteString("token".to_string().into_bytes()),
                ),
            ])),
            string_data: None,
            type_: None,
            immutable: None,
        }));

        create_prometheus_client_from_environment(secret_api)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_create_client_none() {
        let secret_api = Arc::new(MockK8sResourceApi::new());

        create_prometheus_client_from_environment(secret_api)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_create_client_env() {
        let secret_api = Arc::new(MockK8sResourceApi::new());

        env::set_var("PROMETHEUS_URL", "http://prometheus.site");

        // this is the worst case and will default to local (and not panic)
        create_prometheus_client_from_environment(secret_api.clone())
            .await
            .unwrap();

        env::set_var("PROMETHEUS_TOKEN", "token");

        // this should use the envs
        create_prometheus_client_from_environment(secret_api)
            .await
            .unwrap();

        // cleanup
        env::remove_var("PROMETHEUS_URL");
        env::remove_var("PROMETHEUS_TOKEN");
    }

    #[tokio::test]
    async fn test_query_prometheus() {
        let client_result = get_prometheus_client().await;

        // Currently this test tries to connect to the internet... and doesnt
        // require success so it is likely to be skipped
        // We should come back with some abstractions to make this more testable
        let client = if let Ok(client) = client_result {
            client
        } else {
            println!("Skipping test. Failed to create prometheus client");
            return;
        };

        // try a simple instant query
        // if it fails to connect to a prometheus instance, skip the test
        let query = r#"container_cpu_usage_seconds_total{chain_name=~".*forge.*", pod="aptos-node-0-validator-0", container="validator"}"#;
        let response = client.query(query, None, None).await;
        match response {
            Ok(pres) => {
                println!("{:?}", pres);
            },
            Err(PrometheusError::Client(e)) => {
                println!("Skipping test. Failed to create prometheus client: {}", e);
                return;
            },
            Err(e) => panic!("Expected PromqlResult: {}", e),
        }

        // try a range query
        let start = SystemTime::now();
        let since_the_epoch = start
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();
        let start_timestamp: i64 = (since_the_epoch - 60) as i64;
        let end_timestamp: i64 = since_the_epoch as i64;
        let step = 15.0;

        let response = client
            .query_range(query, start_timestamp, end_timestamp, step, None)
            .await;
        match response {
            Ok(pres) => println!("{:?}", pres),
            _ => panic!("Expected PromqlResult"),
        }
    }

    #[test]
    fn test_create_query() {
        let mut labels_map = BTreeMap::new();
        labels_map.insert("a".to_string(), "a".to_string());
        labels_map.insert("some_label".to_string(), "blabla".to_string());

        // test when no existing labels
        let original_query = "aptos_connections";
        let expected_query = r#"aptos_connections{a="a",some_label="blabla"}"#;
        let new_query = construct_query_with_extra_labels(original_query, &labels_map);
        assert_eq!(expected_query, new_query);

        // test when empty labels
        let original_query = "aptos_connections{}";
        let expected_query = r#"aptos_connections{a="a",some_label="blabla"}"#;
        let new_query = construct_query_with_extra_labels(original_query, &labels_map);
        assert_eq!(expected_query, new_query);

        // test when existing labels
        let original_query = r#"aptos_connections{abc="123",def="456"}"#;
        let expected_query = r#"aptos_connections{a="a",some_label="blabla",abc="123",def="456"}"#;
        let new_query = construct_query_with_extra_labels(original_query, &labels_map);
        assert_eq!(expected_query, new_query);

        // test when multiple queries
        let original_query = r#"aptos_connections{abc="123",def="456"} - aptos_disconnects{abc="123"} / aptos_count{}"#;
        let expected_query = r#"aptos_connections{a="a",some_label="blabla",abc="123",def="456"} - aptos_disconnects{a="a",some_label="blabla",abc="123"} / aptos_count{a="a",some_label="blabla"}"#;
        let new_query = construct_query_with_extra_labels(original_query, &labels_map);
        assert_eq!(expected_query, new_query);

        // test when empty labels and parens
        let original_query = "sum(rate(aptos_connections{}[1m])) by (network_id, role_type)";
        let expected_query = r#"sum(rate(aptos_connections{a="a",some_label="blabla"}[1m])) by (network_id, role_type)"#;
        let new_query = construct_query_with_extra_labels(original_query, &labels_map);
        assert_eq!(expected_query, new_query);

        // test when multiple queries and labels
        let original_query = "sum(rate(aptos_connections{role_type='validator'}[1m])) / sum(aptos_network_peers{role_type='validator'})";
        let expected_query = r#"sum(rate(aptos_connections{a="a",some_label="blabla",role_type='validator'}[1m])) / sum(aptos_network_peers{a="a",some_label="blabla",role_type='validator'})"#;
        let new_query = construct_query_with_extra_labels(original_query, &labels_map);
        assert_eq!(expected_query, new_query);
    }
}
