// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    connectivity_manager::{DiscoveredPeer, DiscoveredPeerSet},
    logging::NetworkSchema,
};
use aptos_config::network_id::NetworkContext;
use aptos_infallible::RwLock;
use aptos_logger::error;
use aptos_types::PeerId;
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
        if let Some(ping_latency_ms) = discovered_peers.read().get_ping_latency_ms(peer_id) {
            let latency_weight = convert_latency_to_weight(ping_latency_ms);
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
fn convert_latency_to_weight(latency_ms: u64) -> f64 {
    // If the latency is <= 0, something has gone wrong, so return 0.
    let latency_ms = latency_ms as f64;
    if latency_ms <= 0.0 {
        return 0.0;
    }

    // Invert the latency to get the weight
    let mut weight = 1000.0 / latency_ms;

    // For every 25ms of latency, reduce the weight by 1/2 (to
    // ensure that low latency peers are highly weighted)
    let num_reductions = (latency_ms / 25.0) as usize;
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
