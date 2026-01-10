// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Prometheus Remote Write client.
//!
//! Implements the Prometheus Remote Write protocol for pushing metrics to Prometheus-compatible
//! backends. This uses the standard protobuf + snappy compression format.
//!
//! ## Protocol
//!
//! - Endpoint: `/api/v1/write`
//! - Content-Type: `application/x-protobuf`
//! - Content-Encoding: `snappy`
//! - Protocol version: `0.1.0`
//!
//! ## Reference
//!
//! - Spec: <https://prometheus.io/docs/specs/remote_write_spec/>
//! - Proto: <https://github.com/prometheus/prometheus/blob/main/prompb/remote.proto>
//!
//! ## Example Usage
//!
//! ```ignore
//! let client = PrometheusRemoteWriteClient::new(url, auth_token);
//!
//! // Parse Prometheus text format metrics and push via Remote Write
//! client.push_metrics(metrics_text, extra_labels).await?;
//! ```

pub use super::victoria_metrics::AuthToken;
use anyhow::{anyhow, Result};
use debug_ignore::DebugIgnore;
use flate2::read::GzDecoder;
use prost::Message;
use reqwest::Client as ReqwestClient;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use snap::raw::Encoder as SnappyEncoder;
use std::io::Read;
use url::Url;
use warp::hyper::body::Bytes;

// ============================================================================
// Prometheus Remote Write Protobuf Messages
// ============================================================================
// These match the proto definitions from prometheus/prometheus/prompb/types.proto
// and prompb/remote.proto

/// A label name-value pair.
#[derive(Clone, PartialEq, Message)]
pub struct Label {
    #[prost(string, tag = "1")]
    pub name: String,
    #[prost(string, tag = "2")]
    pub value: String,
}

/// A single metric sample: timestamp + value.
#[derive(Clone, PartialEq, Message)]
pub struct Sample {
    /// Value of the sample.
    #[prost(double, tag = "1")]
    pub value: f64,
    /// Timestamp in milliseconds since epoch.
    #[prost(int64, tag = "2")]
    pub timestamp: i64,
}

/// A time series: metric name + labels + samples.
#[derive(Clone, PartialEq, Message)]
pub struct TimeSeries {
    /// Labels (including __name__ for metric name).
    #[prost(message, repeated, tag = "1")]
    pub labels: Vec<Label>,
    /// Samples for this time series.
    #[prost(message, repeated, tag = "2")]
    pub samples: Vec<Sample>,
}

/// The Remote Write request containing multiple time series.
#[derive(Clone, PartialEq, Message)]
pub struct WriteRequest {
    #[prost(message, repeated, tag = "1")]
    pub timeseries: Vec<TimeSeries>,
}

// ============================================================================
// Constants (compatible with prometheus-reqwest-remote-write crate)
// ============================================================================

/// Content-Type header value for Remote Write requests.
pub const CONTENT_TYPE: &str = "application/x-protobuf";

/// Remote Write version header name.
pub const HEADER_NAME_REMOTE_WRITE_VERSION: &str = "X-Prometheus-Remote-Write-Version";

/// Remote Write protocol version 0.1.0.
pub const REMOTE_WRITE_VERSION_01: &str = "0.1.0";

/// Special label name for the metric name.
pub const LABEL_NAME: &str = "__name__";

// ============================================================================
// Prometheus Remote Write Client
// ============================================================================

/// Client for Prometheus Remote Write protocol.
///
/// Sends metrics using protobuf encoding with snappy compression to
/// Prometheus-compatible endpoints (`/api/v1/write`).
#[derive(Clone, Debug)]
pub struct PrometheusRemoteWriteClient {
    inner: DebugIgnore<ClientWithMiddleware>,
    base_url: Url,
    auth_token: AuthToken,
}

impl PrometheusRemoteWriteClient {
    /// Creates a new Remote Write client.
    pub fn new(base_url: Url, auth_token: AuthToken) -> Self {
        let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);
        let inner = ClientBuilder::new(ReqwestClient::new())
            .with(RetryTransientMiddleware::new_with_policy(retry_policy))
            .build();
        Self {
            inner: DebugIgnore(inner),
            base_url,
            auth_token,
        }
    }

    /// Returns true if the hostname contains "prometheus".
    pub fn is_selfhosted_prometheus_client(&self) -> bool {
        self.base_url
            .host_str()
            .unwrap_or_default()
            .contains("prometheus")
    }

    /// Alias for interface parity with VictoriaMetrics client.
    pub fn is_selfhosted_vm_client(&self) -> bool {
        self.is_selfhosted_prometheus_client()
    }

    /// Push metrics using Prometheus Remote Write protocol.
    ///
    /// Parses Prometheus text format metrics, converts to protobuf, compresses with snappy,
    /// and sends to the remote write endpoint.
    ///
    /// # Arguments
    /// * `raw_metrics_body` - Prometheus text format metrics (may be gzip compressed)
    /// * `extra_labels` - Additional labels to add to all metrics (format: "name=value")
    /// * `encoding` - Content encoding ("gzip" or "identity")
    pub async fn post_prometheus_metrics(
        &self,
        raw_metrics_body: Bytes,
        extra_labels: Vec<String>,
        encoding: String,
    ) -> Result<reqwest::Response, anyhow::Error> {
        // Decompress if gzip encoded
        let decompressed = if encoding == "gzip" {
            let mut decoder = GzDecoder::new(&raw_metrics_body[..]);
            let mut decompressed = Vec::new();
            decoder
                .read_to_end(&mut decompressed)
                .map_err(|e| anyhow!("gzip decompression failed: {}", e))?;
            decompressed
        } else {
            raw_metrics_body.to_vec()
        };

        // Parse Prometheus text format to WriteRequest
        let text = std::str::from_utf8(&decompressed)
            .map_err(|e| anyhow!("invalid UTF-8 in metrics body: {}", e))?;

        tracing::debug!(
            "Remote Write: parsing {} bytes of text: {:?}",
            text.len(),
            text
        );

        let write_request = self.parse_prometheus_text(text, &extra_labels)?;

        // Log what we're sending for debugging
        tracing::debug!(
            "Remote Write: sending {} timeseries to {}",
            write_request.timeseries.len(),
            self.base_url
        );
        for ts in &write_request.timeseries {
            let labels: Vec<_> = ts
                .labels
                .iter()
                .map(|l| format!("{}={}", l.name, l.value))
                .collect();
            tracing::debug!("  TimeSeries labels: [{}]", labels.join(", "));
        }

        // Encode to protobuf
        let mut proto_buf = Vec::with_capacity(write_request.encoded_len());
        write_request
            .encode(&mut proto_buf)
            .map_err(|e| anyhow!("protobuf encoding failed: {}", e))?;

        // Compress with snappy
        let mut snappy_encoder = SnappyEncoder::new();
        let compressed = snappy_encoder
            .compress_vec(&proto_buf)
            .map_err(|e| anyhow!("snappy compression failed: {}", e))?;

        // Build and send request - use base_url directly (config should include full path)
        let req = self.inner.0.post(self.base_url.as_str());
        let req = match &self.auth_token {
            AuthToken::None => req,
            AuthToken::Bearer(token) => req.bearer_auth(token.clone()),
            AuthToken::Basic(username, password) => {
                req.basic_auth(username.clone(), Some(password.clone()))
            },
        };

        req.header("Content-Type", CONTENT_TYPE)
            .header("Content-Encoding", "snappy")
            .header(HEADER_NAME_REMOTE_WRITE_VERSION, REMOTE_WRITE_VERSION_01)
            .body(compressed)
            .send()
            .await
            .map_err(|e| anyhow!("failed to post metrics: {}", e))
    }

    /// Parse Prometheus text format metrics into a WriteRequest.
    ///
    /// Handles HELP, TYPE comments and metric lines.
    /// Adds extra labels to all time series.
    pub fn parse_prometheus_text(
        &self,
        text: &str,
        extra_labels: &[String],
    ) -> Result<WriteRequest> {
        let mut timeseries_map: std::collections::HashMap<String, TimeSeries> =
            std::collections::HashMap::new();

        let timestamp = chrono::Utc::now().timestamp_millis();

        // Parse extra labels (format: "name=value")
        let extra_label_pairs: Vec<(String, String)> = extra_labels
            .iter()
            .filter_map(|s| {
                let mut parts = s.splitn(2, '=');
                match (parts.next(), parts.next()) {
                    (Some(name), Some(value)) => Some((name.to_string(), value.to_string())),
                    _ => None,
                }
            })
            .collect();

        for line in text.lines() {
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse metric line: metric_name{labels} value [timestamp]
            // or: metric_name value [timestamp]
            if let Some((metric_part, value_part)) = Self::split_metric_line(line) {
                let (metric_name, labels) = Self::parse_metric_name_and_labels(metric_part);
                // Value part may be "value" or "value timestamp" - just take the first number
                let value_str = value_part.split_whitespace().next().unwrap_or("");
                let value: f64 = match value_str.parse() {
                    Ok(v) => v,
                    Err(_) => continue, // Skip invalid values
                };

                // Build labels including __name__ and extra labels
                let mut all_labels = vec![Label {
                    name: LABEL_NAME.to_string(),
                    value: metric_name.clone(),
                }];
                all_labels.extend(labels);
                for (name, value) in &extra_label_pairs {
                    all_labels.push(Label {
                        name: name.clone(),
                        value: value.clone(),
                    });
                }

                // Sort labels by name (required by Prometheus)
                all_labels.sort_by(|a, b| a.name.cmp(&b.name));

                // Create a unique key for this label set
                let key = all_labels
                    .iter()
                    .map(|l| format!("{}={}", l.name, l.value))
                    .collect::<Vec<_>>()
                    .join(",");

                // Add sample to existing or new time series
                let ts = timeseries_map.entry(key).or_insert_with(|| TimeSeries {
                    labels: all_labels,
                    samples: Vec::new(),
                });
                ts.samples.push(Sample { value, timestamp });
            }
        }

        Ok(WriteRequest {
            timeseries: timeseries_map.into_values().collect(),
        })
    }

    /// Split a metric line into metric part and value part.
    fn split_metric_line(line: &str) -> Option<(&str, &str)> {
        // Find where the value starts (after } or after metric name)
        if let Some(brace_end) = line.rfind('}') {
            let rest = &line[brace_end + 1..];
            Some((&line[..=brace_end], rest))
        } else {
            // No labels: "metric_name value"
            let mut parts = line.splitn(2, char::is_whitespace);
            match (parts.next(), parts.next()) {
                (Some(name), Some(value)) => Some((name, value)),
                _ => None,
            }
        }
    }

    /// Parse metric name and labels from the metric part.
    fn parse_metric_name_and_labels(metric_part: &str) -> (String, Vec<Label>) {
        if let Some(brace_start) = metric_part.find('{') {
            let name = metric_part[..brace_start].to_string();
            let labels_str = &metric_part[brace_start + 1..metric_part.len() - 1];

            let labels = Self::parse_labels(labels_str);
            (name, labels)
        } else {
            (metric_part.trim().to_string(), Vec::new())
        }
    }

    /// Parse label string into Label structs.
    /// Format: name="value",name2="value2"
    ///
    /// Handles Prometheus text format escape sequences in label values:
    /// - `\"` -> `"`  (escaped double quote)
    /// - `\\` -> `\`  (escaped backslash)
    /// - `\n` -> newline
    pub fn parse_labels(labels_str: &str) -> Vec<Label> {
        let mut labels = Vec::new();
        let mut remaining = labels_str;

        while !remaining.is_empty() {
            // Find name=
            if let Some(eq_pos) = remaining.find('=') {
                let name = remaining[..eq_pos].trim().to_string();
                remaining = &remaining[eq_pos + 1..];

                // Find quoted value
                if remaining.starts_with('"') {
                    remaining = &remaining[1..];

                    // Find the closing quote, handling escaped quotes
                    if let Some((value, rest)) = Self::extract_quoted_value(remaining) {
                        labels.push(Label { name, value });
                        remaining = rest.trim_start_matches(',').trim();
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        labels
    }

    /// Extract a quoted value from the input, handling escape sequences.
    /// Returns (unescaped_value, remaining_input_after_closing_quote).
    fn extract_quoted_value(input: &str) -> Option<(String, &str)> {
        let mut value = String::new();
        let mut chars = input.char_indices().peekable();

        while let Some((idx, ch)) = chars.next() {
            match ch {
                '"' => {
                    // Found unescaped closing quote
                    return Some((value, &input[idx + 1..]));
                },
                '\\' => {
                    // Handle escape sequence
                    if let Some((_, next_ch)) = chars.next() {
                        match next_ch {
                            '"' => value.push('"'),   // \" -> "
                            '\\' => value.push('\\'), // \\ -> \
                            'n' => value.push('\n'),  // \n -> newline
                            _ => {
                                // Unknown escape - preserve as-is
                                value.push('\\');
                                value.push(next_ch);
                            },
                        }
                    } else {
                        // Backslash at end of string - preserve it
                        value.push('\\');
                    }
                },
                _ => value.push(ch),
            }
        }

        // No closing quote found
        None
    }

    pub fn base_url(&self) -> &Url {
        &self.base_url
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_prometheus_text_simple() {
        let client = PrometheusRemoteWriteClient::new(
            Url::parse("http://localhost:9090/").unwrap(),
            AuthToken::Bearer("test".to_string()),
        );

        let text = r#"
# HELP test_metric A test metric
# TYPE test_metric gauge
test_metric 42
test_metric_with_labels{label1="value1",label2="value2"} 123.45
"#;

        let result = client.parse_prometheus_text(text, &[]).unwrap();
        assert_eq!(result.timeseries.len(), 2);
    }

    #[test]
    fn test_parse_prometheus_text_with_extra_labels() {
        let client = PrometheusRemoteWriteClient::new(
            Url::parse("http://localhost:9090/").unwrap(),
            AuthToken::Bearer("test".to_string()),
        );

        let text = "test_metric 42";
        let extra_labels = vec!["peer_id=abc123".to_string(), "chain_id=1".to_string()];

        let result = client.parse_prometheus_text(text, &extra_labels).unwrap();
        assert_eq!(result.timeseries.len(), 1);

        let ts = &result.timeseries[0];
        // Should have __name__, chain_id, peer_id (sorted)
        assert!(ts.labels.iter().any(|l| l.name == LABEL_NAME));
        assert!(ts
            .labels
            .iter()
            .any(|l| l.name == "peer_id" && l.value == "abc123"));
        assert!(ts
            .labels
            .iter()
            .any(|l| l.name == "chain_id" && l.value == "1"));
    }

    #[test]
    fn test_parse_labels() {
        let labels =
            PrometheusRemoteWriteClient::parse_labels(r#"label1="value1",label2="value2""#);
        assert_eq!(labels.len(), 2);
        assert_eq!(labels[0].name, "label1");
        assert_eq!(labels[0].value, "value1");
        assert_eq!(labels[1].name, "label2");
        assert_eq!(labels[1].value, "value2");
    }

    #[test]
    fn test_parse_labels_with_escaped_quotes() {
        // Prometheus text format uses \" for escaped quotes in label values
        let labels =
            PrometheusRemoteWriteClient::parse_labels(r#"msg="value with \"quotes\" inside""#);
        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0].name, "msg");
        // The value should be unescaped: value with "quotes" inside
        assert_eq!(labels[0].value, r#"value with "quotes" inside"#);
    }

    #[test]
    fn test_parse_labels_with_escaped_backslash() {
        // Prometheus text format uses \\ for escaped backslash
        let labels = PrometheusRemoteWriteClient::parse_labels(r#"path="C:\\Users\\test""#);
        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0].name, "path");
        assert_eq!(labels[0].value, r#"C:\Users\test"#);
    }

    #[test]
    fn test_parse_labels_with_escaped_newline() {
        // Prometheus text format uses \n for newline
        let labels = PrometheusRemoteWriteClient::parse_labels(r#"msg="line1\nline2""#);
        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0].name, "msg");
        assert_eq!(labels[0].value, "line1\nline2");
    }

    #[test]
    fn test_parse_labels_multiple_with_escapes() {
        let labels = PrometheusRemoteWriteClient::parse_labels(
            r#"label1="normal",label2="has \"quotes\"",label3="also normal""#,
        );
        assert_eq!(labels.len(), 3);
        assert_eq!(labels[0].name, "label1");
        assert_eq!(labels[0].value, "normal");
        assert_eq!(labels[1].name, "label2");
        assert_eq!(labels[1].value, r#"has "quotes""#);
        assert_eq!(labels[2].name, "label3");
        assert_eq!(labels[2].value, "also normal");
    }
}
