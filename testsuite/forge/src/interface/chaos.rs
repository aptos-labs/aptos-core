// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_sdk::types::PeerId;
use std::fmt::{Display, Formatter};

#[derive(Eq, Hash, PartialEq, Debug, Clone)]
pub enum SwarmChaos {
    Delay(SwarmNetworkDelay),
    Partition(SwarmNetworkPartition),
    Bandwidth(SwarmNetworkBandwidth),
    Loss(SwarmNetworkLoss),
    NetEm(SwarmNetEm),
    CpuStress(SwarmCpuStress),
}

#[derive(Eq, Hash, PartialEq, Debug, Clone)]
pub struct SwarmNetworkDelay {
    pub group_network_delays: Vec<GroupNetworkDelay>,
}

impl Display for SwarmNetworkDelay {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "Delay nodes {:?}", self.group_network_delays)
    }
}

#[derive(Eq, Hash, PartialEq, Debug, Clone)]
pub struct GroupNetworkDelay {
    pub name: String,
    pub source_nodes: Vec<PeerId>,
    pub target_nodes: Vec<PeerId>,
    pub latency_ms: u64,
    pub jitter_ms: u64,
    pub correlation_percentage: u64,
}

#[derive(Eq, Hash, PartialEq, Debug, Clone)]
pub struct SwarmNetworkPartition {
    pub partition_percentage: u64,
}

impl Display for SwarmNetworkPartition {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "Partition {} nodes", self.partition_percentage)
    }
}

#[derive(Eq, Hash, PartialEq, Debug, Clone)]
pub struct SwarmNetworkBandwidth {
    pub group_network_bandwidths: Vec<GroupNetworkBandwidth>,
}

impl Display for SwarmNetworkBandwidth {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "Bandwidth nodes {:?}", self.group_network_bandwidths)
    }
}

#[derive(Eq, Hash, PartialEq, Debug, Clone)]
pub struct GroupNetworkBandwidth {
    pub name: String,
    /// Rate in megabytes per second
    pub rate: u64,
    pub limit: u64,
    pub buffer: u64,
}

#[derive(Eq, Hash, PartialEq, Debug, Clone)]
pub struct SwarmNetworkLoss {
    pub loss_percentage: u64,
    pub correlation_percentage: u64,
}

impl Display for SwarmNetworkLoss {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "Loss on all nodes: loss {}, correlation {},",
            self.loss_percentage, self.correlation_percentage,
        )
    }
}

#[derive(Eq, Hash, PartialEq, Debug, Clone)]
pub struct SwarmNetEm {
    pub group_netems: Vec<GroupNetEm>,
}

impl Display for SwarmNetEm {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "NetEm nodes {:?}", self.group_netems)
    }
}

#[derive(Eq, Hash, PartialEq, Debug, Clone)]
pub struct GroupNetEm {
    pub name: String,
    pub source_nodes: Vec<PeerId>,
    pub target_nodes: Vec<PeerId>,
    pub delay_latency_ms: u64,
    pub delay_jitter_ms: u64,
    pub delay_correlation_percentage: u64,
    pub loss_percentage: u64,
    pub loss_correlation_percentage: u64,
    pub rate_in_mbps: u64,
}

#[derive(Eq, Hash, PartialEq, Debug, Clone)]
pub struct SwarmCpuStress {
    pub group_cpu_stresses: Vec<GroupCpuStress>,
}

impl Display for SwarmCpuStress {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "CpuStress nodes {:?}", self.group_cpu_stresses)
    }
}

#[derive(Eq, Hash, PartialEq, Debug, Clone)]
pub struct GroupCpuStress {
    pub name: String,
    pub target_nodes: Vec<PeerId>,
    pub num_workers: u64,
    pub load_per_worker: u64,
}
