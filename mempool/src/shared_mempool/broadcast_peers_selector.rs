// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_config::{config::PeerRole, network_id::PeerNetworkId};
use aptos_logger::info;
use aptos_network::application::metadata::PeerMetadata;
use aptos_types::{account_address::AccountAddress, transaction::Version, PeerId};
use itertools::Itertools;
use moka::sync::Cache;
use std::{
    cmp::Ordering,
    collections::{hash_map::RandomState, HashMap, HashSet},
    hash::{BuildHasher, Hasher},
    sync::Arc,
    time::Duration,
};

pub enum SelectedPeers {
    All,
    Selected(Vec<PeerNetworkId>),
    None,
}

impl From<Vec<PeerNetworkId>> for SelectedPeers {
    fn from(peers: Vec<PeerNetworkId>) -> Self {
        if peers.is_empty() {
            SelectedPeers::None
        } else {
            SelectedPeers::Selected(peers)
        }
    }
}

pub trait BroadcastPeersSelector: Send + Sync {
    fn update_peers(&mut self, updated_peers: &HashMap<PeerNetworkId, PeerMetadata>);
    // TODO: for backwards compatibility, an empty vector could mean we send to all?
    // TODO: for all the tests, just added an empty vector, need to audit later
    fn broadcast_peers(&self, account: &AccountAddress) -> SelectedPeers;
}

#[derive(Clone, Debug)]
struct PrioritizedPeersComparator {
    random_state: RandomState,
}

impl PrioritizedPeersComparator {
    fn new() -> Self {
        Self {
            random_state: RandomState::new(),
        }
    }

    /// Provides ordering for peers to send transactions to
    fn compare(
        &self,
        peer_a: &(PeerNetworkId, PeerRole),
        peer_b: &(PeerNetworkId, PeerRole),
    ) -> Ordering {
        let peer_network_id_a = peer_a.0;
        let peer_network_id_b = peer_b.0;

        // Sort by NetworkId
        match peer_network_id_a
            .network_id()
            .cmp(&peer_network_id_b.network_id())
        {
            Ordering::Equal => {
                // Then sort by Role
                let role_a = peer_a.1;
                let role_b = peer_b.1;
                match role_a.cmp(&role_b) {
                    // Tiebreak by hash_peer_id.
                    Ordering::Equal => {
                        let hash_a = self.hash_peer_id(&peer_network_id_a.peer_id());
                        let hash_b = self.hash_peer_id(&peer_network_id_b.peer_id());

                        hash_a.cmp(&hash_b)
                    },
                    ordering => ordering,
                }
            },
            ordering => ordering,
        }
    }

    /// Stable within a mempool instance but random between instances.
    fn hash_peer_id(&self, peer_id: &PeerId) -> u64 {
        let mut hasher = self.random_state.build_hasher();
        hasher.write(peer_id.as_ref());
        hasher.finish()
    }
}

pub struct AllPeersSelector {}

impl AllPeersSelector {
    pub fn new() -> Self {
        Self {}
    }
}

impl BroadcastPeersSelector for AllPeersSelector {
    fn update_peers(&mut self, _updated_peers: &HashMap<PeerNetworkId, PeerMetadata>) {
        // Do nothing
    }

    fn broadcast_peers(&self, _account: &AccountAddress) -> SelectedPeers {
        SelectedPeers::All
    }
}

pub struct PrioritizedPeersSelector {
    max_selected_peers: usize,
    prioritized_peers: Vec<PeerNetworkId>,
    prioritized_peers_comparator: PrioritizedPeersComparator,
}

impl PrioritizedPeersSelector {
    pub fn new(max_selected_peers: usize) -> Self {
        Self {
            max_selected_peers,
            prioritized_peers: Vec::new(),
            prioritized_peers_comparator: PrioritizedPeersComparator::new(),
        }
    }
}

impl BroadcastPeersSelector for PrioritizedPeersSelector {
    fn update_peers(&mut self, updated_peers: &HashMap<PeerNetworkId, PeerMetadata>) {
        self.prioritized_peers = updated_peers
            .iter()
            .map(|(peer, metadata)| (*peer, metadata.get_connection_metadata().role))
            .sorted_by(|peer_a, peer_b| self.prioritized_peers_comparator.compare(peer_a, peer_b))
            .map(|(peer, _)| peer)
            .collect();
    }

    fn broadcast_peers(&self, _account: &AccountAddress) -> SelectedPeers {
        let peers: Vec<_> = self
            .prioritized_peers
            .iter()
            .take(self.max_selected_peers)
            .cloned()
            .collect();
        info!(
            "prioritized_peers (len {}): {:?}",
            self.prioritized_peers.len(),
            peers
        );
        peers.into()
    }
}

pub struct FreshPeersSelector {
    max_selected_peers: usize,
    stickiness_cache: Arc<Cache<AccountAddress, Vec<PeerNetworkId>>>,
    sorted_peers: Vec<(PeerNetworkId, Version)>,
    peers: HashSet<PeerNetworkId>,
}

impl FreshPeersSelector {
    pub fn new(max_selected_peers: usize) -> Self {
        Self {
            max_selected_peers,
            stickiness_cache: Arc::new(
                Cache::builder()
                    .max_capacity(100_000)
                    .time_to_idle(Duration::from_secs(10))
                    .build(),
            ),
            sorted_peers: Vec::new(),
            peers: HashSet::new(),
        }
    }

    fn broadcast_peers_inner(&self, account: &PeerId) -> Vec<PeerNetworkId> {
        self.stickiness_cache.get_with_by_ref(account, || {
            let peers: Vec<_> = self
                .sorted_peers
                .iter()
                .rev()
                .take(self.max_selected_peers)
                .map(|(peer, _version)| *peer)
                .collect();
            // TODO: random shuffle among similar versions to keep from biasing
            // TODO: add a sample, completely remove
            info!(
                "fresh_peers: {:?} / total peers (len {}): {:?}",
                peers,
                self.sorted_peers.len(),
                self.sorted_peers
            );
            peers
        })
    }
}

impl BroadcastPeersSelector for FreshPeersSelector {
    fn update_peers(&mut self, updated_peers: &HashMap<PeerNetworkId, PeerMetadata>) {
        // TODO: Also need prioritized peers for VFN. Or is it always better to send to fresh peer?

        let mut peer_versions: Vec<_> = updated_peers
            .iter()
            .map(|(peer, metadata)| {
                if let Some(node_information) = metadata
                    .get_peer_monitoring_metadata()
                    .latest_node_info_response
                {
                    return (*peer, node_information.highest_synced_version);
                }
                (*peer, 0)
            })
            .collect();
        // TODO: what if we don't actually have a mempool connection to this host?
        // TODO: do we have to filter? or penalize but still allow selection?
        peer_versions.sort_by_key(|(_peer, version)| *version);
        info!("fresh_peers update_peers: {:?}", peer_versions);

        self.sorted_peers = peer_versions;
        self.peers = HashSet::from_iter(self.sorted_peers.iter().map(|(peer, _version)| *peer));
    }

    fn broadcast_peers(&self, account: &PeerId) -> SelectedPeers {
        let possibly_cached_results = self.broadcast_peers_inner(account);
        let mut peers: Vec<_> = possibly_cached_results
            .iter()
            .filter(|peer| self.peers.contains(peer))
            .cloned()
            .collect();
        if peers.is_empty() {
            self.stickiness_cache.remove(account);
            peers = self.broadcast_peers_inner(account);
            info!("fresh_peers, stickiness removed");
        }
        peers.into()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use aptos_config::{config::PeerRole, network_id::NetworkId};
    use aptos_types::PeerId;
    use std::cmp::Ordering;

    #[test]
    fn check_peer_prioritization() {
        let comparator = PrioritizedPeersComparator::new();

        let peer_id_1 = PeerId::from_hex_literal("0x1").unwrap();
        let peer_id_2 = PeerId::from_hex_literal("0x2").unwrap();
        let val_1 = (
            PeerNetworkId::new(NetworkId::Vfn, peer_id_1),
            PeerRole::Validator,
        );
        let val_2 = (
            PeerNetworkId::new(NetworkId::Vfn, peer_id_2),
            PeerRole::Validator,
        );
        let vfn_1 = (
            PeerNetworkId::new(NetworkId::Public, peer_id_1),
            PeerRole::ValidatorFullNode,
        );
        let preferred_1 = (
            PeerNetworkId::new(NetworkId::Public, peer_id_1),
            PeerRole::PreferredUpstream,
        );

        // NetworkId ordering
        assert_eq!(Ordering::Greater, comparator.compare(&vfn_1, &val_1));
        assert_eq!(Ordering::Less, comparator.compare(&val_1, &vfn_1));

        // PeerRole ordering
        assert_eq!(Ordering::Greater, comparator.compare(&vfn_1, &preferred_1));
        assert_eq!(Ordering::Less, comparator.compare(&preferred_1, &vfn_1));

        // Tiebreaker on peer_id
        let hash_1 = comparator.hash_peer_id(&val_1.0.peer_id());
        let hash_2 = comparator.hash_peer_id(&val_2.0.peer_id());

        assert_eq!(hash_2.cmp(&hash_1), comparator.compare(&val_2, &val_1));
        assert_eq!(hash_1.cmp(&hash_2), comparator.compare(&val_1, &val_2));

        // Same the only equal case
        assert_eq!(Ordering::Equal, comparator.compare(&val_1, &val_1));
    }
}
