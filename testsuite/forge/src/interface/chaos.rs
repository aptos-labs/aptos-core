// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::fmt::{Display, Formatter};

use aptos_sdk::types::PeerId;

#[derive(Eq, Hash, PartialEq, Debug, Clone)]
pub enum SwarmChaos {
    Delay(SwarmNetworkDelay),
    Partition(SwarmNetworkPartition),
    Bandwidth(SwarmNetworkBandwidth),
    Loss(SwarmNetworkLoss),
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
    pub rate: u64,
    pub limit: u64,
    pub buffer: u64,
}

impl Display for SwarmNetworkBandwidth {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "Limit bandwidth on all nodes: rate {}, limit {}, buffer {}",
            self.rate, self.limit, self.buffer
        )
    }
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
