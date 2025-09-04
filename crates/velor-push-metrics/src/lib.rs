// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

// Re-export counter types from prometheus crate
use velor_logger::{error, info, warn};
pub use velor_metrics_core::{
    exponential_buckets, register_histogram, register_histogram_vec, register_int_counter,
    register_int_counter_vec, register_int_gauge, register_int_gauge_vec, Histogram,
    HistogramTimer, HistogramVec, IntCounter, IntCounterVec, IntGauge, IntGaugeVec,
};
use velor_metrics_core::{Encoder, TextEncoder};
use std::{
    env,
    ops::Sub,
    sync::mpsc,
    thread,
    thread::JoinHandle,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use url::Url;

const DEFAULT_PUSH_FREQUENCY_SECS: u64 = 15;
const DEFAULT_DASHBOARD_BASE_URL: &str = "https://velorlabs.grafana.net/d/execution/execution";

/// MetricsPusher provides a function to push a list of Metrics to a configurable
/// pushgateway endpoint.
#[must_use = "Assign the contructed pusher to a variable, \
              otherwise the worker thread is joined immediately."]
pub struct MetricsPusher {
    worker_thread: Option<JoinHandle<()>>,
    quit_sender: mpsc::Sender<()>,
}

impl MetricsPusher {
    fn push(
        push_metrics_endpoint: &str,
        api_token: Option<&str>,
        push_metrics_extra_labels: &[String],
    ) {
        let mut buffer = Vec::new();

        if let Err(e) = TextEncoder::new().encode(&velor_metrics_core::gather(), &mut buffer) {
            error!("Failed to encode push metrics: {}.", e.to_string());
        } else {
            let mut request = ureq::post(push_metrics_endpoint);
            if let Some(token) = api_token {
                request.set("apikey", token);
            }
            push_metrics_extra_labels.iter().for_each(|label| {
                request.query("extra_label", label);
            });
            let response = request.timeout_connect(10_000).send_bytes(&buffer);
            if !response.ok() {
                warn!(
                    "Failed to push metrics to {},  resp: {}",
                    push_metrics_endpoint,
                    response.status_text()
                )
            }
        }
    }

    fn worker(
        quit_receiver: mpsc::Receiver<()>,
        push_metrics_endpoint: String,
        push_metrics_frequency_secs: u64,
        push_metrics_api_token: Option<String>,
        push_metrics_extra_labels: Vec<String>,
    ) {
        while quit_receiver
            .recv_timeout(Duration::from_secs(push_metrics_frequency_secs))
            .is_err()
        {
            // Timeout, no quit signal received.
            Self::push(
                &push_metrics_endpoint,
                push_metrics_api_token.as_deref(),
                &push_metrics_extra_labels,
            );
        }
        // final push
        Self::push(
            &push_metrics_endpoint,
            push_metrics_api_token.as_deref(),
            &push_metrics_extra_labels,
        );
    }

    fn start_worker_thread(
        quit_receiver: mpsc::Receiver<()>,
        push_metrics_extra_labels: Vec<String>,
    ) -> Option<JoinHandle<()>> {
        // eg value for PUSH_METRICS_ENDPOINT: "http://pushgateway.server.com:9091/metrics/job/safety_rules"
        let push_metrics_endpoint = match env::var("PUSH_METRICS_ENDPOINT") {
            Ok(s) => s,
            Err(_) => {
                info!("PUSH_METRICS_ENDPOINT env var is not set. Skipping sending metrics.");
                return None;
            },
        };
        let push_metrics_frequency_secs = match env::var("PUSH_METRICS_FREQUENCY_SECS") {
            Ok(s) => match s.parse::<u64>() {
                Ok(i) => i,
                Err(_) => {
                    error!("Invalid value for PUSH_METRICS_FREQUENCY_SECS: {}", s);
                    return None;
                },
            },
            Err(_) => DEFAULT_PUSH_FREQUENCY_SECS,
        };
        let push_metrics_api_token = env::var("PUSH_METRICS_API_TOKEN").ok();
        info!(
            "Starting push metrics loop. Sending metrics to {} with a frequency of {} seconds",
            push_metrics_endpoint, push_metrics_frequency_secs
        );

        Some(thread::spawn(move || {
            Self::worker(
                quit_receiver,
                push_metrics_endpoint,
                push_metrics_frequency_secs,
                push_metrics_api_token,
                push_metrics_extra_labels,
            )
        }))
    }

    fn get_dashboard_link(chain_name: &str, namespace: &str) -> Url {
        let end_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis()
            .to_string();
        let start_time = SystemTime::now()
            .sub(Duration::from_secs(600))
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis()
            .to_string();
        let mut url = Url::parse(
            &env::var("DASHBOARD_BASE_URL")
                .unwrap_or_else(|_| DEFAULT_DASHBOARD_BASE_URL.to_string()),
        )
        .unwrap();
        url.query_pairs_mut()
            .append_pair("from", &start_time)
            .append_pair("to", &end_time)
            .append_pair("var-Datasource", "fHo-R604z")
            .append_pair("var-metrics_source", "All")
            .append_pair("var-chain_name", chain_name)
            .append_pair("var-cluster", "All")
            .append_pair("var-namespace", namespace)
            .append_pair("var-kubernetes_pod_name", "All")
            .append_pair("var-role", "All");
        url
    }

    fn push_metrics_extra_labels(chain_name: &str, namespace: &str) -> Vec<String> {
        vec![
            format!("chain_name={}", chain_name),
            "cluster=unknown".into(),
            "metrics_source=unknown".into(),
            "kubernetes_pod_name=unknown".into(),
            "role=unknown".into(),
            format!("run_uuid={:x}", rand::random::<u64>()),
            format!("namespace={}", namespace),
        ]
    }

    /// start starts a new thread and periodically pushes the metrics to a pushgateway endpoint
    pub fn start(push_metrics_labels: Vec<String>) -> Self {
        let (tx, rx) = mpsc::channel();
        let worker_thread = Self::start_worker_thread(rx, push_metrics_labels);

        Self {
            worker_thread,
            quit_sender: tx,
        }
    }

    pub fn start_for_local_run(chain_name: &str) -> Self {
        let namespace = env::var("PUSH_METRICS_NAMESPACE").unwrap_or_else(|_| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
                .to_string()
        });
        let push_metrics_labels = Self::push_metrics_extra_labels(chain_name, &namespace);
        let pusher = Self::start(push_metrics_labels);

        info!(
            "Execution dashboard link: {} ",
            Self::get_dashboard_link(chain_name, &namespace)
        );
        pusher
    }

    pub fn join(&mut self) {
        if let Some(worker_thread) = self.worker_thread.take() {
            if let Err(e) = self.quit_sender.send(()) {
                error!(
                    "Failed to send quit signal to metric pushing worker thread: {:?}",
                    e
                );
            }
            if let Err(e) = worker_thread.join() {
                error!("Failed to join metric pushing worker thread: {:?}", e);
            }
        }
    }
}

impl Drop for MetricsPusher {
    #[allow(deprecated)]
    fn drop(&mut self) {
        self.join()
    }
}
