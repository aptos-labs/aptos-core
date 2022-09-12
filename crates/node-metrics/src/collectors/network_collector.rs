// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use aptos_infallible::Mutex;
use aptos_metrics_core::const_metric::ConstMetric;
use prometheus::{
    core::{Collector, Desc, Describer},
    proto::MetricFamily,
    Opts,
};
use sysinfo::{NetworkExt, NetworksExt, RefreshKind, System, SystemExt};

use super::common::NAMESPACE;

const NETWORK_SUBSYSTEM: &str = "network";

/// A Collector for exposing network metrics
pub(crate) struct NetworkCollector {
    system: Arc<Mutex<System>>,

    total_received: Desc,
    total_transmitted: Desc,
    total_packets_received: Desc,
    total_packets_transmitted: Desc,
    total_errors_on_received: Desc,
    total_errors_on_transmitted: Desc,
}

impl NetworkCollector {
    fn new() -> Self {
        let system = Arc::new(Mutex::new(System::new_with_specifics(
            RefreshKind::new().with_networks_list().with_networks(),
        )));

        let total_received = Opts::new("total_received", "Total number of received bytes")
            .namespace(NAMESPACE)
            .subsystem(NETWORK_SUBSYSTEM)
            .variable_label("interface_name")
            .describe()
            .unwrap();
        let total_transmitted = Opts::new("total_transmitted", "Total number of transmitted bytes")
            .namespace(NAMESPACE)
            .subsystem(NETWORK_SUBSYSTEM)
            .variable_label("interface_name")
            .describe()
            .unwrap();
        let total_packets_received =
            Opts::new("total_packets_received", "Total number of incoming packets")
                .namespace(NAMESPACE)
                .subsystem(NETWORK_SUBSYSTEM)
                .variable_label("interface_name")
                .describe()
                .unwrap();
        let total_packets_transmitted = Opts::new(
            "total_packets_transmitted",
            "Total number of outgoing packets",
        )
        .namespace(NAMESPACE)
        .subsystem(NETWORK_SUBSYSTEM)
        .variable_label("interface_name")
        .describe()
        .unwrap();
        let total_errors_on_received = Opts::new(
            "total_errors_on_received",
            "Total number of incoming errors",
        )
        .namespace(NAMESPACE)
        .subsystem(NETWORK_SUBSYSTEM)
        .variable_label("interface_name")
        .describe()
        .unwrap();
        let total_errors_on_transmitted = Opts::new(
            "total_errors_on_transmitted",
            "Total number of transmission errors",
        )
        .namespace(NAMESPACE)
        .subsystem(NETWORK_SUBSYSTEM)
        .variable_label("interface_name")
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

impl Default for NetworkCollector {
    fn default() -> Self {
        NetworkCollector::new()
    }
}

impl Collector for NetworkCollector {
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
        let mut system = self.system.lock();
        system.refresh_networks_list();
        system.refresh_networks();

        let mfs = system
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
            .collect();

        mfs
    }
}

#[cfg(test)]
mod tests {
    use super::NetworkCollector;
    use prometheus::Registry;

    #[test]
    fn test_cpu_collector_register() {
        let collector = NetworkCollector::default();

        let r = Registry::new();
        let res = r.register(Box::new(collector));
        assert!(res.is_ok());
    }
}
