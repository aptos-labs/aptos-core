// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    dump_string_to_file, K8sSwarm, Result, Swarm, SwarmChaos, SwarmCpuStress, SwarmNetEm,
    SwarmNetworkBandwidth, SwarmNetworkDelay, SwarmNetworkLoss, SwarmNetworkPartition, KUBECTL_BIN,
};
use anyhow::bail;
use aptos_sdk::{move_types::account_address::AccountAddress, types::PeerId};
use log::info;
use std::process::{Command, Stdio};
use tempfile::TempDir;

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

macro_rules! NETEM_CHAOS_TEMPLATE {
    () => {
        "chaos/netem.yaml"
    };
}

macro_rules! CPU_STRESS_CHAOS_TEMPLATE {
    () => {
        "chaos/cpu_stress.yaml"
    };
}

// The node name for an address that could not be found in the swarm
const INVALID_NODE_STRING: &str = "invalid-node";

impl K8sSwarm {
    /// Injects the SwarmChaos into the specified namespace
    pub fn inject_swarm_chaos(&self, chaos: &SwarmChaos) -> Result<()> {
        let template = self.create_chaos_template(chaos)?;
        info!("Injecting chaos: {}", template);
        self.inject_chaos_template(template)
    }

    /// Removes the SwarmChaos from the specified namespace, if it exists
    /// Most types of Chaos are represented by a single NetworkChaos CRD, so we can just reconstruct
    /// it and kubectl delete -f it. However, Delay Chaos is represented by however many pairwise delays there
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
                        .args(delete_networkchaos)
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
            },
            _ => {
                let template = self.create_chaos_template(chaos)?;
                self.remove_chaos_template(template)
            },
        }
    }

    fn create_network_delay_template(
        &self,
        swarm_network_delay: &SwarmNetworkDelay,
    ) -> Result<String> {
        let mut network_chaos_specs = vec![];

        for group_network_delay in &swarm_network_delay.group_network_delays {
            let source_instance_labels =
                self.get_instance_labels(&group_network_delay.source_nodes);
            let target_instance_labels =
                self.get_instance_labels(&group_network_delay.target_nodes);

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
        let mut network_chaos_specs = vec![];

        for group_network_bandwidth in &swarm_network_bandwidth.group_network_bandwidths {
            network_chaos_specs.push(format!(
                include_str!(BANDWIDTH_NETWORK_CHAOS_TEMPLATE!()),
                namespace = self.kube_namespace,
                rate = group_network_bandwidth.rate,
                limit = group_network_bandwidth.limit,
                buffer = group_network_bandwidth.buffer,
            ));
        }

        Ok(network_chaos_specs.join("\n---\n"))
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

    fn create_netem_template(&self, swarm_netem: &SwarmNetEm) -> Result<String> {
        let mut network_chaos_specs = vec![];

        for group_netem in &swarm_netem.group_netems {
            let source_instance_labels = self.get_instance_labels(&group_netem.source_nodes);
            let target_instance_labels = self.get_instance_labels(&group_netem.target_nodes);
            let service_targets = self.get_service_targets(&group_netem.target_nodes);

            network_chaos_specs.push(format!(
                include_str!(NETEM_CHAOS_TEMPLATE!()),
                name = &group_netem.name,
                namespace = self.kube_namespace,
                delay_latency_ms = group_netem.delay_latency_ms,
                delay_jitter_ms = group_netem.delay_jitter_ms,
                delay_correlation_percentage = group_netem.delay_correlation_percentage,
                loss_percentage = group_netem.loss_percentage,
                loss_correlation_percentage = group_netem.loss_correlation_percentage,
                instance_labels = &source_instance_labels,
                target_instance_labels = &target_instance_labels,
                rate = group_netem.rate_in_mbps,
                service_targets = &service_targets,
            ));
        }

        Ok(network_chaos_specs.join("\n---\n"))
    }

    /// Creates the CPU stress template, which can be used to inject CPU stress into a pod.
    /// This can be used to simulate nodes with different available CPU resource even though the
    /// nodes have identical hardware. For example, a node with 4 cores can be simulated as a node
    /// with 2 cores by setting num_workers to 2.
    fn create_cpu_stress_template(&self, swarm_cpu_stress: &SwarmCpuStress) -> Result<String> {
        let mut cpu_stress_specs = vec![];

        for group_cpu_stress in &swarm_cpu_stress.group_cpu_stresses {
            let instance_labels = self.get_instance_labels(&group_cpu_stress.target_nodes);

            cpu_stress_specs.push(format!(
                include_str!(CPU_STRESS_CHAOS_TEMPLATE!()),
                name = &group_cpu_stress.name,
                namespace = self.kube_namespace,
                num_workers = group_cpu_stress.num_workers,
                load_per_worker = group_cpu_stress.load_per_worker,
                instance_labels = &instance_labels,
            ));
        }

        Ok(cpu_stress_specs.join("\n---\n"))
    }

    fn create_chaos_template(&self, chaos: &SwarmChaos) -> Result<String> {
        match chaos {
            SwarmChaos::Delay(c) => self.create_network_delay_template(c),
            SwarmChaos::Partition(c) => self.create_network_partition_template(c),
            SwarmChaos::Bandwidth(c) => self.create_network_bandwidth_template(c),
            SwarmChaos::Loss(c) => self.create_network_loss_template(c),
            SwarmChaos::NetEm(c) => self.create_netem_template(c),
            SwarmChaos::CpuStress(c) => self.create_cpu_stress_template(c),
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

    /// Returns the instance labels for the given peers
    /// as a string (separated by commas).
    fn get_instance_labels(&self, peers: &[PeerId]) -> String {
        peers
            .iter()
            .map(|node| self.get_node_name(node))
            .collect::<Vec<_>>()
            .join(",")
    }

    /// Returns the name of the node associated with the given account address
    fn get_node_name(&self, node: &AccountAddress) -> &str {
        if let Some(validator) = self.validator(*node) {
            validator.name()
        } else if let Some(fullnode) = self.full_node(*node) {
            fullnode.name()
        } else {
            // TODO: should we throw an error here instead of failing silently?
            INVALID_NODE_STRING
        }
    }

    fn get_service_name(&self, node: &AccountAddress) -> Option<String> {
        if let Some(validator) = self.validator(*node) {
            validator.service_name()
        } else if let Some(fullnode) = self.full_node(*node) {
            fullnode.service_name()
        } else {
            // TODO: should we throw an error here instead of failing silently?
            None
        }
    }

    pub(crate) fn get_service_targets(&self, target_nodes: &[AccountAddress]) -> String {
        target_nodes
            .iter()
            .filter_map(|node| self.get_service_name(node))
            .collect::<Vec<_>>()
            .join(",")
    }
}
