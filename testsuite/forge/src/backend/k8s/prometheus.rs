// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{create_k8s_client, Get, K8sApi, Result};

use anyhow::bail;
use aptos_logger::info;
use k8s_openapi::api::core::v1::Secret;
use prometheus_http_query::{response::PromqlResult, Client as PrometheusClient};
use reqwest::{header, Client as HttpClient};
use std::{collections::BTreeMap, sync::Arc};

pub async fn get_prometheus_client() -> Result<PrometheusClient> {
    // read from the environment
    let kube_client = create_k8s_client().await;
    let secrets_api = Arc::new(K8sApi::<Secret>::from_client(
        kube_client,
        Some("default".to_string()),
    ));
    create_prometheus_client_from_environment(secrets_api).await
}

async fn create_prometheus_client_from_environment(
    secrets_api: Arc<dyn Get<Secret>>,
) -> Result<PrometheusClient> {
    let prom_url_env = std::env::var("PROMETHEUS_URL");
    let prom_token_env = std::env::var("PROMETHEUS_TOKEN");

    let (prom_url, prom_token) = match (prom_url_env.clone(), prom_token_env) {
        // if both variables are provided, use them, otherwise try inferring from environment
        (Ok(url), Ok(token)) => {
            info!("Creating prometheus client from environment variables");
            (url, Some(token))
        }
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
                            }
                            _ => {
                                bail!("Failed to read prometheus-read-only url and token");
                            }
                        }
                    } else {
                        bail!("Failed to read prometheus-read-only secret data");
                    }
                }
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
                }
            }
        }
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

pub fn construct_query_with_extra_labels(
    query: &str,
    labels_map: BTreeMap<String, String>,
) -> String {
    // edit the query string to insert swarm metadata
    let mut new_query = query.to_string();
    let mut label_start_idx = query.find('{').unwrap_or(query.len());
    if label_start_idx == query.len() {
        // add a new curly and insert after it
        new_query.insert_str(query.len(), "{}");
        label_start_idx += 1;
    } else {
        // add a comma prefix to the existing labels and insert before it
        label_start_idx += 1;
        new_query.insert(label_start_idx, ',');
    }

    let mut labels_strs = vec![];
    for (k, v) in labels_map {
        labels_strs.push(format!(r#"{}="{}""#, k, v));
    }

    let labels = labels_strs.join(",");

    // assume no collisions in Forge namespace
    new_query.insert_str(label_start_idx, &labels);
    new_query
}

pub async fn query_with_metadata(
    prom_client: &PrometheusClient,
    query: &str,
    time: Option<i64>,
    timeout: Option<i64>,
    labels_map: BTreeMap<String, String>,
) -> Result<PromqlResult> {
    let new_query = construct_query_with_extra_labels(query, labels_map);
    match prom_client.query(&new_query, time, timeout).await {
        Ok(r) => Ok(r),
        Err(e) => bail!(e),
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use k8s_openapi::ByteString;
    use kube::{api::ObjectMeta, error::ErrorResponse, Error as KubeError};
    use prometheus_http_query::Error as PrometheusError;
    use std::{
        env,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    struct MockSecretApi {
        secret: Option<Secret>,
    }

    impl MockSecretApi {
        fn from_secret(secret: Option<Secret>) -> Self {
            MockSecretApi { secret }
        }
    }

    #[async_trait]
    impl Get<Secret> for MockSecretApi {
        async fn get(&self, _name: &str) -> Result<Secret, KubeError> {
            match self.secret {
                Some(ref s) => Ok(s.clone()),
                None => Err(KubeError::Api(ErrorResponse {
                    status: "status".to_string(),
                    message: "message".to_string(),
                    reason: "reason".to_string(),
                    code: 404,
                })),
            }
        }
    }

    #[tokio::test]
    async fn test_create_client_secret() {
        let secret_api = Arc::new(MockSecretApi::from_secret(Some(Secret {
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
        })));

        create_prometheus_client_from_environment(secret_api)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_create_client_none() {
        let secret_api = Arc::new(MockSecretApi::from_secret(None));

        create_prometheus_client_from_environment(secret_api)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_create_client_env() {
        let secret_api = Arc::new(MockSecretApi::from_secret(None));

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
        let client = get_prometheus_client().await.unwrap();

        // try a simple instant query
        // if it fails to connect to a prometheus instance, skip the test
        let query = r#"container_cpu_usage_seconds_total{chain_name=~".*forge.*", pod="aptos-node-0-validator-0", container="validator"}"#;
        let response = client.query(query, None, None).await;
        match response {
            Ok(pres) => {
                println!("{:?}", pres);
            }
            Err(PrometheusError::Client(e)) => {
                println!("Skipping test. Failed to create prometheus client: {}", e);
                return;
            }
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
        // test when no existing labels
        let original_query = "aptos_connections";
        let mut labels_map = BTreeMap::new();
        labels_map.insert("a".to_string(), "a".to_string());
        labels_map.insert("some_label".to_string(), "blabla".to_string());
        let expected_query = r#"aptos_connections{a="a",some_label="blabla"}"#;
        let new_query = construct_query_with_extra_labels(original_query, labels_map);
        assert_eq!(expected_query, new_query);

        // test when existing labels
        let original_query = r#"aptos_connections{abc="123",def="456"}"#;
        let mut labels_map = BTreeMap::new();
        labels_map.insert("a".to_string(), "a".to_string());
        labels_map.insert("some_label".to_string(), "blabla".to_string());
        let expected_query = r#"aptos_connections{a="a",some_label="blabla",abc="123",def="456"}"#;
        let new_query = construct_query_with_extra_labels(original_query, labels_map);
        assert_eq!(expected_query, new_query);
    }
}
