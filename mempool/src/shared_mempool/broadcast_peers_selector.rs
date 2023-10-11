// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::counters;
use aptos_config::{config::PeerRole, network_id::PeerNetworkId};
use aptos_logger::prelude::*;
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

pub trait BroadcastPeersSelector: Send + Sync {
    fn update_peers(
        &mut self,
        updated_peers: &HashMap<PeerNetworkId, PeerMetadata>,
    ) -> (Vec<PeerNetworkId>, Vec<PeerNetworkId>);
    fn broadcast_peers(&self, account: &AccountAddress) -> Vec<PeerNetworkId>;
    fn num_peers_to_select(&self) -> usize;
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

pub struct PrioritizedPeersSelector {
    num_peers_to_select: usize,
    prioritized_peers: Vec<PeerNetworkId>,
    prioritized_peers_comparator: PrioritizedPeersComparator,
    peers: HashSet<PeerNetworkId>,
}

impl PrioritizedPeersSelector {
    pub fn new(num_peers_to_select: usize) -> Self {
        Self {
            num_peers_to_select,
            prioritized_peers: Vec::new(),
            prioritized_peers_comparator: PrioritizedPeersComparator::new(),
            peers: HashSet::new(),
        }
    }
}

impl BroadcastPeersSelector for PrioritizedPeersSelector {
    fn update_peers(
        &mut self,
        updated_peers: &HashMap<PeerNetworkId, PeerMetadata>,
    ) -> (Vec<PeerNetworkId>, Vec<PeerNetworkId>) {
        let new_peers = HashSet::from_iter(updated_peers.keys().cloned());
        let added: Vec<_> = new_peers.difference(&self.peers).cloned().collect();
        let removed: Vec<_> = self.peers.difference(&new_peers).cloned().collect();

        self.prioritized_peers = updated_peers
            .iter()
            .map(|(peer, metadata)| (*peer, metadata.get_connection_metadata().role))
            .sorted_by(|peer_a, peer_b| self.prioritized_peers_comparator.compare(peer_a, peer_b))
            .map(|(peer, _)| peer)
            .collect();

        (added, removed)
    }

    fn broadcast_peers(&self, _account: &AccountAddress) -> Vec<PeerNetworkId> {
        let peers: Vec<_> = self
            .prioritized_peers
            .iter()
            .take(self.num_peers_to_select)
            .cloned()
            .collect();
        info!(
            "prioritized_peers (len {}): {:?}",
            self.prioritized_peers.len(),
            peers
        );
        peers
    }

    fn num_peers_to_select(&self) -> usize {
        self.num_peers_to_select
    }
}

pub struct FreshPeersSelector {
    num_peers_to_select: usize,
    // TODO: what is a reasonable threshold? is there a way to make it time-based instead?
    // TODO: also, maybe only apply the threshold if there are more than num_peers_to_select peers?
    version_threshold: u64,
    // Note, only a single read happens at a time, so we don't use the thread-safeness of the cache
    stickiness_cache: Arc<Cache<AccountAddress, (u64, Vec<PeerNetworkId>)>>,
    // TODO: is there a data structure that can do peers and sorted_peers all at once?
    // Sorted in descending order (highest version first, i.e., up-to-date peers first)
    sorted_peers: Vec<(PeerNetworkId, Version)>,
    peers: HashSet<PeerNetworkId>,
    peers_generation: u64,
}

impl FreshPeersSelector {
    pub fn new(num_peers_to_select: usize, version_threshold: u64) -> Self {
        Self {
            num_peers_to_select,
            version_threshold,
            stickiness_cache: Arc::new(
                Cache::builder()
                    .max_capacity(100_000)
                    .time_to_idle(Duration::from_secs(10))
                    .build(),
            ),
            sorted_peers: Vec::new(),
            peers: HashSet::new(),
            peers_generation: 0,
        }
    }

    fn get_or_fill_stickiness_cache(&self, account: &PeerId) -> (u64, Vec<PeerNetworkId>) {
        self.stickiness_cache.get_with_by_ref(account, || {
            let peers: Vec<_> = self
                .sorted_peers
                .iter()
                .take(self.num_peers_to_select)
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
            (self.peers_generation, peers)
        })
    }

    fn broadcast_peers_inner(&self, account: &PeerId) -> Vec<PeerNetworkId> {
        // (1) get cached entry, or fill in with fresh peers
        let (generation, mut peers) = self.get_or_fill_stickiness_cache(account);

        // (2) if entry generation == current generation -- return
        if generation == self.peers_generation {
            return peers;
        }

        // (3) remove non-fresh peers
        peers.retain(|peer| self.peers.contains(peer));

        // (4) if not full, try to fill in more fresh peers
        if peers.len() < self.num_peers_to_select {
            let peers_cloned = peers.clone();
            let peers_set: HashSet<_> = HashSet::from_iter(peers_cloned.iter());
            let more_peers = self
                .sorted_peers
                .iter()
                .filter_map(|(peer, _version)| {
                    if !peers_set.contains(peer) {
                        Some(*peer)
                    } else {
                        None
                    }
                })
                .take(self.num_peers_to_select - peers.len());
            // add more_peers to end of peers
            peers.extend(more_peers);
        }

        // (5) update the stickiness cache
        self.stickiness_cache
            .insert(*account, (self.peers_generation, peers.clone()));

        peers
    }
}

impl BroadcastPeersSelector for FreshPeersSelector {
    fn update_peers(
        &mut self,
        updated_peers: &HashMap<PeerNetworkId, PeerMetadata>,
    ) -> (Vec<PeerNetworkId>, Vec<PeerNetworkId>) {
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
        // Sort in descending order (highest version first, i.e., up-to-date peers first)
        peer_versions.sort_by(|(_, version_a), (_, version_b)| version_b.cmp(version_a));
        info!("fresh_peers update_peers: {:?}", peer_versions);
        counters::SHARED_MEMPOOL_SELECTOR_NUM_PEERS.observe(peer_versions.len() as f64);

        // Select a minimum of num_peers_to_select, and include all peers within version_threshold
        let max_version = peer_versions
            .first()
            .map(|(_peer, version)| *version)
            .unwrap_or(0);
        let mut selected_peer_versions = vec![];
        let mut num_selected = 0;
        let mut num_fresh = 0;
        for (peer, version) in peer_versions {
            let mut to_select = false;
            if num_selected < self.num_peers_to_select {
                to_select = true;
            }
            if max_version - version <= self.version_threshold {
                to_select = true;
                num_fresh += 1;
            }
            if to_select {
                selected_peer_versions.push((peer, version));
                num_selected += 1;
            } else {
                break;
            }
        }
        counters::SHARED_MEMPOOL_SELECTOR_NUM_SELECTED_PEERS.observe(num_selected as f64);
        counters::SHARED_MEMPOOL_SELECTOR_NUM_FRESH_PEERS.observe(num_fresh as f64);

        let selected_peers =
            HashSet::from_iter(selected_peer_versions.iter().map(|(peer, _version)| *peer));
        let added: Vec<_> = selected_peers.difference(&self.peers).cloned().collect();
        let removed: Vec<_> = self.peers.difference(&selected_peers).cloned().collect();
        counters::SHARED_MEMPOOL_SELECTOR_REMOVED_PEERS.observe(removed.len() as f64);

        self.sorted_peers = selected_peer_versions;
        self.peers = selected_peers;

        (added, removed)
    }

    fn broadcast_peers(&self, account: &PeerId) -> Vec<PeerNetworkId> {
        self.broadcast_peers_inner(account)
    }

    fn num_peers_to_select(&self) -> usize {
        self.num_peers_to_select
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
