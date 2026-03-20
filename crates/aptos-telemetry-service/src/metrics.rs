// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    constants::{
        GCP_CLOUD_RUN_INSTANCE_ID_ENV, GCP_CLOUD_RUN_REVISION_ENV, GCP_CLOUD_RUN_SERVICE_ENV,
        GCP_SERVICE_PROJECT_ID_ENV,
    },
    context::MetricsIngestClient,
    debug, error,
};
use anyhow::anyhow;
use aptos_metrics_core::{
    register_histogram_vec, register_int_counter, register_int_counter_vec, register_int_gauge,
    register_int_gauge_vec, HistogramVec, IntCounter, IntCounterVec, IntGauge, IntGaugeVec,
};
use flate2::{write::GzEncoder, Compression};
use once_cell::sync::Lazy;
use std::{env, io::Write, time::Duration};
use tokio::time::{self, Instant};
use warp::hyper::body::Bytes;

const METRICS_EXPORT_FREQUENCY: Duration = Duration::from_secs(15);

// =============================================================================
// Cache Operation Enums (type-safe metric labels)
// =============================================================================

/// Challenge cache operation types for metrics
#[derive(Debug, Clone, Copy)]
pub enum ChallengeCacheOp {
    Store,
    VerifySuccess,
    VerifyNotFound,
    VerifyExpired,
    Evicted,
}

impl ChallengeCacheOp {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Store => "store",
            Self::VerifySuccess => "verify_success",
            Self::VerifyNotFound => "verify_not_found",
            Self::VerifyExpired => "verify_expired",
            Self::Evicted => "evicted",
        }
    }
}

/// Allowlist cache operation types for metrics
#[derive(Debug, Clone, Copy)]
pub enum AllowlistCacheOp {
    Hit,
    Miss,
    Update,
}

impl AllowlistCacheOp {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Hit => "hit",
            Self::Miss => "miss",
            Self::Update => "update",
        }
    }
}

/// Validator cache peer types for metrics
#[derive(Debug, Clone, Copy)]
pub enum ValidatorCachePeerType {
    Validator,
    ValidatorFullnode,
}

impl ValidatorCachePeerType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Validator => "validator",
            Self::ValidatorFullnode => "validator_fullnode",
        }
    }
}

pub(crate) static SERVICE_ERROR_COUNTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "telemetry_web_service_internal_error_counts",
        "Service errors returned by the telemety web service by error_code",
        &["error_code"]
    )
    .unwrap()
});

/// Custom contract endpoint error counter with contract_name label.
/// This allows distinguishing errors between different custom contracts and
/// separating them from standard telemetry endpoint errors.
pub(crate) static CUSTOM_CONTRACT_ERROR_COUNTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "telemetry_web_service_custom_contract_error_counts",
        "Errors from custom contract endpoints by contract name and error type",
        &["contract_name", "endpoint", "error_type"]
    )
    .unwrap()
});

/// Custom contract endpoint types for error metrics
#[derive(Debug, Clone, Copy)]
pub enum CustomContractEndpoint {
    AuthChallenge,
    Auth,
    MetricsIngest,
    LogsIngest,
    EventsIngest,
}

impl CustomContractEndpoint {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AuthChallenge => "auth_challenge",
            Self::Auth => "auth",
            Self::MetricsIngest => "metrics_ingest",
            Self::LogsIngest => "logs_ingest",
            Self::EventsIngest => "events_ingest",
        }
    }
}

/// Custom contract error types for metrics (simplified from ServiceErrorCode)
#[derive(Debug, Clone, Copy)]
pub enum CustomContractErrorType {
    ContractNotConfigured,
    ChallengeFailed,
    SignatureInvalid,
    NotInAllowlist,
    #[allow(dead_code)]
    TokenInvalid,
    TokenMismatch,
    IngestionFailed,
    InvalidPayload,
    RateLimitExceeded,
}

impl CustomContractErrorType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ContractNotConfigured => "contract_not_configured",
            Self::ChallengeFailed => "challenge_failed",
            Self::SignatureInvalid => "signature_invalid",
            Self::NotInAllowlist => "not_in_allowlist",
            Self::TokenInvalid => "token_invalid",
            Self::TokenMismatch => "token_mismatch",
            Self::IngestionFailed => "ingestion_failed",
            Self::InvalidPayload => "invalid_payload",
            Self::RateLimitExceeded => "rate_limit_exceeded",
        }
    }
}

/// Helper to record custom contract errors with all relevant labels
pub fn record_custom_contract_error(
    contract_name: &str,
    endpoint: CustomContractEndpoint,
    error_type: CustomContractErrorType,
) {
    CUSTOM_CONTRACT_ERROR_COUNTS
        .with_label_values(&[contract_name, endpoint.as_str(), error_type.as_str()])
        .inc();
}

pub(crate) static LOG_INGEST_BACKEND_REQUEST_DURATION: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "telemetry_web_service_log_ingest_backend_request_duration",
        "Number of log ingest backend requests by response code",
        &["response_code"]
    )
    .unwrap()
});

pub(crate) static METRICS_INGEST_BACKEND_REQUEST_DURATION: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "telemetry_web_service_metrics_ingest_backend_request_duration",
        "Number of metrics ingest backend requests by response code",
        &["peer_id", "endpoint_name", "response_code"]
    )
    .unwrap()
});

pub(crate) static BIG_QUERY_BACKEND_REQUEST_DURATION: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "telemetry_web_service_big_query_backend_request_duration",
        "Number of big query backend requests by response kind",
        &["kind"]
    )
    .unwrap()
});

pub(crate) static BIG_QUERY_REQUEST_TOTAL: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "big_query_request_total",
        "Total number of big query requests"
    )
    .unwrap()
});

pub(crate) static BIG_QUERY_REQUEST_FAILURES_TOTAL: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "big_query_request_failures_total",
        "Total number of big query request failures"
    )
    .unwrap()
});

pub(crate) static METRICS_EXPORT_DURATION: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "telemetry_web_service_metrics_export_duration",
        "Number of metrics export requests by response code",
        &["response_code"]
    )
    .unwrap()
});

pub(crate) static VALIDATOR_SET_UPDATE_SUCCESS_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "telemetry_web_service_validator_set_update_success_count",
        "Number of metrics validator set update successes",
        &["chain_id"]
    )
    .unwrap()
});

pub(crate) static VALIDATOR_SET_UPDATE_FAILED_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "telemetry_web_service_validator_set_update_failed_count",
        "Number of metrics validator set update failures",
        &["chain_id", "error_code"]
    )
    .unwrap()
});

// =============================================================================
// Cache Observability Metrics
// =============================================================================
// These metrics provide visibility into cache health and staleness.
// Alert on: now() - last_update_timestamp > threshold to detect stuck components.

/// Last update timestamp for validator set cache (unix seconds per chain_id)
pub(crate) static VALIDATOR_CACHE_LAST_UPDATE_TIMESTAMP: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "telemetry_web_service_validator_cache_last_update_timestamp_seconds",
        "Unix timestamp of last successful validator cache update (use now() - value for staleness)",
        &["chain_id"]
    )
    .unwrap()
});

/// Size of validator cache (number of peers per chain_id)
pub(crate) static VALIDATOR_CACHE_SIZE: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "telemetry_web_service_validator_cache_size",
        "Number of peers in validator cache",
        &["chain_id", "peer_type"]
    )
    .unwrap()
});

/// Challenge cache metrics - size and operation counts (per contract)
pub(crate) static CHALLENGE_CACHE_SIZE: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "telemetry_web_service_challenge_cache_size",
        "Total number of pending challenges in cache",
        &["contract_name"]
    )
    .unwrap()
});

pub(crate) static CHALLENGE_CACHE_KEYS: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "telemetry_web_service_challenge_cache_keys",
        "Number of unique keys (contract/chain/address combinations) in challenge cache",
        &["contract_name"]
    )
    .unwrap()
});

pub(crate) static CHALLENGE_CACHE_OPERATIONS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "telemetry_web_service_challenge_cache_operations_total",
        "Challenge cache operations by type and contract",
        &["contract_name", "operation"] // operation: use ChallengeCacheOp enum
    )
    .unwrap()
});

/// Last time a challenge was stored (for detecting stuck issuers per contract)
pub(crate) static CHALLENGE_CACHE_LAST_STORE_TIMESTAMP: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "telemetry_web_service_challenge_cache_last_store_timestamp_seconds",
        "Unix timestamp of last challenge store operation",
        &["contract_name"]
    )
    .unwrap()
});

/// Allowlist cache metrics - size and operation counts
pub(crate) static ALLOWLIST_CACHE_SIZE: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "telemetry_web_service_allowlist_cache_size",
        "Number of addresses in allowlist cache",
        &["contract_name", "chain_id"]
    )
    .unwrap()
});

pub(crate) static ALLOWLIST_CACHE_ENTRIES: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "telemetry_web_service_allowlist_cache_entries",
        "Total number of contract/chain entries in allowlist cache"
    )
    .unwrap()
});

pub(crate) static ALLOWLIST_CACHE_OPERATIONS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "telemetry_web_service_allowlist_cache_operations_total",
        "Allowlist cache operations by type and contract",
        &["contract_name", "chain_id", "operation"] // operation: use AllowlistCacheOp enum
    )
    .unwrap()
});

/// Last time allowlist cache was updated (per contract/chain)
pub(crate) static ALLOWLIST_CACHE_LAST_UPDATE_TIMESTAMP: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "telemetry_web_service_allowlist_cache_last_update_timestamp_seconds",
        "Unix timestamp of last allowlist cache update (use now() - value for staleness)",
        &["contract_name", "chain_id"]
    )
    .unwrap()
});

/// Allowlist cache update success counter (similar to validator set)
pub(crate) static ALLOWLIST_CACHE_UPDATE_SUCCESS_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "telemetry_web_service_allowlist_cache_update_success_count",
        "Number of successful allowlist cache updates by contract",
        &["contract_name"]
    )
    .unwrap()
});

/// Allowlist cache update failure counter (similar to validator set)
pub(crate) static ALLOWLIST_CACHE_UPDATE_FAILED_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "telemetry_web_service_allowlist_cache_update_failed_count",
        "Number of failed allowlist cache updates by contract and error type",
        &["contract_name", "error_type"]
    )
    .unwrap()
});

pub struct PrometheusExporter {
    project_id: String,
    service: String,
    revision: String,
    instance_id: String,
    client: MetricsIngestClient,
}

impl PrometheusExporter {
    pub fn new(client: MetricsIngestClient) -> Self {
        let service = env::var(GCP_CLOUD_RUN_SERVICE_ENV).unwrap_or_else(|_| "Unknown".into());
        let revision = env::var(GCP_CLOUD_RUN_REVISION_ENV).unwrap_or_else(|_| "Unknown".into());
        let instance_id =
            env::var(GCP_CLOUD_RUN_INSTANCE_ID_ENV).unwrap_or_else(|_| "Unknown".into());
        let project_id = env::var(GCP_SERVICE_PROJECT_ID_ENV).unwrap_or_else(|_| "Unknown".into());

        Self {
            project_id,
            service,
            revision,
            instance_id,
            client,
        }
    }

    pub fn run(self) {
        tokio::spawn(async move {
            let mut interval = time::interval(METRICS_EXPORT_FREQUENCY);
            loop {
                interval.tick().await;
                match self.gather_and_send().await {
                    Ok(()) => debug!("service metrics exported successfully"),
                    Err(err) => error!("error exporting metrics {}", err),
                }
            }
        });
    }

    async fn gather_and_send(&self) -> Result<(), anyhow::Error> {
        let scraped_metrics = prometheus::TextEncoder::new()
            .encode_to_string(&prometheus::default_registry().gather())
            .map_err(|e| anyhow!("text encoding error {}", e))?;

        let mut gzip_encoder = GzEncoder::new(Vec::new(), Compression::default());
        gzip_encoder
            .write_all(scraped_metrics.as_bytes())
            .map_err(|e| anyhow!("gzip encoding error {}", e))?;
        let metrics_body = gzip_encoder.finish()?;

        let extra_labels = vec![
            "namespace=telemetry-web-service".into(),
            format!("cloud_run_revision={}", self.revision),
            format!("cloud_run_service={}", self.service),
            format!("cloud_run_container_id={}", self.instance_id),
            format!("gcp_project_id={}", self.project_id),
        ];

        let start_timer = Instant::now();

        let res = self
            .client
            .post_prometheus_metrics(Bytes::from(metrics_body), extra_labels, "gzip".into())
            .await;

        match res {
            Ok(res) => {
                METRICS_EXPORT_DURATION
                    .with_label_values(&[res.status().as_str()])
                    .observe(start_timer.elapsed().as_millis() as f64);
                if !res.status().is_success() {
                    return Err(anyhow!(
                        "remote write failed to victoria_metrics: {}",
                        res.error_for_status().err().unwrap()
                    ));
                }
            },
            Err(err) => {
                METRICS_EXPORT_DURATION
                    .with_label_values(&["Unknown"])
                    .observe(start_timer.elapsed().as_millis() as f64);
                return Err(anyhow!("error sending remote write request: {}", err));
            },
        }

        Ok(())
    }
}
