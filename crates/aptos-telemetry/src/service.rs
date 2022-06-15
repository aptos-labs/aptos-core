// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{
    build_information::create_build_info_telemetry_event,
    constants::{
        APTOS_GA_API_SECRET, APTOS_GA_MEASUREMENT_ID, ENV_APTOS_DISABLE_TELEMETRY,
        ENV_GA_API_SECRET, ENV_GA_MEASUREMENT_ID, GA4_URL, HTTPBIN_URL,
        NODE_CORE_METRICS_FREQ_SECS, NODE_NETWORK_METRICS_FREQ_SECS, NODE_SYS_INFO_FREQ_SECS,
    },
    core_metrics::create_core_metric_telemetry_event,
    metrics,
    network_metrics::create_network_metric_telemetry_event,
    system_information::create_system_info_telemetry_event,
};
use aptos_config::config::{NodeConfig, RoleType};
use aptos_logger::prelude::*;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    env,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::{
    runtime::{Builder, Runtime},
    task::JoinHandle,
};
use tokio_stream::wrappers::IntervalStream;
use uuid::Uuid;

const IP_ADDRESS_KEY: &str = "IP_ADDRESS"; // The IP address key
const UNKNOWN_METRIC_VALUE: &str = "UNKNOWN"; // The default for unknown metric values

/// Returns true iff telemetry is disabled
fn telemetry_is_disabled() -> bool {
    env::var(ENV_APTOS_DISABLE_TELEMETRY).is_ok()
}

/// Starts the telemetry service and returns the execution runtime.
/// Note: The service will not be created if telemetry is disabled.
pub fn start_telemetry_service(node_config: NodeConfig, chain_id: String) -> Option<Runtime> {
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
    let node_role_type = node_config.base.role;
    telemetry_runtime
        .handle()
        .spawn(spawn_telemetry_service(peer_id, chain_id, node_role_type));

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

/// Spawns the dedicated telemetry service that operates periodically
async fn spawn_telemetry_service(peer_id: String, chain_id: String, node_role_type: RoleType) {
    // Send build information once (only on startup)
    send_build_information(peer_id.clone(), chain_id.clone()).await;

    // Periodically send node core metrics
    let mut core_metrics_interval = IntervalStream::new(tokio::time::interval(
        std::time::Duration::from_secs(NODE_CORE_METRICS_FREQ_SECS),
    ))
    .fuse();

    // Periodically send node network metrics
    let mut network_metrics_interval = IntervalStream::new(tokio::time::interval(
        std::time::Duration::from_secs(NODE_NETWORK_METRICS_FREQ_SECS),
    ))
    .fuse();

    // Periodically send system information
    let mut system_information_interval = IntervalStream::new(tokio::time::interval(
        std::time::Duration::from_secs(NODE_SYS_INFO_FREQ_SECS),
    ))
    .fuse();

    info!("Telemetry service started!");
    loop {
        futures::select! {
            _ = system_information_interval.select_next_some() => {
                send_system_information(peer_id.clone()).await;
            }
            _ = core_metrics_interval.select_next_some() => {
                send_node_core_metrics(peer_id.clone(), node_role_type).await;
            }
            _ = network_metrics_interval.select_next_some() => {
                send_node_network_metrics(peer_id.clone()).await;
            }
        }
    }
}

/// Collects and sends the build information via telemetry
async fn send_build_information(peer_id: String, chain_id: String) {
    let telemetry_event = create_build_info_telemetry_event(chain_id).await;
    let _join_handle = send_telemetry_event_with_ip(peer_id, telemetry_event).await;
}

/// Collects and sends the core node metrics via telemetry
async fn send_node_core_metrics(peer_id: String, node_role_type: RoleType) {
    let telemetry_event = create_core_metric_telemetry_event(node_role_type).await;
    let _join_handle = send_telemetry_event_with_ip(peer_id, telemetry_event).await;
}

/// Collects and sends the node network metrics via telemetry
async fn send_node_network_metrics(peer_id: String) {
    let telemetry_event = create_network_metric_telemetry_event().await;
    let _join_handle = send_telemetry_event_with_ip(peer_id, telemetry_event).await;
}

/// Collects and sends the system information via telemetry
async fn send_system_information(peer_id: String) {
    let telemetry_event = create_system_info_telemetry_event().await;
    let _join_handle = send_telemetry_event_with_ip(peer_id, telemetry_event).await;
}

/// Fetches the IP address and sends the given telemetry event
/// along with the IP address.
pub(crate) async fn send_telemetry_event_with_ip(
    peer_id: String,
    telemetry_event: TelemetryEvent,
) -> JoinHandle<()> {
    // Fetch the IP address and update the telemetry event
    let TelemetryEvent { name, mut params } = telemetry_event;
    params.insert(IP_ADDRESS_KEY.to_string(), get_origin_ip().await);
    let telemetry_event = TelemetryEvent { name, params };

    // Send the telemetry event
    send_telemetry_event(peer_id, telemetry_event).await
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
async fn send_telemetry_event(peer_id: String, telemetry_event: TelemetryEvent) -> JoinHandle<()> {
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
    spawn_telemetry_event_sender(api_secret, measurement_id, event_name, telemetry_dump)
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
                let status_code = response.status().as_u16();
                if status_code > 200 && status_code < 299 {
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

/// A useful struct for serialization a telemetry event
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct TelemetryEvent {
    pub(crate) name: String,
    pub(crate) params: BTreeMap<String, String>,
}

/// A useful struct for serializing a telemetry dump
#[derive(Debug, Serialize, Deserialize)]
struct TelemetryDump {
    client_id: String,
    user_id: String,
    timestamp_micros: String,
    events: Vec<TelemetryEvent>,
}
