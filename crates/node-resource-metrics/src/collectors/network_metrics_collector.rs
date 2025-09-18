// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::common::NAMESPACE;
use crate::collectors::common::MeasureLatency;
use aptos_infallible::Mutex;
use aptos_metrics_core::const_metric::ConstMetric;
use prometheus::{
    core::{Collector, Desc, Describer},
    proto::MetricFamily,
    Opts,
};
use std::sync::Arc;
use sysinfo::{NetworkExt, NetworksExt, RefreshKind, System, SystemExt};

const NETWORK_SUBSYSTEM: &str = "network";

const TOTAL_RECEIVED: &str = "total_received";
const TOTAL_TRANSMITTED: &str = "total_transmitted";
const TOTAL_PACKETS_RECEIVED: &str = "total_packets_received";
const TOTAL_PACKETS_TRANSMITTED: &str = "total_packets_transmitted";
const TOTAL_ERRORS_ON_RECEIVED: &str = "total_errors_on_received";
const TOTAL_ERRORS_ON_TRANSMITTED: &str = "total_errors_on_transmitted";

const INTERFACE_NAME_LABEL: &str = "interface_name";

/// A Collector for exposing network metrics
pub(crate) struct NetworkMetricsCollector {
    system: Arc<Mutex<System>>,

    total_received: Desc,
    total_transmitted: Desc,
    total_packets_received: Desc,
    total_packets_transmitted: Desc,
    total_errors_on_received: Desc,
    total_errors_on_transmitted: Desc,
}

impl NetworkMetricsCollector {
    fn new() -> Self {
        let system = Arc::new(Mutex::new(System::new_with_specifics(
            RefreshKind::new().with_networks_list().with_networks(),
        )));

        let total_received = Opts::new(TOTAL_RECEIVED, "Total number of received bytes")
            .namespace(NAMESPACE)
            .subsystem(NETWORK_SUBSYSTEM)
            .variable_label(INTERFACE_NAME_LABEL)
            .describe()
            .unwrap();
        let total_transmitted = Opts::new(TOTAL_TRANSMITTED, "Total number of transmitted bytes")
            .namespace(NAMESPACE)
            .subsystem(NETWORK_SUBSYSTEM)
            .variable_label(INTERFACE_NAME_LABEL)
            .describe()
            .unwrap();
        let total_packets_received =
            Opts::new(TOTAL_PACKETS_RECEIVED, "Total number of incoming packets")
                .namespace(NAMESPACE)
                .subsystem(NETWORK_SUBSYSTEM)
                .variable_label(INTERFACE_NAME_LABEL)
                .describe()
                .unwrap();
        let total_packets_transmitted = Opts::new(
            TOTAL_PACKETS_TRANSMITTED,
            "Total number of outgoing packets",
        )
        .namespace(NAMESPACE)
        .subsystem(NETWORK_SUBSYSTEM)
        .variable_label(INTERFACE_NAME_LABEL)
        .describe()
        .unwrap();
        let total_errors_on_received =
            Opts::new(TOTAL_ERRORS_ON_RECEIVED, "Total number of incoming errors")
                .namespace(NAMESPACE)
                .subsystem(NETWORK_SUBSYSTEM)
                .variable_label(INTERFACE_NAME_LABEL)
                .describe()
                .unwrap();
        let total_errors_on_transmitted = Opts::new(
            TOTAL_ERRORS_ON_TRANSMITTED,
            "Total number of transmission errors",
        )
        .namespace(NAMESPACE)
        .subsystem(NETWORK_SUBSYSTEM)
        .variable_label(INTERFACE_NAME_LABEL)
        .describe()
        .unwrap();

        Self {
            system,
            total_received,
            total_transmitted,
            total_packets_received,
            total_packets_transmitted,
            total_errors_on_received,
            total_errors_on_transmitted,
        }
    }
}

impl Default for NetworkMetricsCollector {
    fn default() -> Self {
        NetworkMetricsCollector::new()
    }
}

impl Collector for NetworkMetricsCollector {
    fn desc(&self) -> Vec<&Desc> {
        vec![
            &self.total_received,
            &self.total_transmitted,
            &self.total_packets_received,
            &self.total_packets_transmitted,
            &self.total_errors_on_received,
            &self.total_errors_on_transmitted,
        ]
    }

    fn collect(&self) -> Vec<MetricFamily> {
        let _measure = MeasureLatency::new("network".into());

        let mut system = self.system.lock();
        system.refresh_networks_list();
        system.refresh_networks();

        system
            .networks()
            .iter()
            .flat_map(|(interface_name, network)| {
                let total_received = ConstMetric::new_counter(
                    self.total_received.clone(),
                    network.total_received() as f64,
                    Some(&[interface_name.into()]),
                )
                .unwrap();
                let total_transmitted = ConstMetric::new_counter(
                    self.total_transmitted.clone(),
                    network.total_transmitted() as f64,
                    Some(&[interface_name.into()]),
                )
                .unwrap();
                let total_packets_received = ConstMetric::new_counter(
                    self.total_packets_received.clone(),
                    network.total_packets_received() as f64,
                    Some(&[interface_name.into()]),
                )
                .unwrap();
                let total_packets_transmitted = ConstMetric::new_counter(
                    self.total_packets_transmitted.clone(),
                    network.total_packets_transmitted() as f64,
                    Some(&[interface_name.into()]),
                )
                .unwrap();
                let total_errors_on_received = ConstMetric::new_counter(
                    self.total_errors_on_received.clone(),
                    network.total_errors_on_received() as f64,
                    Some(&[interface_name.into()]),
                )
                .unwrap();
                let total_errors_on_transmitted = ConstMetric::new_counter(
                    self.total_errors_on_transmitted.clone(),
                    network.total_errors_on_transmitted() as f64,
                    Some(&[interface_name.into()]),
                )
                .unwrap();

                vec![
                    total_received,
                    total_transmitted,
                    total_packets_received,
                    total_packets_transmitted,
                    total_errors_on_received,
                    total_errors_on_transmitted,
                ]
            })
            .flat_map(|metric| metric.collect())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::NetworkMetricsCollector;
    use prometheus::Registry;

    #[test]
    fn test_cpu_collector_register() {
        let collector = NetworkMetricsCollector::default();

        let r = Registry::new();
        let res = r.register(Box::new(collector));
        assert!(res.is_ok());
    }
}
