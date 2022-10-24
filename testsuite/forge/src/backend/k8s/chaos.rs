// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::process::{Command, Stdio};

use anyhow::bail;
use aptos_logger::info;
use tempfile::TempDir;

use crate::{
    dump_string_to_file, K8sSwarm, Result, Swarm, SwarmChaos, SwarmNetworkBandwidth,
    SwarmNetworkDelay, SwarmNetworkLoss, SwarmNetworkPartition, KUBECTL_BIN,
};

macro_rules! DELAY_NETWORK_CHAOS_TEMPLATE {
    () => {
        "chaos/network_delay.yaml"
    };
}
macro_rules! PARTITION_NETWORK_CHAOS_TEMPLATE {
    () => {
        "chaos/network_partition.yaml"
    };
}
macro_rules! BANDWIDTH_NETWORK_CHAOS_TEMPLATE {
    () => {
        "chaos/network_bandwidth.yaml"
    };
}

macro_rules! NETWORK_LOSS_CHAOS_TEMPLATE {
    () => {
        "chaos/network_loss.yaml"
    };
}

impl K8sSwarm {
    /// Injects the SwarmChaos into the specified namespace
    pub fn inject_swarm_chaos(&self, chaos: &SwarmChaos) -> Result<()> {
        let template = self.create_chaos_template(chaos)?;
        info!("Injecting chaos: {}", template);
        self.inject_chaos_template(template)
    }

    /// Removes the SwarmChaos from the specified namespace, if it exists
    /// Most types of Chaos are represented by a single NetworkChaos CRD, so we can just reconstruct
    /// it and kubectl delete -f it. However, Delay Chaos is represnted by however many pairwise delays there
    /// are (GroupNetworkDelay ie region), so we need to delete each one individually.
    pub fn remove_swarm_chaos(&self, chaos: &SwarmChaos) -> Result<()> {
        match chaos {
            SwarmChaos::Delay(network_delay) => {
                for group in &network_delay.group_network_delays {
                    let delete_networkchaos = [
                        "-n",
                        &self.kube_namespace,
                        "delete",
                        "networkchaos",
                        &group.name,
                    ];
                    info!("{:?}", delete_networkchaos);
                    let delete_networkchaos_output = Command::new(KUBECTL_BIN)
                        .stdout(Stdio::inherit())
                        .args(&delete_networkchaos)
                        .output()
                        .expect("failed to delete all NetworkChaos");
                    if !delete_networkchaos_output.status.success() {
                        bail!(
                            "{}",
                            String::from_utf8(delete_networkchaos_output.stderr).unwrap()
                        );
                    }
                }
                Ok(())
            }
            _ => {
                let template = self.create_chaos_template(chaos)?;
                self.remove_chaos_template(template)
            }
        }
    }

    fn create_network_delay_template(
        &self,
        swarm_network_delay: &SwarmNetworkDelay,
    ) -> Result<String> {
        let mut network_chaos_specs = vec![];

        for group_network_delay in &swarm_network_delay.group_network_delays {
            let source_instance_labels = group_network_delay
                .source_nodes
                .iter()
                .map(|node| {
                    if let Some(v) = self.validator(*node) {
                        v.name()
                    } else {
                        "invalid-node"
                    }
                })
                .collect::<Vec<_>>()
                .join(",");

            let target_instance_labels = group_network_delay
                .target_nodes
                .iter()
                .map(|node| {
                    if let Some(v) = self.validator(*node) {
                        v.name()
                    } else {
                        "invalid-node"
                    }
                })
                .collect::<Vec<_>>()
                .join(",");

            network_chaos_specs.push(format!(
                include_str!(DELAY_NETWORK_CHAOS_TEMPLATE!()),
                name = &group_network_delay.name,
                namespace = self.kube_namespace,
                latency_ms = group_network_delay.latency_ms,
                jitter_ms = group_network_delay.jitter_ms,
                correlation_percentage = group_network_delay.correlation_percentage,
                instance_labels = &source_instance_labels,
                target_instance_labels = &target_instance_labels,
            ));
        }
        Ok(network_chaos_specs.join("\n---\n"))
    }

    fn create_network_partition_template(
        &self,
        swarm_network_partition: &SwarmNetworkPartition,
    ) -> Result<String> {
        Ok(format!(
            include_str!(PARTITION_NETWORK_CHAOS_TEMPLATE!()),
            namespace = self.kube_namespace,
            partition_percentage = swarm_network_partition.partition_percentage
        ))
    }

    fn create_network_bandwidth_template(
        &self,
        swarm_network_bandwidth: &SwarmNetworkBandwidth,
    ) -> Result<String> {
        Ok(format!(
            include_str!(BANDWIDTH_NETWORK_CHAOS_TEMPLATE!()),
            namespace = self.kube_namespace,
            rate = swarm_network_bandwidth.rate,
            limit = swarm_network_bandwidth.limit,
            buffer = swarm_network_bandwidth.buffer
        ))
    }

    fn create_network_loss_template(
        &self,
        swarm_network_loss: &SwarmNetworkLoss,
    ) -> Result<String> {
        Ok(format!(
            include_str!(NETWORK_LOSS_CHAOS_TEMPLATE!()),
            namespace = self.kube_namespace,
            loss_percentage = swarm_network_loss.loss_percentage,
            correlation_percentage = swarm_network_loss.correlation_percentage,
        ))
    }

    fn create_chaos_template(&self, chaos: &SwarmChaos) -> Result<String> {
        match chaos {
            SwarmChaos::Delay(c) => self.create_network_delay_template(c),
            SwarmChaos::Partition(c) => self.create_network_partition_template(c),
            SwarmChaos::Bandwidth(c) => self.create_network_bandwidth_template(c),
            SwarmChaos::Loss(c) => self.create_network_loss_template(c),
        }
    }

    /// Creates and applies the NetworkChaos CRD
    fn inject_chaos_template(&self, chaos_template: String) -> Result<()> {
        let tmp_dir = TempDir::new().expect("Could not create temp dir");
        let latency_network_chaos_file_path = dump_string_to_file(
            format!("{}-chaos.yaml", self.kube_namespace),
            chaos_template,
            &tmp_dir,
        )?;
        Command::new(KUBECTL_BIN)
            .args([
                "-n",
                &self.kube_namespace,
                "apply",
                "-f",
                &latency_network_chaos_file_path,
            ])
            .status()
            .expect("Failed to submit chaos template");

        Ok(())
    }

    /// Removes the NetworkChaos CRD
    fn remove_chaos_template(&self, chaos_template: String) -> Result<()> {
        let tmp_dir = TempDir::new().expect("Could not create temp dir");
        let latency_network_chaos_file_path = dump_string_to_file(
            format!("{}-chaos.yaml", self.kube_namespace),
            chaos_template,
            &tmp_dir,
        )?;
        Command::new(KUBECTL_BIN)
            .args([
                "-n",
                &self.kube_namespace,
                "delete",
                "-f",
                &latency_network_chaos_file_path,
            ])
            .status()
            .expect("Failed to delete chaos by template");

        Ok(())
    }
}
