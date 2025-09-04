// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::Error,
    logging::{LogEntry, LogEvent, LogSchema},
};
use velor_config::{config::VelorDataClientConfig, network_id::PeerNetworkId};
use velor_logger::{sample, sample::SampleRate, warn};
use velor_network::application::{metadata::PeerMetadata, storage::PeersAndMetadata};
use maplit::hashset;
use ordered_float::OrderedFloat;
use rand::seq::{IteratorRandom, SliceRandom};
use std::{
    collections::{BTreeMap, HashSet},
    sync::Arc,
    time::Duration,
};

// Useful constants
const ERROR_LOG_FREQ_SECS: u64 = 3;

/// Chooses peers weighted by distance from the validator set
/// and latency. We prioritize distance over latency as we want
/// to avoid close but not up-to-date peers.
pub fn choose_random_peers_by_distance_and_latency(
    peers: HashSet<PeerNetworkId>,
    peers_and_metadata: Arc<PeersAndMetadata>,
    num_peers_to_choose: usize,
) -> HashSet<PeerNetworkId> {
    // Group peers and latency weights by validator distance, i.e., distance -> [(peer, latency weight)]
    let mut peers_and_latencies_by_distance = BTreeMap::new();
    for peer in peers {
        if let Some((distance, latency)) =
            get_distance_and_latency_for_peer(&peers_and_metadata, peer)
        {
            let latency_weight = convert_latency_to_weight(latency);
            peers_and_latencies_by_distance
                .entry(distance)
                .or_insert_with(Vec::new)
                .push((peer, latency_weight));
        }
    }

    // Select the peers by distance and latency weights. Note: BTreeMaps are
    // sorted by key, so the entries will be sorted by distance in ascending order.
    let mut selected_peers = HashSet::new();
    for (_, peers_and_latencies) in peers_and_latencies_by_distance {
        // Select the peers by latency weights
        let num_peers_remaining = num_peers_to_choose.saturating_sub(selected_peers.len()) as u64;
        let peers = choose_random_peers_by_weight(num_peers_remaining, peers_and_latencies);

        // Add the peers to the entire set
        selected_peers.extend(peers);

        // If we have selected enough peers, return early
        if selected_peers.len() >= num_peers_to_choose {
            return selected_peers;
        }
    }

    // Return the selected peers
    selected_peers
}

/// Selects the specified number of peers from the list of potential
/// peers. Peer selection is weighted by peer latencies (i.e., the
/// lower the latency, the higher the probability of selection).
///
/// If `ignore_high_latency_peers` is true, the list of potential peers
/// may be filtered to only include a subset of peers with lower latencies.
/// This helps to avoid sub-optimal peer selection and bad tail behaviours.
pub fn choose_peers_by_latency(
    data_client_config: Arc<VelorDataClientConfig>,
    num_peers_to_choose: u64,
    potential_peers: HashSet<PeerNetworkId>,
    peers_and_metadata: Arc<PeersAndMetadata>,
    ignore_high_latency_peers: bool,
) -> HashSet<PeerNetworkId> {
    // If no peers can be chosen, return an empty set
    if num_peers_to_choose == 0 || potential_peers.is_empty() {
        return hashset![];
    }

    // Gather the latency weights for all potential peers
    let mut potential_peers_and_latency_weights = vec![];
    for peer in potential_peers {
        if let Some(latency) = get_latency_for_peer(&peers_and_metadata, peer) {
            let latency_weight = convert_latency_to_weight(latency);
            potential_peers_and_latency_weights.push((peer, OrderedFloat(latency_weight)));
        }
    }

    // Determine the number of peers to consider. If high latency peers can be
    // ignored, we only want to consider a subset of peers with the lowest
    // latencies. However, this can only be done if we have a large total
    // number of peers, and there are enough potential peers for each request.
    let mut num_peers_to_consider = potential_peers_and_latency_weights.len() as u64;
    if ignore_high_latency_peers {
        let latency_filtering_config = &data_client_config.latency_filtering_config;
        let peer_ratio_per_request = num_peers_to_consider / num_peers_to_choose;
        if num_peers_to_consider >= latency_filtering_config.min_peers_for_latency_filtering
            && peer_ratio_per_request
                >= latency_filtering_config.min_peer_ratio_for_latency_filtering
        {
            // Consider a subset of peers with the lowest latencies
            num_peers_to_consider /= latency_filtering_config.latency_filtering_reduction_factor
        }
    }

    // Sort the peers by latency weights and take the number of peers to consider
    potential_peers_and_latency_weights.sort_by_key(|(_, latency_weight)| *latency_weight);
    let potential_peers_and_latency_weights = potential_peers_and_latency_weights
        .into_iter()
        .take(num_peers_to_consider as usize)
        .map(|(peer, latency_weight)| (peer, latency_weight.into_inner()))
        .collect::<Vec<_>>();

    // Select the peers by latency weights
    choose_random_peers_by_weight(num_peers_to_choose, potential_peers_and_latency_weights)
}

/// Selects a single peer randomly from the list of specified peers
pub fn choose_random_peer(peers: HashSet<PeerNetworkId>) -> Option<PeerNetworkId> {
    peers.into_iter().choose(&mut rand::thread_rng())
}

/// Selects a set of peers randomly from the list of specified peers
pub fn choose_random_peers(
    num_peers_to_choose: usize,
    peers: HashSet<PeerNetworkId>,
) -> HashSet<PeerNetworkId> {
    let random_peers = peers
        .into_iter()
        .choose_multiple(&mut rand::thread_rng(), num_peers_to_choose);
    random_peers.into_iter().collect()
}

/// Selects a set of peers randomly from the list of specified peers,
/// weighted by the peer's weight. If an error is encountered, it is
/// logged and an empty set is returned.
pub fn choose_random_peers_by_weight(
    num_peers_to_choose: u64,
    peers_and_weights: Vec<(PeerNetworkId, f64)>,
) -> HashSet<PeerNetworkId> {
    // Get the random peers by weight
    let random_peers_by_weight = peers_and_weights
        .choose_multiple_weighted(
            &mut rand::thread_rng(),
            num_peers_to_choose as usize,
            |peer| peer.1,
        )
        .map(|peers| peers.into_iter().map(|peer| peer.0).collect())
        .map_err(|error| Error::UnexpectedErrorEncountered(error.to_string()));

    // Return the random peers by weight (or an empty set if an error was encountered)
    random_peers_by_weight.unwrap_or_else(|error| {
        // Log the error
        log_warning_with_sample(
            LogSchema::new(LogEntry::PeerStates)
                .event(LogEvent::PeerSelectionError)
                .message(&format!(
                    "Unable to select peer by latencies! Error: {:?}",
                    error
                )),
        );

        // No peer was selected
        hashset![]
    })
}

/// Converts the given latency measurement to a weight.
/// The lower the latency, the higher the weight.
fn convert_latency_to_weight(latency: f64) -> f64 {
    // If the latency is <= 0, something has gone wrong, so return 0.
    if latency <= 0.0 {
        return 0.0;
    }

    // Otherwise, invert the latency to get the weight
    1000.0 / latency
}

/// If the number of selected peers is less than the number of required peers,
/// select remaining peers from the serviceable peers (at random).
pub fn extend_with_random_peers(
    mut selected_peers: HashSet<PeerNetworkId>,
    serviceable_peers: HashSet<PeerNetworkId>,
    num_required_peers: usize,
) -> HashSet<PeerNetworkId> {
    if selected_peers.len() < num_required_peers {
        // Randomly select the remaining peers
        let num_remaining_peers = num_required_peers.saturating_sub(selected_peers.len());
        let remaining_serviceable_peers = serviceable_peers
            .difference(&selected_peers)
            .cloned()
            .collect();
        let remaining_peers = choose_random_peers(num_remaining_peers, remaining_serviceable_peers);

        // Add the remaining peers to the selected peers
        selected_peers.extend(remaining_peers);
    }

    // Return the selected peers
    selected_peers
}

/// Gets the latency for the specified peer from the peer monitoring metadata
fn get_latency_for_peer(
    peers_and_metadata: &Arc<PeersAndMetadata>,
    peer: PeerNetworkId,
) -> Option<f64> {
    if let Some(peer_metadata) = get_metadata_for_peer(peers_and_metadata, peer) {
        let peer_monitoring_metadata = peer_metadata.get_peer_monitoring_metadata();
        if let Some(latency) = peer_monitoring_metadata.average_ping_latency_secs {
            return Some(latency); // The latency was found
        }
    }

    // Otherwise, no latency was found
    log_warning_with_sample(
        LogSchema::new(LogEntry::PeerStates)
            .event(LogEvent::PeerSelectionError)
            .message(&format!("Unable to get latency for peer! Peer: {:?}", peer)),
    );
    None
}

/// Gets the distance from the validators and measured latency (for the specified peer)
fn get_distance_and_latency_for_peer(
    peers_and_metadata: &Arc<PeersAndMetadata>,
    peer: PeerNetworkId,
) -> Option<(u64, f64)> {
    if let Some(peer_metadata) = get_metadata_for_peer(peers_and_metadata, peer) {
        // Get the distance and latency for the peer
        let peer_monitoring_metadata = peer_metadata.get_peer_monitoring_metadata();
        let distance = peer_monitoring_metadata
            .latest_network_info_response
            .as_ref()
            .map(|response| response.distance_from_validators);
        let latency = peer_monitoring_metadata.average_ping_latency_secs;

        // Return the distance and latency if both were found
        if let (Some(distance), Some(latency)) = (distance, latency) {
            return Some((distance, latency));
        }
    }

    // Otherwise, no distance and latency was found
    log_warning_with_sample(
        LogSchema::new(LogEntry::PeerStates)
            .event(LogEvent::PeerSelectionError)
            .message(&format!(
                "Unable to get distance and latency for peer! Peer: {:?}",
                peer
            )),
    );
    None
}

/// Returns the metadata for the specified peer. If no metadata
/// is found, an error is logged and None is returned.
pub fn get_metadata_for_peer(
    peers_and_metadata: &Arc<PeersAndMetadata>,
    peer: PeerNetworkId,
) -> Option<PeerMetadata> {
    match peers_and_metadata.get_metadata_for_peer(peer) {
        Ok(peer_metadata) => Some(peer_metadata),
        Err(error) => {
            log_warning_with_sample(
                LogSchema::new(LogEntry::PeerStates)
                    .event(LogEvent::PeerSelectionError)
                    .message(&format!(
                        "Unable to get peer metadata! Peer: {:?}, Error: {:?}",
                        peer, error
                    )),
            );
            None // No metadata was found
        },
    }
}

/// Logs the given schema as a warning with a sampled frequency
fn log_warning_with_sample(log: LogSchema) {
    sample!(
        SampleRate::Duration(Duration::from_secs(ERROR_LOG_FREQ_SECS)),
        warn!(log);
    );
}

#[cfg(test)]
mod tests {
    use crate::utils::{choose_random_peer, choose_random_peers, choose_random_peers_by_weight};
    use velor_config::network_id::{NetworkId, PeerNetworkId};
    use velor_types::PeerId;
    use maplit::hashset;
    use std::collections::{HashMap, HashSet};

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
        let chosen_peers = choose_random_peers_by_weight(1, vec![]);
        assert!(chosen_peers.is_empty());

        // Choose 1 peer from a list of length 1, and verify the peer is returned
        let peer = create_random_peer_network_id();
        let chosen_peers = choose_random_peers_by_weight(1, vec![(peer, 1.0)]);
        assert_eq!(chosen_peers, hashset![peer]);

        // Choose 2 peers from a list of length 2, and verify the peers are returned
        let peer_1 = create_random_peer_network_id();
        let peer_2 = create_random_peer_network_id();
        let chosen_peers = choose_random_peers_by_weight(2, vec![(peer_1, 1.0), (peer_2, 1.0)]);
        assert_eq!(chosen_peers, hashset![peer_1, peer_2]);

        // Choose 5 peers from a list of length 2, and verify the peers are returned
        let peer_1 = create_random_peer_network_id();
        let peer_2 = create_random_peer_network_id();
        let chosen_peers = choose_random_peers_by_weight(5, vec![(peer_1, 1.0), (peer_2, 1.0)]);
        assert_eq!(chosen_peers, hashset![peer_1, peer_2]);

        // Choose 5 peers from a list of length 10, and verify only 5 are returned
        let peers_and_weights = (0..10)
            .map(|_| (create_random_peer_network_id(), 1.0))
            .collect::<Vec<_>>();
        let chosen_peers = choose_random_peers_by_weight(5, peers_and_weights);
        assert_eq!(chosen_peers.len(), 5);

        // Choose 0 peers from a list of length 10, and verify an empty set is returned
        let peers_and_weights = (0..10)
            .map(|_| (create_random_peer_network_id(), 1.0))
            .collect::<Vec<_>>();
        let chosen_peers = choose_random_peers_by_weight(0, peers_and_weights);
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
            let chosen_peers = choose_random_peers_by_weight(1, peers_and_weights.clone());
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
