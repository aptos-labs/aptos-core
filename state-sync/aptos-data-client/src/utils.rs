// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::Error,
    logging::{LogEntry, LogEvent, LogSchema},
};
use aptos_config::{
    config::BaseConfig,
    network_id::{NetworkId, PeerNetworkId},
};
use aptos_logger::warn;
use aptos_netcore::transport::ConnectionOrigin;
use aptos_network::application::storage::PeersAndMetadata;
use itertools::Itertools;
use rand::seq::{IteratorRandom, SliceRandom};
use std::{collections::HashSet, sync::Arc};

/// Returns true iff the given peer is high-priority.
///
/// TODO(joshlind): make this less hacky using network topological awareness.
pub fn is_priority_peer(
    base_config: Arc<BaseConfig>,
    peers_and_metadata: Arc<PeersAndMetadata>,
    peer: &PeerNetworkId,
) -> bool {
    // Validators should only prioritize other validators
    let peer_network_id = peer.network_id();
    if base_config.role.is_validator() {
        return peer_network_id.is_validator_network();
    }

    // VFNs should only prioritize validators
    if peers_and_metadata
        .get_registered_networks()
        .contains(&NetworkId::Vfn)
    {
        return peer_network_id.is_vfn_network();
    }

    // PFNs should only prioritize outbound connections (this targets seed peers and VFNs)
    match peers_and_metadata.get_metadata_for_peer(*peer) {
        Ok(peer_metadata) => {
            if peer_metadata.get_connection_metadata().origin == ConnectionOrigin::Outbound {
                return true;
            }
        },
        Err(error) => {
            warn!(
                (LogSchema::new(LogEntry::PeerStates)
                    .event(LogEvent::PriorityAndRegularPeers)
                    .message(&format!(
                        "Unable to locate metadata for peer! Error: {:?}",
                        error
                    ))
                    .peer(peer))
            );
        },
    }

    false
}

/// Selects a single peer randomly from the list of specified peers
pub fn choose_random_peer(peers: HashSet<PeerNetworkId>) -> Option<PeerNetworkId> {
    peers.into_iter().choose(&mut rand::thread_rng())
}

/// Selects a set of peers randomly from the list of specified peers
pub fn choose_random_peers(
    num_peers_to_choose: u64,
    peers: HashSet<PeerNetworkId>,
) -> HashSet<PeerNetworkId> {
    let random_peers = peers
        .into_iter()
        .choose_multiple(&mut rand::thread_rng(), num_peers_to_choose as usize);
    random_peers.into_iter().collect()
}

/// Selects a set of peers randomly from the list of specified peers,
/// weighted by the peer's weight.
pub fn choose_random_peers_by_weight(
    num_peers_to_choose: u64,
    peers_and_weights: Vec<(PeerNetworkId, f64)>,
) -> Result<HashSet<PeerNetworkId>, Error> {
    peers_and_weights
        .choose_multiple_weighted(
            &mut rand::thread_rng(),
            num_peers_to_choose as usize,
            |peer| peer.1,
        )
        .map(|peers| peers.into_iter().map(|peer| peer.0).collect())
        .map_err(|error| Error::UnexpectedErrorEncountered(error.to_string()))
}

#[cfg(test)]
mod tests {
    use crate::utils::{
        choose_random_peer, choose_random_peers, choose_random_peers_by_weight, is_priority_peer,
    };
    use aptos_config::{
        config::{BaseConfig, PeerRole, RoleType},
        network_id::{NetworkId, PeerNetworkId},
    };
    use aptos_netcore::transport::ConnectionOrigin;
    use aptos_network::{application::storage::PeersAndMetadata, transport::ConnectionMetadata};
    use aptos_types::PeerId;
    use maplit::hashset;
    use std::{
        collections::{HashMap, HashSet},
        sync::Arc,
    };

    #[test]
    fn test_choose_random_peer() {
        // Choose a peer from an empty list, and verify none are returned
        let chosen_peer = choose_random_peer(hashset![]);
        assert!(chosen_peer.is_none());

        // Choose a peer from a list of length 1, and verify the peer is returned
        let peer = create_random_peer_network_id();
        let chosen_peer = choose_random_peer(hashset![peer]);
        assert_eq!(chosen_peer, Some(peer));

        // Choose a peer from a list of length 2, and verify a peer is returned
        let peer_1 = create_random_peer_network_id();
        let peer_2 = create_random_peer_network_id();
        let chosen_peer = choose_random_peer(hashset![peer_1, peer_2]);
        assert!(chosen_peer == Some(peer_1) || chosen_peer == Some(peer_2));

        // Choose a peer from a list of length 10, and verify a peer is returned
        let peers = (0..10)
            .map(|_| create_random_peer_network_id())
            .collect::<HashSet<_>>();
        let chosen_peer = choose_random_peer(peers);
        assert!(chosen_peer.is_some());
    }

    #[test]
    fn test_choose_random_peers() {
        // Choose 1 peer from an empty list, and verify none are returned
        let chosen_peers = choose_random_peers(1, hashset![]);
        assert!(chosen_peers.is_empty());

        // Choose 1 peer from a list of length 1, and verify the peer is returned
        let peer = create_random_peer_network_id();
        let chosen_peers = choose_random_peers(1, hashset![peer]);
        assert_eq!(chosen_peers, hashset![peer]);

        // Choose 2 peers from a list of length 2, and verify the peers are returned
        let peer_1 = create_random_peer_network_id();
        let peer_2 = create_random_peer_network_id();
        let chosen_peers = choose_random_peers(2, hashset![peer_1, peer_2]);
        assert_eq!(chosen_peers, hashset![peer_1, peer_2]);

        // Choose 5 peers from a list of length 2, and verify the peers are returned
        let peer_1 = create_random_peer_network_id();
        let peer_2 = create_random_peer_network_id();
        let chosen_peers = choose_random_peers(5, hashset![peer_1, peer_2]);
        assert_eq!(chosen_peers, hashset![peer_1, peer_2]);

        // Choose 5 peers from a list of length 10, and verify only 5 are returned
        let peers = (0..10)
            .map(|_| create_random_peer_network_id())
            .collect::<HashSet<_>>();
        let chosen_peers = choose_random_peers(5, peers);
        assert_eq!(chosen_peers.len(), 5);

        // Choose 0 peers from a list of length 10, and verify an empty set is returned
        let peers = (0..10)
            .map(|_| create_random_peer_network_id())
            .collect::<HashSet<_>>();
        let chosen_peers = choose_random_peers(0, peers);
        assert!(chosen_peers.is_empty());
    }

    #[test]
    fn test_choose_random_peers_by_weight() {
        // Choose 1 peer from an empty list, and verify none are returned
        let chosen_peers = choose_random_peers_by_weight(1, vec![]).unwrap();
        assert!(chosen_peers.is_empty());

        // Choose 1 peer from a list of length 1, and verify the peer is returned
        let peer = create_random_peer_network_id();
        let chosen_peers = choose_random_peers_by_weight(1, vec![(peer, 1.0)]).unwrap();
        assert_eq!(chosen_peers, hashset![peer]);

        // Choose 2 peers from a list of length 2, and verify the peers are returned
        let peer_1 = create_random_peer_network_id();
        let peer_2 = create_random_peer_network_id();
        let chosen_peers =
            choose_random_peers_by_weight(2, vec![(peer_1, 1.0), (peer_2, 1.0)]).unwrap();
        assert_eq!(chosen_peers, hashset![peer_1, peer_2]);

        // Choose 5 peers from a list of length 2, and verify the peers are returned
        let peer_1 = create_random_peer_network_id();
        let peer_2 = create_random_peer_network_id();
        let chosen_peers =
            choose_random_peers_by_weight(5, vec![(peer_1, 1.0), (peer_2, 1.0)]).unwrap();
        assert_eq!(chosen_peers, hashset![peer_1, peer_2]);

        // Choose 5 peers from a list of length 10, and verify only 5 are returned
        let peers_and_weights = (0..10)
            .map(|_| (create_random_peer_network_id(), 1.0))
            .collect::<Vec<_>>();
        let chosen_peers = choose_random_peers_by_weight(5, peers_and_weights).unwrap();
        assert_eq!(chosen_peers.len(), 5);

        // Choose 0 peers from a list of length 10, and verify an empty set is returned
        let peers_and_weights = (0..10)
            .map(|_| (create_random_peer_network_id(), 1.0))
            .collect::<Vec<_>>();
        let chosen_peers = choose_random_peers_by_weight(0, peers_and_weights).unwrap();
        assert!(chosen_peers.is_empty());

        // Create a set of peers with decreasing weights
        let peer_1 = create_random_peer_network_id();
        let peer_2 = create_random_peer_network_id();
        let peer_3 = create_random_peer_network_id();
        let peers_and_weights = vec![(peer_1, 1000.0), (peer_2, 100.0), (peer_3, 1.0)];

        // Choose a single peer multiple times and track the selection counts
        let mut chosen_peers_and_counts = HashMap::new();
        for _ in 0..1_000_000 {
            // Choose the peer and verify only 1 is returned
            let chosen_peers = choose_random_peers_by_weight(1, peers_and_weights.clone()).unwrap();
            assert_eq!(chosen_peers.len(), 1);

            // Update the peer counts
            let chosen_peer = chosen_peers.into_iter().next().unwrap();
            *chosen_peers_and_counts.entry(chosen_peer).or_insert(0) += 1;
        }

        // Verify that the peer counts decrease with decreasing weight
        let peer_count_1 = chosen_peers_and_counts.get(&peer_1).unwrap_or(&0);
        let peer_count_2 = chosen_peers_and_counts.get(&peer_2).unwrap_or(&0);
        let peer_count_3 = chosen_peers_and_counts.get(&peer_3).unwrap_or(&0);
        assert!(peer_count_1 > peer_count_2);
        assert!(peer_count_2 > peer_count_3);
    }

    #[test]
    fn test_is_priority_peer_validator() {
        // Create a base config for a validator node
        let base_config = Arc::new(BaseConfig {
            role: RoleType::Validator,
            ..Default::default()
        });

        // Create a peers and metadata struct with all networks registered
        let peers_and_metadata =
            PeersAndMetadata::new(&[NetworkId::Validator, NetworkId::Vfn, NetworkId::Public]);

        // Create a VFN peer and verify it is not prioritized
        let vfn_peer = PeerNetworkId::new(NetworkId::Vfn, PeerId::random());
        assert!(!is_priority_peer(
            base_config.clone(),
            peers_and_metadata.clone(),
            &vfn_peer
        ));

        // Create a PFN peer and verify it is not prioritized
        let pfn_peer = PeerNetworkId::new(NetworkId::Public, PeerId::random());
        assert!(!is_priority_peer(
            base_config.clone(),
            peers_and_metadata.clone(),
            &pfn_peer
        ));

        // Create a validator peer and verify it is prioritized
        let validator_peer = PeerNetworkId::new(NetworkId::Validator, PeerId::random());
        assert!(is_priority_peer(
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

        // Create a peers and metadata struct with a VFN network
        let peers_and_metadata = PeersAndMetadata::new(&[NetworkId::Vfn]);

        // Create a PFN peer and verify it is not prioritized
        let pfn_peer = PeerNetworkId::new(NetworkId::Public, PeerId::random());
        assert!(!is_priority_peer(
            base_config.clone(),
            peers_and_metadata.clone(),
            &pfn_peer
        ));

        // Create a validator peer and verify it is prioritized
        let validator_peer = PeerNetworkId::new(NetworkId::Vfn, PeerId::random());
        assert!(is_priority_peer(
            base_config.clone(),
            peers_and_metadata.clone(),
            &validator_peer
        ));
    }

    #[test]
    fn test_is_priority_peer_pfn() {
        // Create a base config for a PFN
        let base_config = Arc::new(BaseConfig {
            role: RoleType::FullNode,
            ..Default::default()
        });

        // Create two PFN peers
        let pfn_peer_1 = PeerNetworkId::new(NetworkId::Public, PeerId::random());
        let pfn_peer_2 = PeerNetworkId::new(NetworkId::Public, PeerId::random());

        // Create a peers and metadata struct with a PFN network
        let peers_and_metadata = PeersAndMetadata::new(&[NetworkId::Public]);

        // Insert the connection metadata for PFN 1 and
        // mark it as having dialed us.
        let connection_metadata = ConnectionMetadata::mock_with_role_and_origin(
            pfn_peer_1.peer_id(),
            PeerRole::Unknown,
            ConnectionOrigin::Inbound,
        );
        peers_and_metadata
            .insert_connection_metadata(pfn_peer_1, connection_metadata)
            .unwrap();

        // Insert the connection metadata for PFN 2 and
        // mark it as having been dialed by us.
        let connection_metadata = ConnectionMetadata::mock_with_role_and_origin(
            pfn_peer_2.peer_id(),
            PeerRole::Upstream,
            ConnectionOrigin::Outbound,
        );
        peers_and_metadata
            .insert_connection_metadata(pfn_peer_2, connection_metadata)
            .unwrap();

        // Verify that PFN 1 is not prioritized
        assert!(!is_priority_peer(
            base_config.clone(),
            peers_and_metadata.clone(),
            &pfn_peer_1
        ));

        // Verify that PFN 2 is prioritized
        assert!(is_priority_peer(
            base_config.clone(),
            peers_and_metadata.clone(),
            &pfn_peer_2
        ));
    }

    /// Creates and returns a random peer network ID
    fn create_random_peer_network_id() -> PeerNetworkId {
        // Create a random network ID
        let network_id = match rand::random::<u8>() % 3 {
            0 => NetworkId::Validator,
            1 => NetworkId::Vfn,
            _ => NetworkId::Public,
        };

        // Create a random peer ID
        let peer_id = PeerId::random();

        PeerNetworkId::new(network_id, peer_id)
    }
}
