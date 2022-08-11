// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#[derive(Eq, Hash, PartialEq, Debug, Clone)]
pub enum SwarmChaos {
    Delay(SwarmNetworkDelay),
    Partition(SwarmNetworkPartition),
    Bandwidth(SwarmNetworkBandwidth),
    Loss(SwarmNetworkLoss),
}

#[derive(Eq, Hash, PartialEq, Debug, Clone)]
pub enum NodeChaos {
    NodeNetworkDelayChaos(NodeNetworkDelay),
}

#[derive(Eq, Hash, PartialEq, Debug, Clone)]
pub struct SwarmNetworkDelay {
    pub latency_ms: u64,
    pub jitter_ms: u64,
    pub correlation_percentage: u64,
}

#[derive(Eq, Hash, PartialEq, Debug, Clone)]
pub struct SwarmNetworkPartition {
    pub partition_percentage: u64,
}

#[derive(Eq, Hash, PartialEq, Debug, Clone)]
pub struct SwarmNetworkBandwidth {
    pub rate: u64,
    pub limit: u64,
    pub buffer: u64,
}

#[derive(Eq, Hash, PartialEq, Debug, Clone)]
pub struct SwarmNetworkLoss {
    pub loss_percentage: u64,
    pub correlation_percentage: u64,
}

#[derive(Eq, Hash, PartialEq, Debug, Clone)]
pub struct NodeNetworkDelay {
    pub latency_ms: u64,
}
