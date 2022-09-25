// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::Result;

use anyhow::bail;
use aptos_logger::info;
use prometheus_http_query::{response::PromqlResult, Client as PrometheusClient};
use reqwest::{header, Client as HttpClient};
use std::collections::BTreeMap;

pub fn get_prometheus_client() -> Result<PrometheusClient> {
    let prom_url =
        std::env::var("PROMETHEUS_URL").unwrap_or_else(|_| "http://127.0.0.1:9090".to_string());
    info!("Attempting to create prometheus client with: {} ", prom_url);

    // add auth header if specified
    let mut headers = header::HeaderMap::new();
    if let Ok(token) = std::env::var("PROMETHEUS_TOKEN") {
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
    use prometheus_http_query::Error as PrometheusError;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[tokio::test]
    async fn test_query_prometheus() {
        let client = get_prometheus_client().unwrap();

        // try a simple instant query
        // if it fails to connect to a prometheus instance, skip the test
        let query = r#"rate(container_cpu_usage_seconds_total{pod=~".*validator.*", container="validator"}[1m])"#;
        let response = client.query(query, None, None).await;
        match response {
            Ok(pres) => println!("{:?}", pres),
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
