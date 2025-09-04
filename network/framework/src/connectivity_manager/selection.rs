// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    connectivity_manager::{DiscoveredPeer, DiscoveredPeerSet},
    logging::NetworkSchema,
};
use velor_config::network_id::NetworkContext;
use velor_infallible::RwLock;
use velor_logger::error;
use velor_types::PeerId;
use maplit::hashset;
use ordered_float::OrderedFloat;
use rand_latest::prelude::*;
use std::{cmp::Ordering, collections::HashSet, sync::Arc};

/// Chooses peers to dial randomly from the given list of eligible
/// peers. We take last dial times into account to ensure that we
/// don't dial the same peers too frequently.
pub fn choose_peers_to_dial_randomly(
    mut eligible_peers: Vec<(PeerId, DiscoveredPeer)>,
    num_peers_to_dial: usize,
) -> Vec<(PeerId, DiscoveredPeer)> {
    // Shuffle the peers (so that we don't always dial the same ones first)
    eligible_peers.shuffle(&mut ::rand_latest::thread_rng());

    // Sort the peers by priority (this takes into account last dial times)
    eligible_peers
        .sort_by(|(_, peer), (_, other)| peer.partial_cmp(other).unwrap_or(Ordering::Equal));

    // Select the peers to dial
    eligible_peers.into_iter().take(num_peers_to_dial).collect()
}

/// Chooses peers randomly weighted by latency from the given list of peers
pub fn choose_random_peers_by_ping_latency(
    network_context: NetworkContext,
    eligible_peers: Vec<(PeerId, DiscoveredPeer)>,
    num_peers_to_choose: usize,
    discovered_peers: Arc<RwLock<DiscoveredPeerSet>>,
) -> Vec<(PeerId, DiscoveredPeer)> {
    // Get all eligible peer IDs
    let eligible_peer_ids = eligible_peers
        .iter()
        .map(|(peer_id, _)| *peer_id)
        .collect::<HashSet<_>>();

    // Identify the peer IDs that haven't been dialed recently
    let non_recently_dialed_peer_ids = eligible_peers
        .iter()
        .filter(|(_, peer)| !peer.has_dialed_recently())
        .map(|(peer_id, _)| *peer_id)
        .collect::<HashSet<_>>();

    // Choose peers (weighted by latency) from the non-recently dialed peers
    let mut selected_peer_ids = choose_peers_by_ping_latency(
        &network_context,
        &non_recently_dialed_peer_ids,
        num_peers_to_choose,
        discovered_peers.clone(),
    );

    // If not enough peers were selected, choose additional peers weighted by latency
    let num_selected_peer_ids = selected_peer_ids.len();
    if num_selected_peer_ids < num_peers_to_choose {
        // Filter out the already selected peers
        let unselected_peer_ids = get_unselected_peer_ids(&eligible_peer_ids, &selected_peer_ids);

        // Choose the remaining peers weighted by latency
        let num_remaining_peers = num_peers_to_choose.saturating_sub(num_selected_peer_ids);
        let remaining_selected_peer_ids = choose_peers_by_ping_latency(
            &network_context,
            &unselected_peer_ids,
            num_remaining_peers,
            discovered_peers.clone(),
        );

        // Extend the selected peers with the remaining peers
        selected_peer_ids.extend(remaining_selected_peer_ids);
    }

    // Extend the selected peers with random peers (if necessary)
    let selected_peer_ids =
        extend_with_random_peers(selected_peer_ids, &eligible_peer_ids, num_peers_to_choose);

    // Return the selected peers
    get_discovered_peers_for_ids(selected_peer_ids, discovered_peers)
}

/// Returns true iff peers should be selected by ping latency. Note: this only
/// makes sense for the public network, as the validator and VFN networks
/// establish all-to-all connections.
pub fn should_select_peers_by_latency(
    network_context: &NetworkContext,
    enable_latency_aware_dialing: bool,
) -> bool {
    network_context.network_id().is_public_network() && enable_latency_aware_dialing
}

/// Selects the specified number of peers from the list of potential
/// peers. Peer selection is weighted by peer latencies (i.e., the
/// lower the ping latency, the higher the probability of selection).
fn choose_peers_by_ping_latency(
    network_context: &NetworkContext,
    peer_ids: &HashSet<PeerId>,
    num_peers_to_choose: usize,
    discovered_peers: Arc<RwLock<DiscoveredPeerSet>>,
) -> HashSet<PeerId> {
    // If no peers can be chosen, return an empty list
    if num_peers_to_choose == 0 || peer_ids.is_empty() {
        return hashset![];
    }

    // Gather the latency weights for all peers
    let mut peer_ids_and_latency_weights = vec![];
    for peer_id in peer_ids {
        if let Some(ping_latency_secs) = discovered_peers.read().get_ping_latency_secs(peer_id) {
            let latency_weight = convert_latency_to_weight(ping_latency_secs);
            peer_ids_and_latency_weights.push((peer_id, OrderedFloat(latency_weight)));
        }
    }

    // Get the random peers by weight
    let weighted_selected_peers = peer_ids_and_latency_weights
        .choose_multiple_weighted(
            &mut ::rand_latest::thread_rng(),
            num_peers_to_choose,
            |peer| peer.1,
        )
        .map(|peers| peers.into_iter().map(|peer| *peer.0).collect::<Vec<_>>());

    // Return the random peers by weight
    weighted_selected_peers
        .unwrap_or_else(|error| {
            // We failed to select any peers
            error!(
                NetworkSchema::new(network_context),
                "Failed to choose peers by latency for network context: {:?}. Error: {:?}",
                network_context,
                error
            );
            vec![]
        })
        .into_iter()
        .collect::<HashSet<_>>()
}

/// Converts the given latency measurement to a weight. The weight
/// is calculated as the inverse of the latency, with a scaling
/// factor to ensure that low latency peers are highly weighted.
fn convert_latency_to_weight(latency_secs: f64) -> f64 {
    // If the latency is <= 0, something has gone wrong, so return 0.
    if latency_secs <= 0.0 {
        return 0.0;
    }

    // Invert the latency to get the weight
    let mut weight = 1.0 / latency_secs;

    // For every 25ms of latency, reduce the weight by 1/2 (to
    // ensure that low latency peers are highly weighted)
    let num_reductions = (latency_secs / 0.025) as usize;
    for _ in 0..num_reductions {
        weight /= 2.0;
    }

    weight
}

/// If the number of selected peers is less than the number of required peers,
/// select remaining peers from the serviceable peers (at random).
fn extend_with_random_peers(
    mut selected_peer_ids: HashSet<PeerId>,
    peer_ids: &HashSet<PeerId>,
    num_required_peers: usize,
) -> HashSet<PeerId> {
    // Only select random peers if we don't have enough peers
    let num_selected_peers = selected_peer_ids.len();
    if num_selected_peers < num_required_peers {
        // Filter out the already selected peers
        let unselected_peer_ids = get_unselected_peer_ids(peer_ids, &selected_peer_ids);

        // Randomly select the remaining peers
        let num_remaining_peers = num_required_peers.saturating_sub(num_selected_peers);
        let remaining_peer_ids = unselected_peer_ids
            .into_iter()
            .choose_multiple(&mut ::rand_latest::thread_rng(), num_remaining_peers);

        // Add the remaining peers to the selected peers
        selected_peer_ids.extend(remaining_peer_ids);
    }

    selected_peer_ids
}

/// Returns the discovered peer states for the given peer ids
fn get_discovered_peers_for_ids(
    peer_ids: HashSet<PeerId>,
    discovered_peers: Arc<RwLock<DiscoveredPeerSet>>,
) -> Vec<(PeerId, DiscoveredPeer)> {
    peer_ids
        .into_iter()
        .filter_map(|peer_id| {
            discovered_peers
                .read()
                .peer_set
                .get(&peer_id)
                .map(|peer| (peer_id, peer.clone()))
        })
        .collect()
}

/// Returns the unselected peer IDs from the given set of eligible and selected peer IDs
fn get_unselected_peer_ids(
    eligible_peer_ids: &HashSet<PeerId>,
    selected_peer_ids: &HashSet<PeerId>,
) -> HashSet<PeerId> {
    eligible_peer_ids
        .difference(selected_peer_ids)
        .cloned()
        .collect()
}

#[cfg(test)]
mod test {
    use super::*;
    use velor_config::{
        config::{PeerRole, RoleType},
        network_id::NetworkId,
    };
    use velor_types::account_address::AccountAddress;
    use rand::Rng;
    use std::collections::{BinaryHeap, HashMap};

    #[test]
    fn test_choose_random_peers() {
        // Create an empty eligible peers set
        let eligible_peers = vec![];

        // Choose several peers randomly and verify none are selected
        let selected_peers = choose_peers_to_dial_randomly(eligible_peers, 5);
        assert!(selected_peers.is_empty());

        // Create a large set of eligible peers
        let eligible_peers = create_eligible_peers(100);

        // Choose several peers randomly and verify the number of selected peers
        let num_peers_to_dial = 5;
        let selected_peers = choose_peers_to_dial_randomly(eligible_peers, num_peers_to_dial);
        assert_eq!(selected_peers.len(), num_peers_to_dial);

        // Create a small set of eligible peers
        let num_eligible_peers = 5;
        let eligible_peers = create_eligible_peers(num_eligible_peers);

        // Choose many peers randomly and verify the number of selected peers
        let selected_peers = choose_peers_to_dial_randomly(eligible_peers, 20);
        assert_eq!(selected_peers.len(), num_eligible_peers);
    }

    #[test]
    fn test_choose_random_peers_shuffle() {
        // Create a set of 10 eligible peers
        let num_eligible_peers = 10;
        let eligible_peers = create_eligible_peers(num_eligible_peers);

        // Choose all the peers randomly and verify the number of selected peers
        let selected_peers_1 =
            choose_peers_to_dial_randomly(eligible_peers.clone(), num_eligible_peers);
        assert_eq!(selected_peers_1.len(), num_eligible_peers);

        // Choose all the peers randomly again and verify the number of selected peers
        let selected_peers_2 = choose_peers_to_dial_randomly(eligible_peers, num_eligible_peers);
        assert_eq!(selected_peers_2.len(), num_eligible_peers);

        // Verify the selected peer sets are identical
        for peer in selected_peers_1.clone() {
            assert!(selected_peers_2.contains(&peer));
        }

        // Verify that the peer orders are different (the peers were shuffled randomly!)
        assert_ne!(selected_peers_1, selected_peers_2);
    }

    #[test]
    fn test_choose_random_peers_recently_dialed() {
        // Create a set of eligible peers
        let mut eligible_peers = vec![];

        // Add peers that have not been dialed recently
        let num_non_dialed_peers = 20;
        let non_dialed_peers = insert_non_dialed_peers(num_non_dialed_peers, &mut eligible_peers);

        // Add peers that have been dialed recently
        let num_dialed_peers = 60;
        let dialed_peers = insert_dialed_peers(num_dialed_peers, &mut eligible_peers);

        // Choose various peers randomly (until the max non-dialed peers) and verify the selection
        for num_peers_to_dial in 1..=num_non_dialed_peers {
            // Choose peers randomly and verify the number of selected peers
            let selected_peers =
                choose_peers_to_dial_randomly(eligible_peers.clone(), num_peers_to_dial);
            assert_eq!(selected_peers.len(), num_peers_to_dial);

            // Verify that all of the selected peers were not dialed recently
            for (peer_id, _) in selected_peers {
                assert!(non_dialed_peers.contains(&peer_id));
                assert!(!dialed_peers.contains(&peer_id));
            }
        }

        // Choose various peers randomly (beyond the max non-dialed peers) and verify the selection
        let mut non_dialed_peer_selected = false;
        let mut dialed_peer_selected = false;
        let total_num_peers = num_non_dialed_peers + num_dialed_peers;
        for num_peers_to_dial in num_non_dialed_peers + 1..=total_num_peers {
            // Choose peers randomly and verify the number of selected peers
            let selected_peers =
                choose_peers_to_dial_randomly(eligible_peers.clone(), num_peers_to_dial);
            assert_eq!(selected_peers.len(), num_peers_to_dial);

            // Update the selected peer flags
            for (peer_id, _) in selected_peers {
                if non_dialed_peers.contains(&peer_id) {
                    non_dialed_peer_selected = true;
                }
                if dialed_peers.contains(&peer_id) {
                    dialed_peer_selected = true;
                }
            }

            // Verify that at least one of each peer type was selected
            assert!(non_dialed_peer_selected);
            assert!(dialed_peer_selected);
        }
    }

    #[test]
    fn test_choose_peers_by_latency_dialed() {
        // Create a set of eligible peers
        let mut eligible_peers = vec![];

        // Add peers that have not been dialed recently
        let num_non_dialed_peers = 30;
        let non_dialed_peers = insert_non_dialed_peers(num_non_dialed_peers, &mut eligible_peers);

        // Add peers that have been dialed recently
        let num_dialed_peers = 30;
        let dialed_peers = insert_dialed_peers(num_dialed_peers, &mut eligible_peers);

        // Create the discovered peer set
        let discovered_peers = create_discovered_peers(eligible_peers.clone(), true);

        // Choose peers by latency (until the max non-dialed peers) and verify the selection
        for num_peers_to_dial in 1..=num_non_dialed_peers {
            // Choose peers by latency and verify the number of selected peers
            let selected_peers = choose_random_peers_by_ping_latency(
                NetworkContext::mock(),
                eligible_peers.clone(),
                num_peers_to_dial,
                discovered_peers.clone(),
            );
            assert_eq!(selected_peers.len(), num_peers_to_dial);

            // Verify that all of the selected peers were not dialed recently
            for (peer_id, _) in selected_peers {
                assert!(non_dialed_peers.contains(&peer_id));
                assert!(!dialed_peers.contains(&peer_id));
            }
        }

        // Choose peers by latency (beyond the max non-dialed peers) and verify the selection
        let total_num_peers = num_non_dialed_peers + num_dialed_peers;
        for num_peers_to_dial in num_non_dialed_peers + 1..=total_num_peers {
            // Choose peers by latency and verify the number of selected peers
            let selected_peers = choose_random_peers_by_ping_latency(
                NetworkContext::mock(),
                eligible_peers.clone(),
                num_peers_to_dial,
                discovered_peers.clone(),
            );
            assert_eq!(selected_peers.len(), num_peers_to_dial);

            // Get the selected peer IDs
            let selected_peer_ids = selected_peers
                .iter()
                .map(|(peer_id, _)| *peer_id)
                .collect::<HashSet<_>>();

            // Verify the peer selection
            for non_dialed_peer in non_dialed_peers.clone() {
                assert!(selected_peer_ids.contains(&non_dialed_peer));
            }

            // Verify that at least some dialed peers were selected
            let dialed_selected_peers = non_dialed_peers
                .difference(&selected_peer_ids)
                .cloned()
                .collect::<HashSet<_>>();
            assert!(dialed_peers.is_superset(&dialed_selected_peers));
        }
    }

    #[test]
    fn test_choose_peers_by_latency_missing_pings() {
        // Create an empty set of eligible peers
        let mut eligible_peers = vec![];

        // Choose several peers by latency and verify none are selected
        let network_context = NetworkContext::mock();
        let discovered_peers = Arc::new(RwLock::new(DiscoveredPeerSet::default()));
        let selected_peers = choose_random_peers_by_ping_latency(
            network_context,
            eligible_peers.clone(),
            5,
            discovered_peers.clone(),
        );
        assert!(selected_peers.is_empty());

        // Add peers that have not been dialed recently
        let num_non_dialed_peers = 30;
        let _ = insert_non_dialed_peers(num_non_dialed_peers, &mut eligible_peers);

        // Create the discovered peer set (without ping latencies)
        let discovered_peers = create_discovered_peers(eligible_peers.clone(), false);

        // Choose several peers by latency and verify the number of selected peers
        let num_peers_to_choose = 5;
        let selected_peers = choose_random_peers_by_ping_latency(
            network_context,
            eligible_peers.clone(),
            num_peers_to_choose,
            discovered_peers.clone(),
        );
        assert_eq!(selected_peers.len(), num_peers_to_choose);

        // Choose all peers by latency and verify the number of selected peers
        let selected_peers = choose_random_peers_by_ping_latency(
            network_context,
            eligible_peers.clone(),
            num_non_dialed_peers,
            discovered_peers.clone(),
        );
        assert_eq!(selected_peers.len(), num_non_dialed_peers);

        // Choose more peers by latency than are available and verify the number of selected peers
        let selected_peers = choose_random_peers_by_ping_latency(
            network_context,
            eligible_peers.clone(),
            num_non_dialed_peers + 1,
            discovered_peers.clone(),
        );
        assert_eq!(selected_peers.len(), num_non_dialed_peers);

        // Add peers that have been dialed recently (with no ping latencies)
        let num_dialed_peers = 30;
        let _ = insert_dialed_peers(num_dialed_peers, &mut eligible_peers);

        // Create the discovered peer set (without ping latencies)
        let discovered_peers = create_discovered_peers(eligible_peers.clone(), false);

        // Choose more peers than non dialed-peers and verify the number of selected peers
        let num_peers_to_choose = num_non_dialed_peers + 10;
        let selected_peers = choose_random_peers_by_ping_latency(
            network_context,
            eligible_peers.clone(),
            num_peers_to_choose,
            discovered_peers.clone(),
        );
        assert_eq!(selected_peers.len(), num_peers_to_choose);

        // Choose all peers by latency and verify the number of selected peers
        let num_peers_to_choose = num_non_dialed_peers + num_dialed_peers;
        let selected_peers = choose_random_peers_by_ping_latency(
            network_context,
            eligible_peers.clone(),
            num_peers_to_choose,
            discovered_peers.clone(),
        );
        assert_eq!(selected_peers.len(), num_peers_to_choose);

        // Choose more peers than are available and verify the number of selected peers
        let num_total_peers = num_non_dialed_peers + num_dialed_peers;
        let selected_peers = choose_random_peers_by_ping_latency(
            network_context,
            eligible_peers.clone(),
            num_total_peers + 10,
            discovered_peers.clone(),
        );
        assert_eq!(selected_peers.len(), num_total_peers);
    }

    #[test]
    fn test_choose_peers_by_latency_prioritized_dialed() {
        // Create a set of eligible peers
        let mut eligible_peers = vec![];

        // Add peers that have been dialed recently
        let num_dialed_peers = 100;
        let dialed_peers = insert_dialed_peers(num_dialed_peers, &mut eligible_peers);

        // Create the discovered peer set
        let discovered_peers = create_discovered_peers(eligible_peers.clone(), true);

        // Add peers that have not been dialed recently (with no ping latencies)
        let num_non_dialed_peers = 100;
        let non_dialed_peers = insert_non_dialed_peers(num_non_dialed_peers, &mut eligible_peers);

        // Choose peers by latency (multiple times) and verify the selection
        let mut peer_selection_counts = HashMap::new();
        for _ in 0..5000 {
            // Choose a single peer by latency and verify the number of selected peers
            let num_peers_to_dial = 1;
            let selected_peers = choose_random_peers_by_ping_latency(
                NetworkContext::mock(),
                eligible_peers.clone(),
                num_peers_to_dial,
                discovered_peers.clone(),
            );
            assert_eq!(selected_peers.len(), num_peers_to_dial);

            // Verify the selection and update the peer selection counts
            for (peer_id, _) in selected_peers {
                // Verify that the peer was dialed recently
                assert!(!non_dialed_peers.contains(&peer_id));
                assert!(dialed_peers.contains(&peer_id));

                // Update the peer selection counts
                let count = peer_selection_counts.entry(peer_id).or_insert(0);
                *count += 1;
            }
        }

        // Verify the top 10% of selected peers are the lowest latency peers
        verify_highest_peer_selection_latencies(discovered_peers.clone(), &peer_selection_counts);
    }

    #[test]
    fn test_choose_peers_by_latency_prioritized_non_dialed() {
        // Create a set of eligible peers
        let mut eligible_peers = vec![];

        // Add peers that have not been dialed recently
        let num_non_dialed_peers = 100;
        let non_dialed_peers = insert_non_dialed_peers(num_non_dialed_peers, &mut eligible_peers);

        // Add peers that have been dialed recently
        let num_dialed_peers = 100;
        let dialed_peers = insert_dialed_peers(num_dialed_peers, &mut eligible_peers);

        // Create the discovered peer set (with ping latencies)
        let discovered_peers = create_discovered_peers(eligible_peers.clone(), true);

        // Choose peers by latency (multiple times) and verify the selection
        let mut peer_selection_counts = HashMap::new();
        for _ in 0..5000 {
            // Choose a single peer by latency and verify the number of selected peers
            let num_peers_to_dial = 1;
            let selected_peers = choose_random_peers_by_ping_latency(
                NetworkContext::mock(),
                eligible_peers.clone(),
                num_peers_to_dial,
                discovered_peers.clone(),
            );
            assert_eq!(selected_peers.len(), num_peers_to_dial);

            // Verify the selection and update the peer selection counts
            for (peer_id, _) in selected_peers {
                // Verify that the peer was not dialed recently
                assert!(non_dialed_peers.contains(&peer_id));
                assert!(!dialed_peers.contains(&peer_id));

                // Update the peer selection counts
                let count = peer_selection_counts.entry(peer_id).or_insert(0);
                *count += 1;
            }
        }

        // Verify the top 10% of selected peers are the lowest latency peers
        verify_highest_peer_selection_latencies(discovered_peers.clone(), &peer_selection_counts);
    }

    #[test]
    fn test_latency_to_weights() {
        // Verify that a latency of 0 has a weight of 0
        assert_eq!(convert_latency_to_weight(0.0), 0.0);

        // Verify that latencies are scaled exponentially
        assert_eq!(convert_latency_to_weight(0.001), 1000.0);
        assert_eq!(convert_latency_to_weight(0.005), 200.0);
        assert_eq!(convert_latency_to_weight(0.01), 100.0);
        assert_eq!(convert_latency_to_weight(0.02), 50.0);
        assert_eq!(convert_latency_to_weight(0.025), 20.0);
        assert_eq!(convert_latency_to_weight(0.05), 5.0);
        assert_eq!(convert_latency_to_weight(0.1), 0.625);
        assert_eq!(convert_latency_to_weight(0.2), 0.01953125);
    }

    #[test]
    fn test_should_select_peers_by_latency() {
        // Create a validator network context
        let validator_network_context =
            NetworkContext::new(RoleType::Validator, NetworkId::Validator, PeerId::random());

        // Verify that we don't select peers by latency for the validator network
        let enable_latency_aware_dialing = true;
        assert!(!should_select_peers_by_latency(
            &validator_network_context,
            enable_latency_aware_dialing
        ));

        // Create a VFN network context
        let vfn_network_context =
            NetworkContext::new(RoleType::FullNode, NetworkId::Vfn, PeerId::random());

        // Verify that we don't select peers by latency for the VFN network
        let enable_latency_aware_dialing = true;
        assert!(!should_select_peers_by_latency(
            &vfn_network_context,
            enable_latency_aware_dialing
        ));

        // Create a public network context
        let public_network_context =
            NetworkContext::new(RoleType::FullNode, NetworkId::Public, PeerId::random());

        // Verify that we select peers by latency for the public network
        let enable_latency_aware_dialing = true;
        assert!(should_select_peers_by_latency(
            &public_network_context,
            enable_latency_aware_dialing
        ));

        // Disable peer ping latencies and verify that we don't select peers by latency
        let enable_latency_aware_dialing = false;
        assert!(!should_select_peers_by_latency(
            &public_network_context,
            enable_latency_aware_dialing
        ));
    }

    /// Creates a set of discovered peers from the given eligible
    /// peers. If `set_ping_latencies` is true, random ping latencies
    /// are set for each peer.
    fn create_discovered_peers(
        eligible_peers: Vec<(PeerId, DiscoveredPeer)>,
        set_ping_latencies: bool,
    ) -> Arc<RwLock<DiscoveredPeerSet>> {
        // Create a new discovered peer set
        let mut peer_set = HashMap::new();
        for (peer_id, mut peer) in eligible_peers {
            // Set a random ping latency between 1 and 1000 ms (if required)
            if set_ping_latencies {
                let ping_latency_ms = rand::thread_rng().gen_range(1, 1000);
                let ping_latency_secs = ping_latency_ms as f64 / 1000.0;
                peer.set_ping_latency_secs(ping_latency_secs);
            }

            // Insert the peer into the set
            peer_set.insert(peer_id, peer.clone());
        }

        // Create and return the discovered peers
        Arc::new(RwLock::new(DiscoveredPeerSet::new_from_peer_set(peer_set)))
    }

    /// Creates a set of eligible peers (as specified by the number of peers)
    fn create_eligible_peers(num_eligible_peers: usize) -> Vec<(PeerId, DiscoveredPeer)> {
        let mut eligible_peers = vec![];
        for _ in 0..num_eligible_peers {
            eligible_peers.push((
                AccountAddress::random(),
                DiscoveredPeer::new(PeerRole::PreferredUpstream),
            ));
        }
        eligible_peers
    }

    /// Creates and inserts a set of dialed peers into the eligible peers
    /// set, and returns the set of dialed peer IDs.
    fn insert_dialed_peers(
        num_dialed_peers: usize,
        eligible_peers: &mut Vec<(PeerId, DiscoveredPeer)>,
    ) -> HashSet<PeerId> {
        let mut dialed_peers = hashset![];
        for _ in 0..num_dialed_peers {
            // Create a dialed peer
            let peer_id = AccountAddress::random();
            let mut peer = DiscoveredPeer::new(PeerRole::PreferredUpstream);
            dialed_peers.insert(peer_id);

            // Set the last dial time to be recent
            peer.update_last_dial_time();

            // Add the peer to the eligible peers
            eligible_peers.push((peer_id, peer));
        }
        dialed_peers
    }

    /// Creates and inserts a set of non-dialed peers into the eligible peers
    /// set, and returns the set of non-dialed peer IDs.
    fn insert_non_dialed_peers(
        num_non_dialed_peers: usize,
        eligible_peers: &mut Vec<(PeerId, DiscoveredPeer)>,
    ) -> HashSet<PeerId> {
        let mut non_dialed_peers = hashset![];
        for _ in 0..num_non_dialed_peers {
            // Create a non-dialed peer
            let peer_id = AccountAddress::random();
            non_dialed_peers.insert(peer_id);

            // Add the peer to the eligible peers
            eligible_peers.push((peer_id, DiscoveredPeer::new(PeerRole::ValidatorFullNode)));
        }
        non_dialed_peers
    }

    /// Verifies the top 10% of selected peers are the lowest latency peers
    fn verify_highest_peer_selection_latencies(
        discovered_peers: Arc<RwLock<DiscoveredPeerSet>>,
        peers_and_selection_counts: &HashMap<PeerId, u64>,
    ) {
        // Build a max-heap of all peers by their selection counts
        let mut max_heap_selection_counts = BinaryHeap::new();
        for (peer, selection_count) in peers_and_selection_counts.clone() {
            max_heap_selection_counts.push((selection_count, peer));
        }

        // Verify the top 10% of polled peers are the lowest latency peers
        let peers_to_verify = peers_and_selection_counts.len() / 10;
        let mut highest_seen_latency = 0.0;
        for _ in 0..peers_to_verify {
            // Get the peer
            let (_, peer) = max_heap_selection_counts.pop().unwrap();

            // Get the peer's ping latency
            let discovered_peers = discovered_peers.read();
            let discovered_peer = discovered_peers.peer_set.get(&peer).unwrap();
            let ping_latency = discovered_peer.ping_latency_secs.unwrap();

            // Verify that the ping latencies are increasing
            if ping_latency <= highest_seen_latency {
                // The ping latencies did not increase. This should only be
                // possible if the latencies are very close (i.e., within 10%).
                if (highest_seen_latency - ping_latency) > 0.1 {
                    panic!("The ping latencies are not increasing! Are peers weighted by latency?");
                }
            }

            // Update the highest seen latency
            highest_seen_latency = ping_latency;
        }
    }
}
