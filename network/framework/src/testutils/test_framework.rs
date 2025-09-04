// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::testutils::test_node::{ApplicationNode, NodeId};
use velor_config::{config::NodeConfig, network_id::PeerNetworkId};
use std::collections::HashMap;

// TODO: this code needs to either be used across applications, or just
// moved into the mempool crate.

/// A trait describing a test framework for a specific application
///
/// This is essentially an abstract implementation, to get around how rust handles traits
/// there are functions to get required variables in the implementation.
///
pub trait TestFramework<Node: ApplicationNode + Sync> {
    /// Constructor for the [`TestFramework`]
    fn new(nodes: HashMap<NodeId, Node>) -> Self;

    /// A constructor for `Node` specific to the application
    fn build_node(node_id: NodeId, config: NodeConfig, peer_network_ids: &[PeerNetworkId]) -> Node;

    /// In order to have separate tasks, we have to pull these out of the framework
    fn take_node(&mut self, node_id: NodeId) -> Node;
}
