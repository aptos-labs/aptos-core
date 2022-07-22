// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::process::Command;

use tempfile::TempDir;

use crate::{
    dump_string_to_file, Result, SwarmChaos, SwarmNetworkBandwidth, SwarmNetworkDelay,
    SwarmNetworkPartition, KUBECTL_BIN,
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

/// Injects the SwarmChaos into the specified namespace
pub fn inject_swarm_chaos(kube_namespace: &str, chaos: &SwarmChaos) -> Result<()> {
    let template = create_chaos_template(kube_namespace, chaos)?;
    inject_chaos_template(kube_namespace, template)
}

/// Removes the SwarmChaos from the specified namespace, if it exists
pub fn remove_swarm_chaos(kube_namespace: &str, chaos: &SwarmChaos) -> Result<()> {
    let template = create_chaos_template(kube_namespace, chaos)?;
    remove_chaos_template(kube_namespace, template)
}

fn create_network_delay_template(
    kube_namespace: &str,
    swarm_network_delay: &SwarmNetworkDelay,
) -> String {
    format!(
        include_str!(DELAY_NETWORK_CHAOS_TEMPLATE!()),
        namespace = kube_namespace,
        latency_ms = swarm_network_delay.latency_ms,
        jitter_ms = swarm_network_delay.jitter_ms,
        correlation_percentage = swarm_network_delay.correlation_percentage,
    )
}

fn create_network_partition_template(
    kube_namespace: &str,
    swarm_network_partition: &SwarmNetworkPartition,
) -> String {
    format!(
        include_str!(PARTITION_NETWORK_CHAOS_TEMPLATE!()),
        namespace = kube_namespace,
        partition_percentage = swarm_network_partition.partition_percentage
    )
}

fn create_network_bandwidth_template(
    kube_namespace: &str,
    swarm_network_bandwidth: &SwarmNetworkBandwidth,
) -> String {
    format!(
        include_str!(BANDWIDTH_NETWORK_CHAOS_TEMPLATE!()),
        namespace = kube_namespace,
        rate = swarm_network_bandwidth.rate,
        limit = swarm_network_bandwidth.limit,
        buffer = swarm_network_bandwidth.buffer
    )
}

fn create_chaos_template(kube_namespace: &str, chaos: &SwarmChaos) -> Result<String> {
    let template = match chaos {
        SwarmChaos::Delay(c) => create_network_delay_template(kube_namespace, c),
        SwarmChaos::Partition(c) => create_network_partition_template(kube_namespace, c),
        SwarmChaos::Bandwidth(c) => create_network_bandwidth_template(kube_namespace, c),
    };
    Ok(template)
}

/// Creates and applies the NetworkChaos CRD
fn inject_chaos_template(kube_namespace: &str, chaos_template: String) -> Result<()> {
    let tmp_dir = TempDir::new().expect("Could not create temp dir");
    let latency_network_chaos_file_path = dump_string_to_file(
        format!("{}-chaos.yaml", kube_namespace),
        chaos_template,
        &tmp_dir,
    )?;
    Command::new(KUBECTL_BIN)
        .args([
            "-n",
            kube_namespace,
            "apply",
            "-f",
            &latency_network_chaos_file_path,
        ])
        .status()
        .expect("Failed to submit chaos template");

    Ok(())
}

/// Removes the NetworkChaos CRD
fn remove_chaos_template(kube_namespace: &str, chaos_template: String) -> Result<()> {
    let tmp_dir = TempDir::new().expect("Could not create temp dir");
    let latency_network_chaos_file_path = dump_string_to_file(
        format!("{}-chaos.yaml", kube_namespace),
        chaos_template,
        &tmp_dir,
    )?;
    Command::new(KUBECTL_BIN)
        .args([
            "-n",
            kube_namespace,
            "delete",
            "-f",
            &latency_network_chaos_file_path,
        ])
        .status()
        .expect("Failed to delete chaos by template");

    Ok(())
}
