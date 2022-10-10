// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::{env, io::Write, time::Duration};

use crate::{constants::GCP_CLOUD_RUN_INSTANCE_ID_ENV, debug, error};
use anyhow::anyhow;
use aptos_metrics_core::{
    register_histogram_vec, register_int_counter_vec, HistogramVec, IntCounterVec,
};
use flate2::{write::GzEncoder, Compression};
use once_cell::sync::Lazy;
use tokio::time::{self, Instant};
use warp::hyper::body::Bytes;

use crate::{
    clients::victoria_metrics_api,
    constants::{
        GCP_CLOUD_RUN_REVISION_ENV, GCP_CLOUD_RUN_SERVICE_ENV, GCP_SERVICE_PROJECT_ID_ENV,
    },
};

const METRICS_EXPORT_FREQUENCY: Duration = Duration::from_secs(15);

pub(crate) static SERVICE_ERROR_COUNTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "telemetry_web_service_internal_error_counts",
        "Service errors returned by the telemety web service by error_code",
        &["error_code"]
    )
    .unwrap()
});

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
        &["endpoint_name", "response_code"]
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

pub struct PrometheusExporter {
    project_id: String,
    service: String,
    revision: String,
    instance_id: String,
    client: victoria_metrics_api::Client,
}

impl PrometheusExporter {
    pub fn new(client: victoria_metrics_api::Client) -> Self {
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
            }
            Err(err) => {
                METRICS_EXPORT_DURATION
                    .with_label_values(&["Unknown"])
                    .observe(start_timer.elapsed().as_millis() as f64);
                return Err(anyhow!("error sending remote write request: {}", err));
            }
        }

        Ok(())
    }
}
