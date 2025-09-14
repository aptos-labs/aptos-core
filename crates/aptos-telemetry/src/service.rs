// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    constants::*, core_metrics::create_core_metric_telemetry_event, metrics,
    network_metrics::create_network_metric_telemetry_event, sender::TelemetrySender,
    system_information::create_system_info_telemetry_event,
    telemetry_log_sender::TelemetryLogSender, utils::create_build_info_telemetry_event,
};
use aptos_config::config::NodeConfig;
use aptos_logger::{
    aptos_logger::RUST_LOG_TELEMETRY, prelude::*, telemetry_log_writer::TelemetryLog,
    LoggerFilterUpdater,
};
use aptos_telemetry_service::types::telemetry::{TelemetryDump, TelemetryEvent};
use aptos_types::chain_id::ChainId;
use futures::channel::mpsc::{self, Receiver};
use once_cell::sync::Lazy;
use rand::Rng;
use rand_core::OsRng;
use reqwest::Url;
use std::{
    collections::BTreeMap,
    env,
    future::Future,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{runtime::Runtime, task::JoinHandle, time};
use uuid::Uuid;

// The chain ID key
const CHAIN_ID_KEY: &str = "CHAIN_ID";
// The telemetry token key
const TELEMETRY_TOKEN_KEY: &str = "TELEMETRY_TOKEN";
// The default for unknown metric values
const UNKNOWN_METRIC_VALUE: &str = "UNKNOWN";

const APTOS_NODE_CONFIG_EVENT_NAME: &str = "APTOS_NODE_CONFIG";

/// The random token presented by the node to connect all
/// telemetry events.
/// TODO(joshlind): leverage real authentication!
static TELEMETRY_TOKEN: Lazy<String> = Lazy::new(|| {
    let mut rng = OsRng;
    let token = rng.r#gen::<u32>();
    format!("TOKEN_{:?}", token)
});

/// Returns true iff telemetry is disabled
#[inline]
pub fn telemetry_is_disabled() -> bool {
    env::var(ENV_APTOS_DISABLE_TELEMETRY).is_ok()
}

/// Flag to force enabling/disabling of telemetry
#[inline]
fn force_enable_telemetry() -> bool {
    env::var(ENV_APTOS_FORCE_ENABLE_TELEMETRY).is_ok()
}

/// Flag to control enabling/disabling prometheus push metrics
#[inline]
fn enable_prometheus_push_metrics() -> bool {
    force_enable_telemetry()
        || !(telemetry_is_disabled() || env::var(ENV_APTOS_DISABLE_TELEMETRY_PUSH_METRICS).is_ok())
}

#[inline]
fn enable_prometheus_node_metrics() -> bool {
    env::var(ENV_APTOS_DISABLE_PROMETHEUS_NODE_METRICS).is_err()
}

/// Flag to control enabling/disabling push logs
#[inline]
fn enable_push_logs() -> bool {
    force_enable_telemetry()
        || !(telemetry_is_disabled() || env::var(ENV_APTOS_DISABLE_TELEMETRY_PUSH_LOGS).is_ok())
}

/// Flag to control enabling/disabling telemetry push events
#[inline]
fn enable_push_custom_events() -> bool {
    force_enable_telemetry()
        || !(telemetry_is_disabled() || env::var(ENV_APTOS_DISABLE_TELEMETRY_PUSH_EVENTS).is_ok())
}

#[inline]
fn enable_log_env_polling() -> bool {
    force_enable_telemetry()
        || !(telemetry_is_disabled() || env::var(ENV_APTOS_DISABLE_LOG_ENV_POLLING).is_ok())
}

/// Starts the telemetry service and returns the execution runtime.
/// Note: The service will not be created if telemetry is disabled.
pub fn start_telemetry_service(
    node_config: NodeConfig,
    chain_id: ChainId,
    build_info: BTreeMap<String, String>,
    remote_log_rx: Option<mpsc::Receiver<TelemetryLog>>,
    logger_filter_update_job: Option<LoggerFilterUpdater>,
) -> Option<Runtime> {
    if enable_prometheus_node_metrics() {
        aptos_node_resource_metrics::register_node_metrics_collector(Some(&build_info));
    }

    // Don't start the service if telemetry has been disabled
    if telemetry_is_disabled() {
        warn!("Aptos telemetry is disabled!");
        return None;
    }

    // Create the telemetry runtime
    let telemetry_runtime = aptos_runtimes::spawn_named_runtime("telemetry".into(), None);
    telemetry_runtime.handle().spawn(spawn_telemetry_service(
        node_config,
        chain_id,
        build_info,
        remote_log_rx,
        logger_filter_update_job,
    ));

    Some(telemetry_runtime)
}

async fn spawn_telemetry_service(
    node_config: NodeConfig,
    chain_id: ChainId,
    build_info: BTreeMap<String, String>,
    remote_log_rx: Option<mpsc::Receiver<TelemetryLog>>,
    logger_filter_update_job: Option<LoggerFilterUpdater>,
) {
    let telemetry_svc_url = env::var(ENV_TELEMETRY_SERVICE_URL).unwrap_or_else(|_| {
        if chain_id == ChainId::mainnet() {
            MAINNET_TELEMETRY_SERVICE_URL.into()
        } else {
            TELEMETRY_SERVICE_URL.into()
        }
    });

    let base_url = Url::parse(&telemetry_svc_url).unwrap_or_else(|err| {
        warn!(
            "Unable to parse telemetry service URL {}. Make sure {} is unset or is set properly: {}. Defaulting to {}.",
            telemetry_svc_url,
            ENV_TELEMETRY_SERVICE_URL, err, TELEMETRY_SERVICE_URL
        );
            Url::parse(TELEMETRY_SERVICE_URL)
                .expect("unable to parse telemetry service default URL")
    });

    let telemetry_sender = TelemetrySender::new(base_url, chain_id, &node_config);

    if !force_enable_telemetry() && !telemetry_sender.check_chain_access(chain_id).await {
        warn!(
                "Aptos telemetry is not sent to the telemetry service because the service is not configured for chain ID {}",
                chain_id
            );
        // Spawn the custom event sender to send to GA4 only.
        // This is a temporary workaround while we deprecate and remove GA4 completely.
        let peer_id = fetch_peer_id(&node_config);
        let handle = tokio::spawn(custom_event_sender(
            None,
            peer_id,
            chain_id,
            node_config.clone(),
            build_info.clone(),
        ));
        info!("Telemetry service for GA4 started!");

        // Check for chain access periodically in case the service is configured later
        let mut interval = time::interval(Duration::from_secs(CHAIN_ACCESS_CHECK_FREQ_SECS));
        loop {
            interval.tick().await;
            if telemetry_sender.check_chain_access(chain_id).await {
                handle.abort();
                info!("Aptos telemetry service is now configured for Chain ID {}. Starting telemetry service...", chain_id);
                break;
            }
        }
    }

    try_spawn_log_sender(telemetry_sender.clone(), remote_log_rx);
    try_spawn_metrics_sender(telemetry_sender.clone());
    try_spawn_custom_event_sender(node_config, telemetry_sender.clone(), chain_id, build_info);
    try_spawn_log_env_poll_task(telemetry_sender);

    // Run the logger filter update job within the telemetry runtime.
    if let Some(job) = logger_filter_update_job {
        tokio::spawn(job.run());
    }

    info!("Telemetry service started!");
}

fn try_spawn_log_env_poll_task(sender: TelemetrySender) {
    if enable_log_env_polling() {
        tokio::spawn(async move {
            let original_value = env::var(RUST_LOG_TELEMETRY).ok();
            let mut interval = time::interval(Duration::from_secs(LOG_ENV_POLL_FREQ_SECS));
            loop {
                interval.tick().await;
                if let Some(env) = sender.get_telemetry_log_env().await {
                    info!(
                        "Updating {} env variable: previous value: {:?}, new value: {}",
                        RUST_LOG_TELEMETRY,
                        env::var(RUST_LOG_TELEMETRY).ok(),
                        env
                    );
                    // TODO: Audit that the environment access only happens in single-threaded code.
                    unsafe { env::set_var(RUST_LOG_TELEMETRY, env) }
                } else if let Some(ref value) = original_value {
                    // TODO: Audit that the environment access only happens in single-threaded code.
                    unsafe { env::set_var(RUST_LOG_TELEMETRY, value) }
                } else {
                    // TODO: Audit that the environment access only happens in single-threaded code.
                    unsafe { env::remove_var(RUST_LOG_TELEMETRY) }
                }
            }
        });
    }
}

fn try_spawn_custom_event_sender(
    node_config: NodeConfig,
    telemetry_sender: TelemetrySender,
    chain_id: ChainId,
    build_info: BTreeMap<String, String>,
) {
    if enable_push_custom_events() {
        // Spawn the custom event sender
        let peer_id = fetch_peer_id(&node_config);
        tokio::spawn(custom_event_sender(
            Some(telemetry_sender),
            peer_id,
            chain_id,
            node_config,
            build_info,
        ));
    }
}

fn try_spawn_metrics_sender(telemetry_sender: TelemetrySender) {
    if enable_prometheus_push_metrics() {
        tokio::spawn(async move {
            // Periodically send ALL prometheus metrics (This replaces the previous core and network metrics implementation)
            let mut interval =
                time::interval(Duration::from_secs(PROMETHEUS_PUSH_METRICS_FREQ_SECS));
            loop {
                interval.tick().await;
                telemetry_sender.try_push_prometheus_metrics().await;
            }
        });
    }
}

fn try_spawn_log_sender(
    telemetry_sender: TelemetrySender,
    remote_log_rx: Option<Receiver<TelemetryLog>>,
) {
    if enable_push_logs() {
        if let Some(rx) = remote_log_rx {
            let telemetry_log_sender = TelemetryLogSender::new(telemetry_sender);
            tokio::spawn(telemetry_log_sender.start(rx));
        }
    }
}

/// Returns the peer id given the node config.
/// Returns UNKNOWN otherwise.
fn fetch_peer_id(node_config: &NodeConfig) -> String {
    match node_config.get_peer_id() {
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
async fn custom_event_sender(
    telemetry_sender: Option<TelemetrySender>,
    peer_id: String,
    chain_id: ChainId,
    node_config: NodeConfig,
    build_info: BTreeMap<String, String>,
) {
    futures::future::join5(
        // Periodically send build information
        run_function_periodically(NODE_BUILD_INFO_FREQ_SECS, || {
            send_build_information(
                peer_id.clone(),
                chain_id.to_string(),
                build_info.clone(),
                telemetry_sender.clone(),
            )
        }),
        // Periodically send system information
        run_function_periodically(NODE_SYS_INFO_FREQ_SECS, || {
            send_system_information(
                peer_id.clone(),
                chain_id.to_string(),
                telemetry_sender.clone(),
            )
        }),
        // Periodically send node core metrics
        run_function_periodically(NODE_CORE_METRICS_FREQ_SECS, || {
            send_node_core_metrics(
                peer_id.clone(),
                chain_id.to_string(),
                &node_config,
                telemetry_sender.clone(),
            )
        }),
        // Periodically send node network metrics
        run_function_periodically(NODE_NETWORK_METRICS_FREQ_SECS, || {
            send_node_network_metrics(
                peer_id.clone(),
                chain_id.to_string(),
                telemetry_sender.clone(),
            )
        }),
        run_function_periodically(NODE_CONFIG_FREQ_SECS, || {
            send_node_config(
                peer_id.clone(),
                chain_id.to_string(),
                &node_config,
                telemetry_sender.clone(),
            )
        }),
    )
    .await;
}

/// Collects and sends the build information via telemetry
async fn send_build_information(
    peer_id: String,
    chain_id: String,
    build_info: BTreeMap<String, String>,
    telemetry_sender: Option<TelemetrySender>,
) {
    let telemetry_event = create_build_info_telemetry_event(build_info).await;
    prepare_and_send_telemetry_event(peer_id, chain_id, telemetry_sender, telemetry_event).await;
}

/// Collects and sends the core node metrics via telemetry
async fn send_node_config(
    peer_id: String,
    chain_id: String,
    node_config: &NodeConfig,
    telemetry_sender: Option<TelemetrySender>,
) {
    let node_config: BTreeMap<String, String> = serde_json::to_value(node_config)
        .map(|value| {
            value
                .as_object()
                .map(|obj| {
                    obj.into_iter()
                        .map(|(k, v)| (k.clone(), v.to_string()))
                        .collect::<BTreeMap<String, String>>()
                })
                .unwrap_or_default()
        })
        .unwrap_or_default();

    let telemetry_event = TelemetryEvent {
        name: APTOS_NODE_CONFIG_EVENT_NAME.into(),
        params: node_config,
    };
    prepare_and_send_telemetry_event(peer_id, chain_id, telemetry_sender, telemetry_event).await;
}

/// Collects and sends the core node metrics via telemetry
async fn send_node_core_metrics(
    peer_id: String,
    chain_id: String,
    node_config: &NodeConfig,
    telemetry_sender: Option<TelemetrySender>,
) {
    let telemetry_event = create_core_metric_telemetry_event(node_config).await;
    prepare_and_send_telemetry_event(peer_id, chain_id, telemetry_sender, telemetry_event).await;
}

/// Collects and sends the node network metrics via telemetry
async fn send_node_network_metrics(
    peer_id: String,
    chain_id: String,
    telemetry_sender: Option<TelemetrySender>,
) {
    let telemetry_event = create_network_metric_telemetry_event().await;
    prepare_and_send_telemetry_event(peer_id, chain_id, telemetry_sender, telemetry_event).await;
}

/// Collects and sends the system information via telemetry
async fn send_system_information(
    peer_id: String,
    chain_id: String,
    telemetry_sender: Option<TelemetrySender>,
) {
    let telemetry_event = create_system_info_telemetry_event().await;
    prepare_and_send_telemetry_event(peer_id, chain_id, telemetry_sender, telemetry_event).await;
}

/// Fetches the IP address and sends the given telemetry event
/// along with the IP address. Also sends a randomly generated
/// token to help correlate metrics across events.
pub(crate) async fn prepare_and_send_telemetry_event(
    peer_id: String,
    chain_id: String,
    telemetry_sender: Option<TelemetrySender>,
    telemetry_event: TelemetryEvent,
) -> JoinHandle<()> {
    // Update the telemetry event with the ip address and random token
    let TelemetryEvent { name, mut params } = telemetry_event;
    params.insert(TELEMETRY_TOKEN_KEY.to_string(), TELEMETRY_TOKEN.clone());
    params.insert(CHAIN_ID_KEY.into(), chain_id);
    let telemetry_event = TelemetryEvent { name, params };

    // Send the telemetry event
    send_telemetry_event(peer_id, telemetry_sender, telemetry_event).await
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
    if telemetry_sender.is_none() {
        // telemetry_sender is None for Aptos CLI.
        spawn_event_sender_to_google_analytics(
            api_secret,
            measurement_id,
            event_name,
            telemetry_dump,
        )
    } else {
        // Aptos nodes send their metrics to aptos-telemetry-service crate.
        spawn_event_sender_to_telemetry_service(event_name, telemetry_sender, telemetry_dump)
    }
}

/// Spawns the telemetry event sender on a new thread to avoid blocking
fn spawn_event_sender_to_telemetry_service(
    event_name: String,
    telemetry_sender: Option<TelemetrySender>,
    telemetry_dump: TelemetryDump,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        telemetry_sender
            .unwrap()
            .try_send_custom_metrics(event_name, telemetry_dump)
            .await;
    })
}

/// Spawns the telemetry event sender on a new thread to avoid blocking
fn spawn_event_sender_to_google_analytics(
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
                    debug!(
                        "Failed to send telemetry event! Status: {}, event: {}.",
                        response.status(),
                        event_name
                    );
                    debug!("Failed telemetry response: {:?}", response.text().await);
                    metrics::increment_telemetry_failures(&event_name);
                }
            },
            Err(error) => {
                debug!(
                    "Failed to send telemetry event: {}. Error: {:?}",
                    event_name, error
                );
                metrics::increment_telemetry_failures(&event_name);
            },
        }
    })
}
