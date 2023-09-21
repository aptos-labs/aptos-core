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
