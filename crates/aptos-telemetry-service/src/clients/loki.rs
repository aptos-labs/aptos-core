// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Loki log ingestion client
//!
//! This client sends logs to Grafana Loki using the push API.
//! Format: https://grafana.com/docs/loki/latest/api/#push-log-entries-to-loki

use anyhow::anyhow;
use debug_ignore::DebugIgnore;
use reqwest::{Client as ReqwestClient, Url};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Loki push request format
#[derive(Debug, Serialize, Deserialize)]
struct LokiPushRequest {
    streams: Vec<LokiStream>,
}

/// A single log stream with labels
#[derive(Debug, Serialize, Deserialize)]
struct LokiStream {
    /// Label set for this stream (e.g., {"job": "aptos", "peer_id": "0x123"})
    stream: HashMap<String, String>,
    /// Log entries as [timestamp_ns, log_line] pairs
    values: Vec<[String; 2]>,
}

/// Authentication configuration for Loki
#[derive(Clone, Debug)]
pub enum LokiAuth {
    /// No authentication
    None,
    /// Bearer token authentication
    Bearer(String),
    /// Basic authentication (username, password)
    Basic(String, String),
}

impl LokiAuth {
    /// Create from optional bearer token (for backward compatibility)
    pub fn from_bearer_token(token: Option<String>) -> Self {
        match token {
            Some(t) if !t.is_empty() => LokiAuth::Bearer(t),
            _ => LokiAuth::None,
        }
    }

    /// Create basic auth from "username:password" string
    pub fn from_basic_auth_string(creds: &str) -> Option<Self> {
        let parts: Vec<&str> = creds.splitn(2, ':').collect();
        if parts.len() == 2 {
            Some(LokiAuth::Basic(parts[0].to_string(), parts[1].to_string()))
        } else {
            None
        }
    }
}

#[derive(Clone, Debug)]
pub struct LokiIngestClient {
    inner: DebugIgnore<ClientWithMiddleware>,
    base_url: Url,
    auth: LokiAuth,
}

impl LokiIngestClient {
    /// Create a new Loki ingest client with optional bearer token (backward compatible)
    ///
    /// # Arguments
    /// * `base_url` - Base URL of Loki (e.g., http://loki:3100)
    /// * `auth_token` - Optional bearer token for authentication
    pub fn new(base_url: Url, auth_token: Option<String>) -> Self {
        Self::with_auth(base_url, LokiAuth::from_bearer_token(auth_token))
    }

    /// Create a new Loki ingest client with custom auth configuration
    ///
    /// # Arguments
    /// * `base_url` - Base URL of Loki (e.g., http://loki:3100)
    /// * `auth` - Authentication configuration
    pub fn with_auth(base_url: Url, auth: LokiAuth) -> Self {
        let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);
        let inner = ClientBuilder::new(ReqwestClient::new())
            .with(RetryTransientMiddleware::new_with_policy(retry_policy))
            .build();
        Self {
            inner: DebugIgnore(inner),
            base_url,
            auth,
        }
    }

    /// Ingest log messages with labels
    ///
    /// # Arguments
    /// * `messages` - Vector of log message strings
    /// * `labels` - Labels to attach to the log stream (e.g., peer_id, node_type, etc.)
    pub async fn ingest_logs(
        &self,
        messages: Vec<String>,
        labels: HashMap<String, String>,
    ) -> Result<reqwest::Response, anyhow::Error> {
        // Get current timestamp in nanoseconds
        let now_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| anyhow!("failed to get timestamp: {}", e))?
            .as_nanos()
            .to_string();

        // Convert messages to Loki format: [[timestamp_ns, log_line], ...]
        let values: Vec<[String; 2]> = messages
            .into_iter()
            .map(|msg| [now_ns.clone(), msg])
            .collect();

        let stream = LokiStream {
            stream: labels,
            values,
        };

        let request = LokiPushRequest {
            streams: vec![stream],
        };

        let json_body = serde_json::to_string(&request)
            .map_err(|e| anyhow!("unable to serialize json: {}", e))?;

        let req = self
            .inner
            .0
            .post(self.base_url.join("/loki/api/v1/push")?)
            .header("Content-Type", "application/json")
            .body(json_body);

        // Add authentication based on configured auth type
        let req = match &self.auth {
            LokiAuth::None => req,
            LokiAuth::Bearer(token) => req.bearer_auth(token),
            LokiAuth::Basic(username, password) => req.basic_auth(username, Some(password)),
        };

        req.send()
            .await
            .map_err(|e| anyhow!("failed to post logs to Loki: {}", e))
    }

    /// Ingest logs from an unstructured log format (for compatibility)
    ///
    /// This converts the Humio UnstructuredLog format to Loki format
    pub async fn ingest_unstructured_log(
        &self,
        unstructured_log: crate::types::humio::UnstructuredLog,
    ) -> Result<reqwest::Response, anyhow::Error> {
        // Combine fields and tags into a single label set
        let mut labels = HashMap::new();

        // Add all fields as labels
        for (key, value) in unstructured_log.fields {
            labels.insert(key, value);
        }

        // Add all tags as labels
        for (key, value) in unstructured_log.tags {
            labels.insert(key, value);
        }

        // Add a default job label if not present
        labels
            .entry("job".to_string())
            .or_insert_with(|| "aptos-telemetry".to_string());

        self.ingest_logs(unstructured_log.messages, labels).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loki_request_format() {
        let mut labels = HashMap::new();
        labels.insert("peer_id".to_string(), "0x123".to_string());
        labels.insert("node_type".to_string(), "storage_provider".to_string());

        let stream = LokiStream {
            stream: labels,
            values: vec![
                ["1234567890000000000".to_string(), "log line 1".to_string()],
                ["1234567890000000001".to_string(), "log line 2".to_string()],
            ],
        };

        let request = LokiPushRequest {
            streams: vec![stream],
        };

        let json = serde_json::to_string_pretty(&request).unwrap();
        println!("Loki request format:\n{}", json);

        // Verify it serializes correctly
        assert!(json.contains("streams"));
        assert!(json.contains("peer_id"));
        assert!(json.contains("log line 1"));
    }
}
