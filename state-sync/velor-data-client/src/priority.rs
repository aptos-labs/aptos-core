// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::utils;
use velor_config::{
    config::BaseConfig,
    network_id::{NetworkId, PeerNetworkId},
};
use velor_network::application::storage::PeersAndMetadata;
use itertools::Itertools;
use std::sync::Arc;

/// A simple enum containing the different categories for peer prioritization.
///
/// Note: If another priority is added to this enum, it should also be added
/// to `get_all_ordered_priorities`, below.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PeerPriority {
    HighPriority,   // Peers to highly prioritize when requesting data
    MediumPriority, // Peers to prioritize iff high priority peers are unavailable
    LowPriority, // Peers to use iff no other peers are available (these are generally unreliable)
}

impl PeerPriority {
    /// Returns a list of all peer priorities, ordered from
    /// highest to lowest priority.
    pub fn get_all_ordered_priorities() -> Vec<PeerPriority> {
        vec![
            PeerPriority::HighPriority,
            PeerPriority::MediumPriority,
            PeerPriority::LowPriority,
        ]
    }

    /// Returns the label for the peer priority
    pub fn get_label(&self) -> String {
        let label = match self {
            Self::HighPriority => "high_priority",
            Self::MediumPriority => "medium_priority",
            Self::LowPriority => "low_priority",
        };
        label.into()
    }

    /// Returns true iff the priority is high priority
    pub fn is_high_priority(&self) -> bool {
        matches!(self, Self::HighPriority)
    }
}

/// Returns the priority for the specified peer, according
/// to the node's config and the peer metadata.
pub fn get_peer_priority(
    base_config: Arc<BaseConfig>,
    peers_and_metadata: Arc<PeersAndMetadata>,
    peer: &PeerNetworkId,
) -> PeerPriority {
    // Handle the case that this node is a validator
    let peer_network_id = peer.network_id();
    if base_config.role.is_validator() {
        // Validators should highly prioritize other validators
        if peer_network_id.is_validator_network() {
            return PeerPriority::HighPriority;
        }

        // VFNs should be prioritized over PFNs. Note: having PFNs
        // connected to a validator is a rare (but possible) scenario.
        return if peer_network_id.is_vfn_network() {
            PeerPriority::MediumPriority
        } else {
            PeerPriority::LowPriority
        };
    }

    // Handle the case that this node is a VFN
    if peers_and_metadata
        .get_registered_networks()
        .contains(&NetworkId::Vfn)
    {
        // VFNs should highly prioritize validators
        if peer_network_id.is_vfn_network() {
            return PeerPriority::HighPriority;
        }

        // Trusted peers should be prioritized over untrusted peers.
        // This prioritizes other VFNs/seed peers over regular PFNs.
        if is_trusted_peer(peers_and_metadata.clone(), peer) {
            return PeerPriority::MediumPriority;
        }

        // Outbound connections should be prioritized over inbound connections.
        // This prioritizes other VFNs/seed peers over regular PFNs.
        return if let Some(metadata) = utils::get_metadata_for_peer(&peers_and_metadata, *peer) {
            if metadata.get_connection_metadata().is_outbound_connection() {
                PeerPriority::MediumPriority
            } else {
                PeerPriority::LowPriority
            }
        } else {
            PeerPriority::LowPriority // We don't have connection metadata
        };
    }

    // Otherwise, this node is a PFN. PFNs should highly
    // prioritize trusted peers (i.e., VFNs and seed peers).
    if is_trusted_peer(peers_and_metadata.clone(), peer) {
        return PeerPriority::HighPriority;
    }

    // Outbound connections should be prioritized. This prioritizes
    // other VFNs/seed peers over regular PFNs. Inbound connections
    // are always low priority (as they are generally unreliable).
    if let Some(metadata) = utils::get_metadata_for_peer(&peers_and_metadata, *peer) {
        if metadata.get_connection_metadata().is_outbound_connection() {
            PeerPriority::HighPriority
        } else {
            PeerPriority::LowPriority
        }
    } else {
        PeerPriority::LowPriority // We don't have connection metadata
    }
}

/// Returns true iff the given peer is a trusted peer
fn is_trusted_peer(peers_and_metadata: Arc<PeersAndMetadata>, peer: &PeerNetworkId) -> bool {
    peers_and_metadata
        .get_trusted_peer_state(peer)
        .is_ok_and(|peer_state| peer_state.is_some())
}

/// Returns true iff the specified peer is a high priority peer
pub fn is_high_priority_peer(
    base_config: Arc<BaseConfig>,
    peers_and_metadata: Arc<PeersAndMetadata>,
    peer: &PeerNetworkId,
) -> bool {
    let peer_priority = get_peer_priority(base_config, peers_and_metadata, peer);
    peer_priority.is_high_priority()
}

#[cfg(test)]
mod tests {
    use crate::priority::{get_peer_priority, is_high_priority_peer, PeerPriority};
    use velor_config::{
        config::{BaseConfig, Peer, PeerRole, RoleType},
        network_id::{NetworkId, PeerNetworkId},
    };
    use velor_netcore::transport::ConnectionOrigin;
    use velor_network::{application::storage::PeersAndMetadata, transport::ConnectionMetadata};
    use velor_types::PeerId;
    use maplit::hashmap;
    use std::{assert_eq, sync::Arc};

    #[test]
    fn test_is_high_priority_peer_validator() {
        // Create a base config for a validator
        let base_config = Arc::new(BaseConfig {
            role: RoleType::Validator,
            ..Default::default()
        });

        // Create a peers and metadata struct with all networks registered
        let peers_and_metadata =
            PeersAndMetadata::new(&[NetworkId::Validator, NetworkId::Vfn, NetworkId::Public]);

        // Create a VFN peer and verify it is not high priority
        let vfn_peer = PeerNetworkId::new(NetworkId::Vfn, PeerId::random());
        assert!(!is_high_priority_peer(
            base_config.clone(),
            peers_and_metadata.clone(),
            &vfn_peer
        ));

        // Create a PFN peer and verify it is not high priority
        let pfn_peer = PeerNetworkId::new(NetworkId::Public, PeerId::random());
        assert!(!is_high_priority_peer(
            base_config.clone(),
            peers_and_metadata.clone(),
            &pfn_peer
        ));

        // Create a validator peer and verify it is high priority
        let validator_peer = PeerNetworkId::new(NetworkId::Validator, PeerId::random());
        assert!(is_high_priority_peer(
            base_config.clone(),
            peers_and_metadata.clone(),
            &validator_peer
        ));
    }

    #[test]
    fn test_is_priority_peer_vfn() {
        // Create a base config for a VFN
        let base_config = Arc::new(BaseConfig {
            role: RoleType::FullNode,
            ..Default::default()
        });

        // Create a peers and metadata struct with VFN and public networks registered
        let peers_and_metadata = PeersAndMetadata::new(&[NetworkId::Vfn, NetworkId::Public]);

        // Create a validator peer and verify it is high priority
        let validator_peer = PeerNetworkId::new(NetworkId::Vfn, PeerId::random());
        assert!(is_high_priority_peer(
            base_config.clone(),
            peers_and_metadata.clone(),
            &validator_peer
        ));

        // Create a VFN peer (with an outbound connection) and verify it is not high priority
        let vfn_peer = PeerNetworkId::new(NetworkId::Public, PeerId::random());
        create_connection_metadata(&peers_and_metadata, vfn_peer, ConnectionOrigin::Outbound);
        assert!(!is_high_priority_peer(
            base_config.clone(),
            peers_and_metadata.clone(),
            &vfn_peer
        ));

        // Create a trusted VFN peer (with an inbound connection) and verify it is not high priority
        let vfn_peer = PeerNetworkId::new(NetworkId::Public, PeerId::random());
        create_connection_metadata(&peers_and_metadata, vfn_peer, ConnectionOrigin::Inbound);
        add_to_trusted_peers(&peers_and_metadata, vfn_peer);
        assert!(!is_high_priority_peer(
            base_config.clone(),
            peers_and_metadata.clone(),
            &vfn_peer
        ));

        // Create a PFN peer (with an outbound connection) and verify it is not high priority
        let pfn_peer = PeerNetworkId::new(NetworkId::Public, PeerId::random());
        create_connection_metadata(&peers_and_metadata, pfn_peer, ConnectionOrigin::Outbound);
        assert!(!is_high_priority_peer(
            base_config.clone(),
            peers_and_metadata.clone(),
            &vfn_peer
        ));

        // Create a trusted PFN peer (with an inbound connection) and verify it is not high priority
        let pfn_peer = PeerNetworkId::new(NetworkId::Public, PeerId::random());
        create_connection_metadata(&peers_and_metadata, pfn_peer, ConnectionOrigin::Inbound);
        add_to_trusted_peers(&peers_and_metadata, pfn_peer);
        assert!(!is_high_priority_peer(
            base_config.clone(),
            peers_and_metadata.clone(),
            &vfn_peer
        ));

        // Create a PFN peer (with an inbound connection) and verify it is not high priority
        let pfn_peer = PeerNetworkId::new(NetworkId::Public, PeerId::random());
        create_connection_metadata(&peers_and_metadata, pfn_peer, ConnectionOrigin::Inbound);
        assert!(!is_high_priority_peer(
            base_config.clone(),
            peers_and_metadata.clone(),
            &vfn_peer
        ));

        // Create a PFN peer (with missing connection metadata) and verify it is not high priority
        let pfn_peer = PeerNetworkId::new(NetworkId::Public, PeerId::random());
        assert!(!is_high_priority_peer(
            base_config.clone(),
            peers_and_metadata.clone(),
            &pfn_peer
        ));
    }

    #[test]
    fn test_is_priority_peer_pfn() {
        // Create a base config for a PFN
        let base_config = Arc::new(BaseConfig {
            role: RoleType::FullNode,
            ..Default::default()
        });

        // Create a peers and metadata struct with the public networks registered
        let peers_and_metadata = PeersAndMetadata::new(&[NetworkId::Public]);

        // Create a PFN peer (with an outbound connection) and verify it is high priority
        let pfn_peer = PeerNetworkId::new(NetworkId::Public, PeerId::random());
        create_connection_metadata(&peers_and_metadata, pfn_peer, ConnectionOrigin::Outbound);
        assert!(is_high_priority_peer(
            base_config.clone(),
            peers_and_metadata.clone(),
            &pfn_peer
        ));

        // Create a trusted PFN peer (with an inbound connection) and verify it is high priority
        let pfn_peer = PeerNetworkId::new(NetworkId::Public, PeerId::random());
        create_connection_metadata(&peers_and_metadata, pfn_peer, ConnectionOrigin::Inbound);
        add_to_trusted_peers(&peers_and_metadata, pfn_peer);
        assert!(is_high_priority_peer(
            base_config.clone(),
            peers_and_metadata.clone(),
            &pfn_peer
        ));

        // Create a PFN peer (with an inbound connection) and verify it is not high priority
        let pfn_peer = PeerNetworkId::new(NetworkId::Public, PeerId::random());
        create_connection_metadata(&peers_and_metadata, pfn_peer, ConnectionOrigin::Inbound);
        assert!(!is_high_priority_peer(
            base_config.clone(),
            peers_and_metadata.clone(),
            &pfn_peer
        ));

        // Create a PFN peer (with missing connection metadata) and verify it is not high priority
        let pfn_peer = PeerNetworkId::new(NetworkId::Public, PeerId::random());
        assert!(!is_high_priority_peer(
            base_config.clone(),
            peers_and_metadata.clone(),
            &pfn_peer
        ));
    }

    #[test]
    fn test_validator_priorities() {
        // Create a base config for a validator
        let base_config = Arc::new(BaseConfig {
            role: RoleType::Validator,
            ..Default::default()
        });

        // Create a peers and metadata struct with all networks registered
        let peers_and_metadata =
            PeersAndMetadata::new(&[NetworkId::Validator, NetworkId::Vfn, NetworkId::Public]);

        // Create a validator peer and verify it is highly prioritized
        let validator_peer = PeerNetworkId::new(NetworkId::Validator, PeerId::random());
        assert_eq!(
            get_peer_priority(
                base_config.clone(),
                peers_and_metadata.clone(),
                &validator_peer
            ),
            PeerPriority::HighPriority
        );

        // Create a VFN peer and verify it is medium prioritized
        let vfn_peer = PeerNetworkId::new(NetworkId::Vfn, PeerId::random());
        assert_eq!(
            get_peer_priority(base_config.clone(), peers_and_metadata.clone(), &vfn_peer),
            PeerPriority::MediumPriority
        );

        // Create a PFN peer and verify it is low prioritized
        let pfn_peer = PeerNetworkId::new(NetworkId::Public, PeerId::random());
        assert_eq!(
            get_peer_priority(base_config.clone(), peers_and_metadata.clone(), &pfn_peer),
            PeerPriority::LowPriority
        );
    }

    #[test]
    fn test_vfn_priorities() {
        // Create a base config for a VFN
        let base_config = Arc::new(BaseConfig {
            role: RoleType::FullNode,
            ..Default::default()
        });

        // Create a peers and metadata struct with VFN and public networks registered
        let peers_and_metadata = PeersAndMetadata::new(&[NetworkId::Vfn, NetworkId::Public]);

        // Create a validator peer and verify it is highly prioritized
        let validator_peer = PeerNetworkId::new(NetworkId::Vfn, PeerId::random());
        assert_eq!(
            get_peer_priority(
                base_config.clone(),
                peers_and_metadata.clone(),
                &validator_peer
            ),
            PeerPriority::HighPriority
        );

        // Create a VFN peer (with an outbound connection) and verify it is medium prioritized
        let vfn_peer = PeerNetworkId::new(NetworkId::Public, PeerId::random());
        create_connection_metadata(&peers_and_metadata, vfn_peer, ConnectionOrigin::Outbound);
        assert_eq!(
            get_peer_priority(base_config.clone(), peers_and_metadata.clone(), &vfn_peer),
            PeerPriority::MediumPriority
        );

        // Create a trusted VFN peer (with an inbound connection) and verify it is medium prioritized
        let vfn_peer = PeerNetworkId::new(NetworkId::Public, PeerId::random());
        create_connection_metadata(&peers_and_metadata, vfn_peer, ConnectionOrigin::Inbound);
        add_to_trusted_peers(&peers_and_metadata, vfn_peer);
        assert_eq!(
            get_peer_priority(base_config.clone(), peers_and_metadata.clone(), &vfn_peer),
            PeerPriority::MediumPriority
        );

        // Create a PFN peer (with an outbound connection) and verify it is medium prioritized
        let pfn_peer = PeerNetworkId::new(NetworkId::Public, PeerId::random());
        create_connection_metadata(&peers_and_metadata, pfn_peer, ConnectionOrigin::Outbound);
        assert_eq!(
            get_peer_priority(base_config.clone(), peers_and_metadata.clone(), &pfn_peer),
            PeerPriority::MediumPriority
        );

        // Create a trusted PFN peer (with an inbound connection) and verify it is medium prioritized
        let pfn_peer = PeerNetworkId::new(NetworkId::Public, PeerId::random());
        create_connection_metadata(&peers_and_metadata, pfn_peer, ConnectionOrigin::Inbound);
        add_to_trusted_peers(&peers_and_metadata, pfn_peer);
        assert_eq!(
            get_peer_priority(base_config.clone(), peers_and_metadata.clone(), &pfn_peer),
            PeerPriority::MediumPriority
        );

        // Create a PFN peer (with an inbound connection) and verify it is low prioritized
        let pfn_peer = PeerNetworkId::new(NetworkId::Public, PeerId::random());
        create_connection_metadata(&peers_and_metadata, pfn_peer, ConnectionOrigin::Inbound);
        assert_eq!(
            get_peer_priority(base_config.clone(), peers_and_metadata.clone(), &pfn_peer),
            PeerPriority::LowPriority
        );

        // Create a PFN peer (with missing connection metadata) and verify it is low prioritized
        let pfn_peer = PeerNetworkId::new(NetworkId::Public, PeerId::random());
        assert_eq!(
            get_peer_priority(base_config.clone(), peers_and_metadata.clone(), &pfn_peer),
            PeerPriority::LowPriority
        );
    }

    #[test]
    fn test_pfn_priorities() {
        // Create a base config for a PFN
        let base_config = Arc::new(BaseConfig {
            role: RoleType::FullNode,
            ..Default::default()
        });

        // Create a peers and metadata struct with the public networks registered
        let peers_and_metadata = PeersAndMetadata::new(&[NetworkId::Public]);

        // Create a PFN peer (with an outbound connection) and verify it is highly prioritized
        let pfn_peer = PeerNetworkId::new(NetworkId::Public, PeerId::random());
        create_connection_metadata(&peers_and_metadata, pfn_peer, ConnectionOrigin::Outbound);
        assert_eq!(
            get_peer_priority(base_config.clone(), peers_and_metadata.clone(), &pfn_peer),
            PeerPriority::HighPriority
        );

        // Create a trusted PFN peer (with an inbound connection) and verify it is highly prioritized
        let pfn_peer = PeerNetworkId::new(NetworkId::Public, PeerId::random());
        create_connection_metadata(&peers_and_metadata, pfn_peer, ConnectionOrigin::Inbound);
        add_to_trusted_peers(&peers_and_metadata, pfn_peer);
        assert_eq!(
            get_peer_priority(base_config.clone(), peers_and_metadata.clone(), &pfn_peer),
            PeerPriority::HighPriority
        );

        // Create a PFN peer (with an inbound connection) and verify it is low prioritized
        let pfn_peer = PeerNetworkId::new(NetworkId::Public, PeerId::random());
        create_connection_metadata(&peers_and_metadata, pfn_peer, ConnectionOrigin::Inbound);
        assert_eq!(
            get_peer_priority(base_config.clone(), peers_and_metadata.clone(), &pfn_peer),
            PeerPriority::LowPriority
        );

        // Create a PFN peer (with missing connection metadata) and verify it is low prioritized
        let pfn_peer = PeerNetworkId::new(NetworkId::Public, PeerId::random());
        assert_eq!(
            get_peer_priority(base_config.clone(), peers_and_metadata.clone(), &pfn_peer),
            PeerPriority::LowPriority
        );
    }

    /// Adds the given peer to the trusted peers set
    fn add_to_trusted_peers(peers_and_metadata: &Arc<PeersAndMetadata>, peer: PeerNetworkId) {
        peers_and_metadata
            .set_trusted_peers(
                &peer.network_id(),
                hashmap! {peer.peer_id() => Peer::default()},
            )
            .unwrap();
    }

    /// Creates the connection metadata for the specified peer based on the given origin
    fn create_connection_metadata(
        peers_and_metadata: &Arc<PeersAndMetadata>,
        peer: PeerNetworkId,
        origin: ConnectionOrigin,
    ) {
        // Determine the peer role
        let peer_role = if origin == ConnectionOrigin::Outbound {
            PeerRole::Upstream
        } else {
            PeerRole::Unknown
        };

        // Update the connection metadata for the peer
        peers_and_metadata
            .insert_connection_metadata(
                peer,
                ConnectionMetadata::mock_with_role_and_origin(peer.peer_id(), peer_role, origin),
            )
            .unwrap();
    }
}
