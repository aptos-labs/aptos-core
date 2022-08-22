// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use std::future::Future;
use std::time::Duration;
use std::{
    env,
    time::{SystemTime, UNIX_EPOCH},
};

use futures::future;
use once_cell::sync::Lazy;
use rand::Rng;
use rand_core::OsRng;
use serde::Deserialize;
use tokio::{
    runtime::{Builder, Runtime},
    task::JoinHandle,
    time,
};
use uuid::Uuid;

use aptos_config::config::NodeConfig;
use aptos_logger::prelude::*;
use aptos_telemetry_service::types::telemetry::{TelemetryDump, TelemetryEvent};
use aptos_types::chain_id::ChainId;

use crate::constants::{
    ENV_APTOS_DISABLE_EXPERIMENTAL_PUSH_METRICS, ENV_TELEMETRY_SERVICE_URL,
    PROMETHEUS_PUSH_METRICS_FREQ_SECS,
};
use crate::{
    build_information::create_build_info_telemetry_event,
    constants::{
        APTOS_GA_API_SECRET, APTOS_GA_MEASUREMENT_ID, ENV_APTOS_DISABLE_TELEMETRY,
        ENV_GA_API_SECRET, ENV_GA_MEASUREMENT_ID, GA4_URL, HTTPBIN_URL,
        NODE_CORE_METRICS_FREQ_SECS, NODE_NETWORK_METRICS_FREQ_SECS, NODE_SYS_INFO_FREQ_SECS,
        TELEMETRY_SERVICE_URL,
    },
    core_metrics::create_core_metric_telemetry_event,
    metrics,
    network_metrics::create_network_metric_telemetry_event,
    sender::TelemetrySender,
    system_information::create_system_info_telemetry_event,
};

const IP_ADDRESS_KEY: &str = "IP_ADDRESS";
// The IP address key
const TELEMETRY_TOKEN_KEY: &str = "TELEMETRY_TOKEN";
// The telemetry token key
const UNKNOWN_METRIC_VALUE: &str = "UNKNOWN"; // The default for unknown metric values

/// The random token presented by the node to connect all
/// telemetry events.
/// TODO(joshlind): leverage real authentication!
static TELEMETRY_TOKEN: Lazy<String> = Lazy::new(|| {
    let mut rng = OsRng;
    let token = rng.gen::<u32>();
    format!("TOKEN_{:?}", token)
});

/// Returns true iff telemetry is disabled
fn telemetry_is_disabled() -> bool {
    env::var(ENV_APTOS_DISABLE_TELEMETRY).is_ok()
}

/// Temporary flag to control enabling/disabling prometheus push metrics
fn enable_experimental_prometheus_push_metrics() -> bool {
    !(telemetry_is_disabled() || env::var(ENV_APTOS_DISABLE_EXPERIMENTAL_PUSH_METRICS).is_ok())
}

/// Starts the telemetry service and returns the execution runtime.
/// Note: The service will not be created if telemetry is disabled.
pub fn start_telemetry_service(node_config: NodeConfig, chain_id: ChainId) -> Option<Runtime> {
    // Don't start the service if telemetry has been disabled
    if telemetry_is_disabled() {
        warn!("Aptos telemetry is disabled!");
        return None;
    }

    // Create the telemetry runtime
    let telemetry_runtime = Builder::new_multi_thread()
        .thread_name("aptos-telemetry")
        .enable_all()
        .build()
        .expect("Failed to create the Aptos Telemetry runtime!");

    // Spawn the telemetry service
    let peer_id = fetch_peer_id(&node_config);
    telemetry_runtime
        .handle()
        .spawn(spawn_telemetry_service(peer_id, chain_id, node_config));

    Some(telemetry_runtime)
}

/// Returns the peer id given the node config.
/// Returns UNKNOWN otherwise.
fn fetch_peer_id(node_config: &NodeConfig) -> String {
    match node_config.peer_id() {
        Some(peer_id) => peer_id.to_string(),
        None => UNKNOWN_METRIC_VALUE.into(),
    }
}

async fn run_function_periodically<Fut>(interval_seconds: u64, function_to_run: impl Fn() -> Fut)
where
    Fut: Future<Output = ()>,
{
    let mut interval = time::interval(Duration::from_secs(interval_seconds));
    loop {
        interval.tick().await;
        function_to_run().await;
    }
}

/// Spawns the dedicated telemetry service that operates periodically
async fn spawn_telemetry_service(peer_id: String, chain_id: ChainId, node_config: NodeConfig) {
    let telemetry_svc_url =
        env::var(ENV_TELEMETRY_SERVICE_URL).unwrap_or_else(|_| TELEMETRY_SERVICE_URL.into());

    let telemetry_sender = TelemetrySender::new(telemetry_svc_url, chain_id, &node_config);

    // Send build information once (only on startup)
    send_build_information(
        peer_id.clone(),
        chain_id.to_string().clone(),
        telemetry_sender.clone(),
    )
    .await;

    info!("Telemetry service started!");

    let stable_collection_fns = future::join3(
        // Periodically send system information
        run_function_periodically(NODE_SYS_INFO_FREQ_SECS, || {
            send_system_information(peer_id.clone(), telemetry_sender.clone())
        }),
        // Periodically send node core metrics
        run_function_periodically(NODE_CORE_METRICS_FREQ_SECS, || {
            send_node_core_metrics(peer_id.clone(), &node_config, telemetry_sender.clone())
        }),
        // Periodically send node network metrics
        run_function_periodically(NODE_NETWORK_METRICS_FREQ_SECS, || {
            send_node_network_metrics(peer_id.clone(), telemetry_sender.clone())
        }),
    );

    if enable_experimental_prometheus_push_metrics() {
        future::join(
            stable_collection_fns,
            // Periodically send ALL prometheus metrics (This replaces the previous core and network metrics implementation)
            run_function_periodically(PROMETHEUS_PUSH_METRICS_FREQ_SECS, || {
                telemetry_sender.try_push_prometheus_metrics()
            }),
        )
        .await;
    } else {
        stable_collection_fns.await;
    }
}

/// Collects and sends the build information via telemetry
async fn send_build_information(
    peer_id: String,
    chain_id: String,
    telemetry_sender: TelemetrySender,
) {
    let telemetry_event = create_build_info_telemetry_event(chain_id).await;
    send_telemetry_event_with_ip(peer_id, Some(telemetry_sender), telemetry_event).await;
}

/// Collects and sends the core node metrics via telemetry
async fn send_node_core_metrics(
    peer_id: String,
    node_config: &NodeConfig,
    telemetry_sender: TelemetrySender,
) {
    let telemetry_event = create_core_metric_telemetry_event(node_config).await;
    send_telemetry_event_with_ip(peer_id, Some(telemetry_sender), telemetry_event).await;
}

/// Collects and sends the node network metrics via telemetry
async fn send_node_network_metrics(peer_id: String, telemetry_sender: TelemetrySender) {
    let telemetry_event = create_network_metric_telemetry_event().await;
    send_telemetry_event_with_ip(peer_id, Some(telemetry_sender), telemetry_event).await;
}

/// Collects and sends the system information via telemetry
async fn send_system_information(peer_id: String, telemetry_sender: TelemetrySender) {
    let telemetry_event = create_system_info_telemetry_event().await;
    send_telemetry_event_with_ip(peer_id, Some(telemetry_sender), telemetry_event).await;
}

/// Fetches the IP address and sends the given telemetry event
/// along with the IP address. Also sends a randomly generated
/// token to help correlate metrics across events.
pub(crate) async fn send_telemetry_event_with_ip(
    peer_id: String,
    telemetry_sender: Option<TelemetrySender>,
    telemetry_event: TelemetryEvent,
) -> JoinHandle<()> {
    // Update the telemetry event with the ip address and random token
    let TelemetryEvent { name, mut params } = telemetry_event;
    params.insert(IP_ADDRESS_KEY.to_string(), get_origin_ip().await);
    params.insert(TELEMETRY_TOKEN_KEY.to_string(), TELEMETRY_TOKEN.clone());
    let telemetry_event = TelemetryEvent { name, params };

    // Send the telemetry event
    send_telemetry_event(peer_id, telemetry_sender, telemetry_event).await
}

/// Gets the IP origin of the machine by pinging a url.
/// If none is found, returns UNKNOWN.
async fn get_origin_ip() -> String {
    let resp = reqwest::get(HTTPBIN_URL).await;
    match resp {
        Ok(json) => match json.json::<OriginIP>().await {
            Ok(origin_ip) => origin_ip.origin,
            Err(_) => UNKNOWN_METRIC_VALUE.into(),
        },
        Err(_) => UNKNOWN_METRIC_VALUE.into(),
    }
}

/// Sends the given event and params to the telemetry endpoint
async fn send_telemetry_event(
    peer_id: String,
    telemetry_sender: Option<TelemetrySender>,
    telemetry_event: TelemetryEvent,
) -> JoinHandle<()> {
    // Parse the Google analytics env variables
    let api_secret =
        env::var(ENV_GA_API_SECRET).unwrap_or_else(|_| APTOS_GA_API_SECRET.to_string());
    let measurement_id =
        env::var(ENV_GA_MEASUREMENT_ID).unwrap_or_else(|_| APTOS_GA_MEASUREMENT_ID.to_string());

    // Create and send the telemetry dump
    let event_name = telemetry_event.name.clone();
    let timestamp_micros = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_micros().to_string(),
        Err(_) => UNKNOWN_METRIC_VALUE.into(),
    };
    let telemetry_dump = TelemetryDump {
        client_id: Uuid::new_v4().to_string(), // We generate a random client id for each request
        user_id: peer_id,
        timestamp_micros,
        events: vec![telemetry_event],
    };
    let _handle = spawn_telemetry_service_event_sender(
        event_name.clone(),
        telemetry_sender,
        telemetry_dump.clone(),
    );
    spawn_telemetry_event_sender(api_secret, measurement_id, event_name, telemetry_dump)
}

fn spawn_telemetry_service_event_sender(
    event_name: String,
    telemetry_sender: Option<TelemetrySender>,
    telemetry_dump: TelemetryDump,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        if telemetry_sender.is_none() {
            return;
        }
        telemetry_sender
            .unwrap()
            .send_metrics(event_name, telemetry_dump)
            .await;
    })
}

/// Spawns the telemetry event sender on a new thread to avoid blocking
fn spawn_telemetry_event_sender(
    api_secret: String,
    measurement_id: String,
    event_name: String,
    telemetry_dump: TelemetryDump,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        // Create a request client
        let client = reqwest::Client::new();

        // Send the request and wait for a response
        let send_result = client
            .post(format!(
                "{}?&measurement_id={}&api_secret={}",
                GA4_URL, measurement_id, api_secret
            ))
            .json::<TelemetryDump>(&telemetry_dump)
            .send()
            .await;

        // Process the response
        match send_result {
            Ok(response) => {
                let status_code = response.status();
                if status_code.is_success() {
                    debug!(
                        "Sent telemetry event {}, data: {:?}",
                        event_name, &telemetry_dump
                    );
                    metrics::increment_telemetry_successes(&event_name);
                } else {
                    error!(
                        "Failed to send telemetry event! Status: {}, event: {}.",
                        response.status(),
                        event_name
                    );
                    debug!("Failed telemetry response: {:?}", response.text().await);
                    metrics::increment_telemetry_failures(&event_name);
                }
            }
            Err(error) => {
                error!(
                    "Failed to send telemetry event: {}. Error: {:?}",
                    event_name, error
                );
                metrics::increment_telemetry_failures(&event_name);
            }
        }
    })
}

/// A json struct useful for fetching the machine origin/IP
#[derive(Deserialize)]
struct OriginIP {
    origin: String,
}
