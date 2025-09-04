// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::testutils::{
    test_framework::TestFramework,
    test_node::{NodeId, NodeType, TestNode},
};
use velor_config::{
    config::NodeConfig,
    network_id::{NetworkId, PeerNetworkId},
};
use rand::{rngs::StdRng, SeedableRng};
use std::{collections::HashMap, marker::PhantomData};

/// A builder for a [`TestFramework`] implementation.
///
/// This handles making sure that nodes are unique, and that they have the channels attached
/// to each other to send messages between each other.
///
pub struct TestFrameworkBuilder<Framework: TestFramework<Node>, Node: TestNode> {
    /// Owners are a simple stand in for actual [`AccountAddress`] uniqueness in the network.
    /// An `owner` can have 1 of each [`NodeType`] of `Node`. This simplifies linking
    /// [`NodeType::Validator`] and [`NodeType::ValidatorFullNode`] later, as well as keeps
    /// a unique identifier for each node as [`NodeId`].
    owners: u32,
    /// The unique mapping of `Node` to ensure no duplicates.
    nodes: HashMap<NodeId, Node>,
    /// Random generator for randomness in keys and [`PeerId`]s. Hardcoded to remove non-determinism.
    rng: StdRng,
    _framework_marker: PhantomData<Framework>,
}

impl<Framework: TestFramework<Node>, Node: TestNode> TestFrameworkBuilder<Framework, Node> {
    /// Create a new [`TestFrameworkBuilder`], ensuring that there is a fixed number of `owners`.
    pub fn new(owners: u32) -> Self {
        Self {
            owners,
            nodes: HashMap::new(),
            rng: StdRng::from_seed([0u8; 32]),
            _framework_marker: PhantomData,
        }
    }

    /// Builds the [`TestFramework`]
    pub fn build(self) -> Framework {
        TestFramework::new(self.nodes)
    }

    /// Adds a [`TestNode`] of [`NodeType::Validator`]
    pub fn add_validator(mut self, owner: u32) -> Self {
        let config = NodeConfig::generate_random_config_with_template(
            &NodeConfig::get_default_validator_config(),
            &mut self.rng,
        );
        let peer_id = config
            .validator_network
            .as_ref()
            .expect("Validator must have a validator network")
            .peer_id();

        self.add_node(owner, NodeType::Validator, config, &[
            PeerNetworkId::new(NetworkId::Validator, peer_id),
            PeerNetworkId::new(NetworkId::Vfn, peer_id),
        ])
    }

    /// Adds a [`TestNode`] of [`NodeType::ValidatorFullNode`]
    pub fn add_vfn(mut self, owner: u32) -> Self {
        let config = NodeConfig::generate_random_config_with_template(
            &NodeConfig::get_default_vfn_config(),
            &mut self.rng,
        );
        let peer_id = config
            .full_node_networks
            .iter()
            .find(|network| network.network_id == NetworkId::Public)
            .expect("Vfn must have a public network")
            .peer_id();

        self.add_node(owner, NodeType::ValidatorFullNode, config, &[
            PeerNetworkId::new(NetworkId::Vfn, peer_id),
            PeerNetworkId::new(NetworkId::Public, peer_id),
        ])
    }

    /// Adds a [`TestNode`] of [`NodeType::PublicFullNode`]
    pub fn add_pfn(mut self, owner: u32) -> Self {
        let config = NodeConfig::generate_random_config_with_template(
            &NodeConfig::get_default_pfn_config(),
            &mut self.rng,
        );
        let peer_id = config
            .full_node_networks
            .iter()
            .find(|network| network.network_id == NetworkId::Public)
            .expect("Pfn must have a public network")
            .peer_id();

        self.add_node(owner, NodeType::PublicFullNode, config, &[
            PeerNetworkId::new(NetworkId::Public, peer_id),
        ])
    }

    /// Add a node to the network, ensuring that it doesn't already exist
    fn add_node(
        mut self,
        owner: u32,
        node_type: NodeType,
        config: NodeConfig,
        peer_network_ids: &[PeerNetworkId],
    ) -> Self {
        assert!(owner < self.owners);

        let node_id = NodeId { owner, node_type };
        assert!(!self.nodes.contains_key(&node_id));

        let mut node = Framework::build_node(node_id, config, peer_network_ids);

        // Add node's sender to every possible node that it could be connected to.
        for (other_node_id, other_node) in self.nodes.iter_mut() {
            if let Some(network_id) = other_node.find_common_network(&node) {
                // The VFN network only goes between the same owner, skip it if it doesn't match
                if network_id == NetworkId::Vfn && owner != other_node_id.owner {
                    continue;
                }

                // Add the inbound handle for the new node to the other node
                add_inbound_peer_handle_to_node(network_id, other_node, &node);

                // Add the inbound handle for the other node to the new node
                add_inbound_peer_handle_to_node(network_id, &mut node, other_node);
            }
        }

        self.nodes.insert(node_id, node);
        self
    }

    pub fn single_validator() -> Node {
        let mut test_framework: Framework = TestFrameworkBuilder::new(1).add_validator(0).build();
        test_framework.take_node(NodeId::validator(0))
    }

    pub fn single_vfn() -> Node {
        let mut test_framework: Framework = TestFrameworkBuilder::new(1).add_vfn(0).build();
        test_framework.take_node(NodeId::vfn(0))
    }

    pub fn single_pfn() -> Node {
        let mut test_framework: Framework = TestFrameworkBuilder::new(1).add_pfn(0).build();
        test_framework.take_node(NodeId::pfn(0))
    }
}

/// Adds `receiving_node`'s peer_handle to the `sending_node`
fn add_inbound_peer_handle_to_node<Node: TestNode>(
    network_id: NetworkId,
    sending_node: &mut Node,
    receiving_node: &Node,
) {
    sending_node.add_inbound_handle_for_peer(
        receiving_node.peer_network_id(network_id),
        receiving_node.get_inbound_handle(network_id),
    );
}
