// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::utils;
use aptos_telemetry_service::types::telemetry::TelemetryEvent;
use prometheus::core::Collector;
use std::collections::BTreeMap;

/// Network metrics event name
const APTOS_NODE_NETWORK_METRICS: &str = "APTOS_NODE_NETWORK_METRICS";

/// Network metric keys
const NETWORK_INBOUND_CONNECTIONS: &str = "network_inbound_connections";
const NETWORK_INBOUND_MESSAGE_SUM: &str = "network_inbound_message_sum";
const NETWORK_INBOUND_TRAFFIC_SUM: &str = "network_inbound_traffic_sum";
const NETWORK_OUTBOUND_CONNECTIONS: &str = "network_outbound_connections";
const NETWORK_OUTBOUND_MESSAGE_SUM: &str = "network_outbound_message_sum";
const NETWORK_OUTBOUND_TRAFFIC_SUM: &str = "network_outbound_traffic_sum";

/// Collects and sends the build information via telemetry
pub(crate) async fn create_network_metric_telemetry_event() -> TelemetryEvent {
    // Collect the network metrics
    let network_metrics = get_network_metrics();

    // Create and return a new telemetry event
    TelemetryEvent {
        name: APTOS_NODE_NETWORK_METRICS.into(),
        params: network_metrics,
    }
}

/// Used to expose network metrics for the node
pub fn get_network_metrics() -> BTreeMap<String, String> {
    let mut network_metrics: BTreeMap<String, String> = BTreeMap::new();
    collect_network_metrics(&mut network_metrics);
    network_metrics
}

/// Collects the network metrics and appends them to the given map
fn collect_network_metrics(network_metrics: &mut BTreeMap<String, String>) {
    collect_connection_metrics(network_metrics);
    collect_message_and_traffic_metrics(network_metrics);
}

/// Collects the connection metrics and appends them to the given map
fn collect_connection_metrics(network_metrics: &mut BTreeMap<String, String>) {
    // Calculate the number of inbound and outbound connections
    let mut inbound_connection_count: f64 = 0.0;
    let mut outbound_connection_count: f64 = 0.0;
    for metric_family in network::counters::APTOS_CONNECTIONS.collect() {
        for metric in metric_family.get_metric() {
            // TODO(joshlind): avoid matching on strings that can change!
            for label in metric.get_label() {
                if label.get_name() == "direction" {
                    if label.get_value() == "inbound" {
                        inbound_connection_count += metric.get_gauge().get_value();
                    } else if label.get_value() == "outbound" {
                        outbound_connection_count += metric.get_gauge().get_value();
                    }
                }
            }
        }
    }

    // Update the connection metrics
    network_metrics.insert(
        NETWORK_INBOUND_CONNECTIONS.into(),
        inbound_connection_count.to_string(),
    );
    network_metrics.insert(
        NETWORK_OUTBOUND_CONNECTIONS.into(),
        outbound_connection_count.to_string(),
    );
}

/// Collects the message and traffic metrics and appends them to the given map
fn collect_message_and_traffic_metrics(network_metrics: &mut BTreeMap<String, String>) {
    // Calculate the inbound messages and traffic
    let inbound_metric_families = network::counters::NETWORK_APPLICATION_INBOUND_METRIC.collect();
    let network_inbound_message_sum = utils::sum_all_histogram_counts(&inbound_metric_families);
    let network_inbound_traffic_sum = utils::sum_all_histogram_sums(&inbound_metric_families);

    // Calculate the outbound messages and traffic
    let outbound_metric_families = network::counters::NETWORK_APPLICATION_OUTBOUND_METRIC.collect();
    let network_outbound_message_sum = utils::sum_all_histogram_counts(&outbound_metric_families);
    let network_outbound_traffic_sum = utils::sum_all_histogram_sums(&outbound_metric_families);

    // Update the metrics
    network_metrics.insert(
        NETWORK_INBOUND_MESSAGE_SUM.into(),
        network_inbound_message_sum.to_string(),
    );
    network_metrics.insert(
        NETWORK_INBOUND_TRAFFIC_SUM.into(),
        network_inbound_traffic_sum.to_string(),
    );
    network_metrics.insert(
        NETWORK_OUTBOUND_MESSAGE_SUM.into(),
        network_outbound_message_sum.to_string(),
    );
    network_metrics.insert(
        NETWORK_OUTBOUND_TRAFFIC_SUM.into(),
        network_outbound_traffic_sum.to_string(),
    );
}
