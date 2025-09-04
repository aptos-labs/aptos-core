// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::types::MempoolSenderBucket;
use crate::{counters, network::BroadcastPeerPriority};
use velor_config::{
    config::{MempoolConfig, NodeType},
    network_id::{NetworkId, PeerNetworkId},
};
use velor_infallible::RwLock;
use velor_logger::prelude::*;
use velor_peer_monitoring_service_types::PeerMonitoringMetadata;
use velor_time_service::{TimeService, TimeServiceTrait};
use itertools::Itertools;
use std::{
    cmp::{max, min, Ordering},
    collections::{hash_map::RandomState, HashMap},
    hash::{BuildHasher, Hasher},
    sync::Arc,
    time::Instant,
};

/// A simple struct that offers comparisons and ordering for peer prioritization
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

    /// Provides simple ordering for peers when forwarding transactions.
    /// Higher priority peers are greater than lower priority peers.
    fn compare_simple(
        &self,
        peer_a: &(PeerNetworkId, Option<&PeerMonitoringMetadata>),
        peer_b: &(PeerNetworkId, Option<&PeerMonitoringMetadata>),
    ) -> Ordering {
        // Deconstruct the peer tuples
        let (peer_network_id_a, _) = peer_a;
        let (peer_network_id_b, _) = peer_b;

        // First, compare by network ID (i.e., Validator > VFN > Public)
        let network_ordering = compare_network_id(
            &peer_network_id_a.network_id(),
            &peer_network_id_b.network_id(),
        );
        if !network_ordering.is_eq() {
            return network_ordering; // Only return if it's not equal
        }

        // Otherwise, simply hash the peer IDs and compare the hashes
        self.compare_hash(peer_network_id_a, peer_network_id_b)
    }

    /// Provides intelligent ordering for peers when forwarding transactions.
    /// Higher priority peers are greater than lower priority peers.
    fn compare_intelligent(
        &self,
        peer_a: &(PeerNetworkId, Option<&PeerMonitoringMetadata>),
        peer_b: &(PeerNetworkId, Option<&PeerMonitoringMetadata>),
    ) -> Ordering {
        // Deconstruct the peer tuples
        let (peer_network_id_a, monitoring_metadata_a) = peer_a;
        let (peer_network_id_b, monitoring_metadata_b) = peer_b;

        // First, compare by network ID (i.e., Validator > VFN > Public)
        let network_ordering = compare_network_id(
            &peer_network_id_a.network_id(),
            &peer_network_id_b.network_id(),
        );
        if !network_ordering.is_eq() {
            return network_ordering; // Only return if it's not equal
        }

        // Otherwise, compare by peer distance from the validators.
        // This avoids badly configured/connected peers (e.g., broken VN-VFN connections).
        let distance_ordering =
            compare_validator_distance(monitoring_metadata_a, monitoring_metadata_b);
        if !distance_ordering.is_eq() {
            return distance_ordering; // Only return if it's not equal
        }

        // Otherwise, compare by peer ping latency (the lower the better)
        let latency_ordering = compare_ping_latency(monitoring_metadata_a, monitoring_metadata_b);
        if !latency_ordering.is_eq() {
            return latency_ordering; // Only return if it's not equal
        }

        // Otherwise, simply hash the peer IDs and compare the hashes.
        // In practice, this should be relatively rare.
        self.compare_hash(peer_network_id_a, peer_network_id_b)
    }

    /// Compares the hash of the given peer IDs
    fn compare_hash(
        &self,
        peer_network_id_a: &PeerNetworkId,
        peer_network_id_b: &PeerNetworkId,
    ) -> Ordering {
        let hash_a = self.hash_peer_id(peer_network_id_a);
        let hash_b = self.hash_peer_id(peer_network_id_b);
        hash_a.cmp(&hash_b)
    }

    /// Stable within a mempool instance but random between instances
    fn hash_peer_id(&self, peer_network_id: &PeerNetworkId) -> u64 {
        let mut hasher = self.random_state.build_hasher();
        hasher.write(peer_network_id.peer_id().as_ref());
        hasher.finish()
    }
}

/// A simple struct to hold state for peer prioritization
#[derive(Clone, Debug)]
pub struct PrioritizedPeersState {
    // The current mempool configuration
    mempool_config: MempoolConfig,

    // The current list of prioritized peers
    prioritized_peers: Arc<RwLock<Vec<PeerNetworkId>>>,

    // We divide mempool transactions into buckets based on hash of the sender.
    // For load balancing, we send transactions from a subset of buckets to a peer.
    // This map stores the buckets that are sent to a peer and the priority of the peer
    // for that bucket.
    peer_to_sender_buckets:
        HashMap<PeerNetworkId, HashMap<MempoolSenderBucket, BroadcastPeerPriority>>,

    // The comparator used to prioritize peers
    peer_comparator: PrioritizedPeersComparator,

    // Whether ping latencies were observed for all peers
    observed_all_ping_latencies: bool,

    // The last time peer priorities were updated
    last_peer_priority_update: Option<Instant>,

    // The time service used to fetch timestamps
    time_service: TimeService,

    // The type of node (Validator, ValidatorFullNode, PublicFullnode)
    node_type: NodeType,
}

impl PrioritizedPeersState {
    pub fn new(
        mempool_config: MempoolConfig,
        node_type: NodeType,
        time_service: TimeService,
    ) -> Self {
        Self {
            mempool_config,
            prioritized_peers: Arc::new(RwLock::new(Vec::new())),
            peer_comparator: PrioritizedPeersComparator::new(),
            observed_all_ping_latencies: false,
            last_peer_priority_update: None,
            time_service,
            peer_to_sender_buckets: HashMap::new(),
            node_type,
        }
    }

    /// Returns the priority of the given peer. The lower the
    /// value, the higher the priority.
    pub fn get_peer_priority(&self, peer_network_id: &PeerNetworkId) -> usize {
        let prioritized_peers = self.prioritized_peers.read();
        prioritized_peers
            .iter()
            .find_position(|peer| *peer == peer_network_id)
            .map_or(usize::MAX, |(position, _)| position)
    }

    pub fn get_sender_bucket_priority_for_peer(
        &self,
        peer: &PeerNetworkId,
        sender_bucket: MempoolSenderBucket,
    ) -> Option<BroadcastPeerPriority> {
        self.peer_to_sender_buckets
            .get(peer)
            .and_then(|buckets| buckets.get(&sender_bucket).cloned())
    }

    /// Returns true iff the prioritized peers list is ready for another update
    pub fn ready_for_update(&self, peers_changed: bool) -> bool {
        // If intelligent peer prioritization is disabled, we should only
        // update the prioritized peers if the peers have changed.
        if !self.mempool_config.enable_intelligent_peer_prioritization {
            return peers_changed;
        }

        // Otherwise, we should update the prioritized peers if the peers have changed
        // or if we haven't observed ping latencies for all peers yet. This is useful
        // because latencies are only populated some time after the peer connects, so
        // we should continuously reprioritize until latencies are observed for all peers.
        if peers_changed || !self.observed_all_ping_latencies {
            return true;
        }

        // Otherwise, we should only update if enough time has passed since the last update
        match self.last_peer_priority_update {
            None => true, // We haven't updated yet
            Some(last_update) => {
                let duration_since_update = self.time_service.now().duration_since(last_update);
                let update_interval_secs = self
                    .mempool_config
                    .shared_mempool_priority_update_interval_secs;
                duration_since_update.as_secs() > update_interval_secs
            },
        }
    }

    pub(crate) fn get_sender_buckets_for_peer(
        &self,
        peer: &PeerNetworkId,
    ) -> Option<&HashMap<MempoolSenderBucket, BroadcastPeerPriority>> {
        self.peer_to_sender_buckets.get(peer)
    }

    /// Sorts the given peers by priority using the prioritized peer comparator.
    /// The peers are sorted in descending order (i.e., higher values are prioritized).
    fn sort_peers_by_priority(
        &self,
        peers_and_metadata: &[(PeerNetworkId, Option<&PeerMonitoringMetadata>)],
    ) -> Vec<PeerNetworkId> {
        peers_and_metadata
            .iter()
            .sorted_by(|peer_a, peer_b| {
                // Only use intelligent peer prioritization if it is enabled
                let ordering = if self.mempool_config.enable_intelligent_peer_prioritization {
                    self.peer_comparator.compare_intelligent(peer_a, peer_b)
                } else {
                    self.peer_comparator.compare_simple(peer_a, peer_b)
                };
                ordering.reverse() // Prioritize higher values (i.e., sorted by descending order)
            })
            .map(|(peer, _)| *peer)
            .collect()
    }

    fn update_sender_bucket_for_peers(
        &mut self,
        peer_monitoring_data: &HashMap<PeerNetworkId, Option<&PeerMonitoringMetadata>>,
        num_mempool_txns_received_since_peers_updated: u64,
        num_committed_txns_received_since_peers_updated: u64,
    ) {
        // TODO: If the top peer set didn't change, then don't change the Primary sender bucket assignment.
        // TODO: (Minor) If the load is low, don't do load balancing for Failover buckets.
        assert!(self.prioritized_peers.read().len() == peer_monitoring_data.len());

        // Obtain the top peers to assign the sender buckets with Primary priority
        let mut top_peers = vec![];
        let secs_elapsed_since_last_update =
            self.last_peer_priority_update.map_or(0, |last_update| {
                self.time_service
                    .now()
                    .duration_since(last_update)
                    .as_secs()
            });

        // When the node is in state sync mode, it will receive more mempool commit notifications than the actual
        // commits that happens on the blockchain during the same time period. As secs_elapsed_since_last_update is
        // local time and not the on chain time, the average_committed_traffic_observed is only a local estimate of
        // the traffic and could differ from the actual traffic observed by the blockchain. If the estimate differs
        // from the actual traffic observed on the blockchain, we could end up load balancing more or less than required.
        let average_mempool_traffic_observed = num_mempool_txns_received_since_peers_updated as f64
            / max(1, secs_elapsed_since_last_update) as f64;
        let average_committed_traffic_observed = num_committed_txns_received_since_peers_updated
            as f64
            / max(1, secs_elapsed_since_last_update) as f64;

        // Obtain the highest threshold from mempool_config.load_balancing_thresholds for which avg_mempool_traffic_threshold_in_tps exceeds average_mempool_traffic_observed
        let threshold_config = self
            .mempool_config
            .load_balancing_thresholds
            .clone()
            .into_iter()
            .rev()
            .find(|threshold_config| {
                threshold_config.avg_mempool_traffic_threshold_in_tps
                    <= max(
                        average_mempool_traffic_observed as u64,
                        average_committed_traffic_observed as u64,
                    )
            })
            .unwrap_or_default();

        let num_top_peers = max(
            1,
            min(
                self.mempool_config.num_sender_buckets,
                if self.mempool_config.enable_max_load_balancing_at_any_load {
                    u8::MAX
                } else {
                    threshold_config.max_number_of_upstream_peers
                },
            ),
        );
        info!(
            "Time elapsed since last peer update: {:?}\n
            Number of mempool transactions received since last peer update: {:?},\n
            Average mempool traffic observed: {:?},\n
            Number of committed transactions received since last peer update: {:?},\n
            Average committed traffic observed: {:?},\n
            Load balancing threshold config: {:?},\n
            Number of top peers picked: {:?}",
            secs_elapsed_since_last_update,
            num_mempool_txns_received_since_peers_updated,
            average_mempool_traffic_observed,
            num_committed_txns_received_since_peers_updated,
            average_committed_traffic_observed,
            threshold_config,
            num_top_peers
        );

        if self.node_type.is_validator_fullnode() {
            // Use the peer on the VFN network with lowest ping latency as the primary peer
            let peers_in_vfn_network = self
                .prioritized_peers
                .read()
                .iter()
                .cloned()
                .filter(|peer| peer.network_id() == NetworkId::Vfn)
                .collect::<Vec<_>>();

            if !peers_in_vfn_network.is_empty() {
                top_peers = vec![peers_in_vfn_network[0]];
            }
        }

        if top_peers.is_empty() {
            let base_ping_latency = self.prioritized_peers.read().first().and_then(|peer| {
                peer_monitoring_data
                    .get(peer)
                    .and_then(|metadata| get_peer_ping_latency(metadata))
            });

            // Extract top peers with ping latency less than base_ping_latency + 50 ms
            for peer in self.prioritized_peers.read().iter() {
                if top_peers.len() >= num_top_peers as usize {
                    break;
                }

                let ping_latency = peer_monitoring_data
                    .get(peer)
                    .and_then(|metadata| get_peer_ping_latency(metadata));

                if base_ping_latency.is_none()
                    || ping_latency.is_none()
                    || ping_latency.unwrap()
                        < base_ping_latency.unwrap()
                            + (threshold_config.latency_slack_between_top_upstream_peers as f64)
                                / 1000.0
                {
                    top_peers.push(*peer);
                }
            }
        }
        info!(
            "Identified top peers: {:?}, node_type: {:?}",
            top_peers, self.node_type
        );

        assert!(top_peers.len() <= num_top_peers as usize);
        // Top peers shouldn't be empty if prioritized_peers is not zero
        assert!(self.prioritized_peers.read().is_empty() || !top_peers.is_empty());

        self.peer_to_sender_buckets = HashMap::new();
        if !self.prioritized_peers.read().is_empty() {
            // Assign sender buckets with Primary priority
            let mut peer_index = 0;
            for bucket_index in 0..self.mempool_config.num_sender_buckets {
                self.peer_to_sender_buckets
                    .entry(*top_peers.get(peer_index).unwrap())
                    .or_default()
                    .insert(bucket_index, BroadcastPeerPriority::Primary);
                peer_index = (peer_index + 1) % top_peers.len();
            }

            // Assign sender buckets with Failover priority. Use Round Robin.
            peer_index = 0;
            let num_prioritized_peers = self.prioritized_peers.read().len();
            for _ in 0..self.mempool_config.default_failovers {
                for bucket_index in 0..self.mempool_config.num_sender_buckets {
                    // Find the first peer that already doesn't have the sender bucket, and add the bucket
                    for _ in 0..num_prioritized_peers {
                        let peer = self.prioritized_peers.read()[peer_index];
                        let sender_bucket_list =
                            self.peer_to_sender_buckets.entry(peer).or_default();
                        if let std::collections::hash_map::Entry::Vacant(e) =
                            sender_bucket_list.entry(bucket_index)
                        {
                            e.insert(BroadcastPeerPriority::Failover);
                            break;
                        }
                        peer_index = (peer_index + 1) % num_prioritized_peers;
                    }
                }
            }
        }
    }

    /// Updates the prioritized peers list
    pub fn update_prioritized_peers(
        &mut self,
        peers_and_metadata: Vec<(PeerNetworkId, Option<&PeerMonitoringMetadata>)>,
        num_mempool_txns_received_since_peers_updated: u64,
        num_committed_txns_received_since_peers_updated: u64,
    ) {
        let peer_monitoring_data: HashMap<PeerNetworkId, Option<&PeerMonitoringMetadata>> =
            peers_and_metadata.clone().into_iter().collect();

        // Calculate the new set of prioritized peers
        let new_prioritized_peers = self.sort_peers_by_priority(&peers_and_metadata);

        // Update the prioritized peer metrics
        self.update_prioritized_peer_metrics(&new_prioritized_peers);

        // Update the prioritized peers
        *self.prioritized_peers.write() = new_prioritized_peers;

        // Check if we've now observed ping latencies for all peers
        if !self.observed_all_ping_latencies {
            self.observed_all_ping_latencies = peers_and_metadata
                .iter()
                .all(|(_, metadata)| get_peer_ping_latency(metadata).is_some());
        }

        // Divide the sender buckets amongst the top peers
        self.update_sender_bucket_for_peers(
            &peer_monitoring_data,
            num_mempool_txns_received_since_peers_updated,
            num_committed_txns_received_since_peers_updated,
        );

        // Set the last peer priority update time
        self.last_peer_priority_update = Some(self.time_service.now());
        info!(
            "Updated prioritized peers. Peer count: {:?}, Latencies: {:?},\n Prioritized peers: {:?},\n Sender bucket assignment: {:?}",
            peers_and_metadata.len(),
            peers_and_metadata
                .iter()
                .map(|(peer, metadata)| (
                    peer,
                    metadata.map(|metadata| metadata.average_ping_latency_secs)
                ))
                .collect::<Vec<_>>(),
            self.prioritized_peers.read(),
            self.peer_to_sender_buckets,
        );
    }

    /// Updates the prioritized peer metrics based on the new prioritization
    fn update_prioritized_peer_metrics(&mut self, new_prioritized_peers: &Vec<PeerNetworkId>) {
        // Calculate the number of peers that changed priorities
        let current_prioritized_peers = self.prioritized_peers.read();
        let num_peers_changed = new_prioritized_peers
            .iter()
            .zip(current_prioritized_peers.iter())
            .filter(|(new_peer, old_peer)| new_peer != old_peer)
            .count();

        // Log the number of peers that changed priorities
        info!(
            "Number of peers that changed mempool priorities: {}. New priority list: {:?}",
            num_peers_changed, new_prioritized_peers
        );

        // Update the metrics for the number of peers that changed priorities
        counters::shared_mempool_priority_change_count(num_peers_changed as i64);
    }
}

/// Returns the distance from the validators for the
/// given monitoring metadata (if one exists).
fn get_distance_from_validators(
    monitoring_metadata: &Option<&PeerMonitoringMetadata>,
) -> Option<u64> {
    monitoring_metadata.and_then(|metadata| {
        metadata
            .latest_network_info_response
            .as_ref()
            .map(|network_info_response| network_info_response.distance_from_validators)
    })
}

/// Returns the ping latency for the given monitoring
/// metadata (if one exists).
fn get_peer_ping_latency(monitoring_metadata: &Option<&PeerMonitoringMetadata>) -> Option<f64> {
    monitoring_metadata.and_then(|metadata| metadata.average_ping_latency_secs)
}

/// Compares the network ID for the given pair of peers.
/// The peer with the highest network is prioritized.
fn compare_network_id(network_id_a: &NetworkId, network_id_b: &NetworkId) -> Ordering {
    // We need to reverse the default ordering to ensure that: Validator > VFN > Public
    network_id_a.cmp(network_id_b).reverse()
}

/// Compares the ping latency for the given pair of monitoring metadata.
/// The peer with the lowest ping latency is prioritized.
fn compare_ping_latency(
    monitoring_metadata_a: &Option<&PeerMonitoringMetadata>,
    monitoring_metadata_b: &Option<&PeerMonitoringMetadata>,
) -> Ordering {
    // Get the ping latency from the monitoring metadata
    let ping_latency_a = get_peer_ping_latency(monitoring_metadata_a);
    let ping_latency_b = get_peer_ping_latency(monitoring_metadata_b);

    // Compare the ping latencies
    match (ping_latency_a, ping_latency_b) {
        (Some(ping_latency_a), Some(ping_latency_b)) => {
            // Prioritize the peer with the lowest ping latency
            ping_latency_a.total_cmp(&ping_latency_b).reverse()
        },
        (Some(_), None) => {
            Ordering::Greater // Prioritize the peer with a ping latency
        },
        (None, Some(_)) => {
            Ordering::Less // Prioritize the peer with a ping latency
        },
        (None, None) => {
            Ordering::Equal // Neither peer has a ping latency
        },
    }
}

/// Compares the validator distance for the given pair of monitoring metadata.
/// The peer with the lowest validator distance is prioritized.
fn compare_validator_distance(
    monitoring_metadata_a: &Option<&PeerMonitoringMetadata>,
    monitoring_metadata_b: &Option<&PeerMonitoringMetadata>,
) -> Ordering {
    // Get the validator distance from the monitoring metadata
    let validator_distance_a = get_distance_from_validators(monitoring_metadata_a);
    let validator_distance_b = get_distance_from_validators(monitoring_metadata_b);

    // Compare the distances
    match (validator_distance_a, validator_distance_b) {
        (Some(validator_distance_a), Some(validator_distance_b)) => {
            // Prioritize the peer with the lowest validator distance
            validator_distance_a.cmp(&validator_distance_b).reverse()
        },
        (Some(_), None) => {
            Ordering::Greater // Prioritize the peer with a validator distance
        },
        (None, Some(_)) => {
            Ordering::Less // Prioritize the peer with a validator distance
        },
        (None, None) => {
            Ordering::Equal // Neither peer has a validator distance
        },
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use velor_config::{
        config::{MempoolConfig, NodeType},
        network_id::{NetworkId, PeerNetworkId},
    };
    use velor_peer_monitoring_service_types::{
        response::NetworkInformationResponse, PeerMonitoringMetadata,
    };
    use velor_types::PeerId;
    use core::cmp::Ordering;
    use std::collections::BTreeMap;

    #[test]
    fn test_compare_network_id() {
        // Create different network types
        let validator_network = NetworkId::Validator;
        let vfn_network = NetworkId::Vfn;
        let public_network = NetworkId::Public;

        // Compare the validator and VFN networks
        assert_eq!(
            Ordering::Greater,
            compare_network_id(&validator_network, &vfn_network)
        );

        // Compare the VFN and public networks
        assert_eq!(
            Ordering::Greater,
            compare_network_id(&validator_network, &public_network)
        );

        // Compare the validator and public networks
        assert_eq!(
            Ordering::Greater,
            compare_network_id(&vfn_network, &public_network)
        );
    }

    #[test]
    fn test_compare_validator_distance() {
        // Create monitoring metadata with the same distance
        let monitoring_metadata_1 = create_metadata_with_distance(Some(1));
        let monitoring_metadata_2 = create_metadata_with_distance(Some(1));

        // Verify that the metadata is equal
        assert_eq!(
            Ordering::Equal,
            compare_validator_distance(
                &Some(&monitoring_metadata_1),
                &Some(&monitoring_metadata_2)
            )
        );

        // Create monitoring metadata with different distances
        let monitoring_metadata_1 = create_metadata_with_distance(Some(0));
        let monitoring_metadata_2 = create_metadata_with_distance(Some(4));

        // Verify that the metadata has different ordering
        assert_eq!(
            Ordering::Greater,
            compare_validator_distance(
                &Some(&monitoring_metadata_1),
                &Some(&monitoring_metadata_2)
            )
        );
        assert_eq!(
            Ordering::Less,
            compare_validator_distance(
                &Some(&monitoring_metadata_2),
                &Some(&monitoring_metadata_1)
            )
        );

        // Create monitoring metadata with and without distances
        let monitoring_metadata_1 = create_metadata_with_distance(Some(0));
        let monitoring_metadata_2 = create_metadata_with_distance(None);

        // Verify that the metadata with a distance has a higher ordering
        assert_eq!(
            Ordering::Greater,
            compare_validator_distance(
                &Some(&monitoring_metadata_1),
                &Some(&monitoring_metadata_2)
            )
        );
        assert_eq!(
            Ordering::Less,
            compare_validator_distance(
                &Some(&monitoring_metadata_2),
                &Some(&monitoring_metadata_1)
            )
        );

        // Compare monitoring metadata that is missing entirely
        assert_eq!(
            Ordering::Greater,
            compare_validator_distance(&Some(&monitoring_metadata_1), &None)
        );
        assert_eq!(
            Ordering::Less,
            compare_validator_distance(&None, &Some(&monitoring_metadata_1))
        );
    }

    #[test]
    fn test_compare_ping_latency() {
        // Create monitoring metadata with the same ping latency
        let monitoring_metadata_1 = create_metadata_with_latency(Some(1.0));
        let monitoring_metadata_2 = create_metadata_with_latency(Some(1.0));

        // Verify that the metadata is equal
        assert_eq!(
            Ordering::Equal,
            compare_ping_latency(&Some(&monitoring_metadata_1), &Some(&monitoring_metadata_2))
        );

        // Create monitoring metadata with different ping latencies
        let monitoring_metadata_1 = create_metadata_with_latency(Some(0.5));
        let monitoring_metadata_2 = create_metadata_with_latency(Some(2.0));

        // Verify that the metadata has different ordering
        assert_eq!(
            Ordering::Greater,
            compare_ping_latency(&Some(&monitoring_metadata_1), &Some(&monitoring_metadata_2))
        );
        assert_eq!(
            Ordering::Less,
            compare_ping_latency(&Some(&monitoring_metadata_2), &Some(&monitoring_metadata_1))
        );

        // Create monitoring metadata with and without ping latencies
        let monitoring_metadata_1 = create_metadata_with_latency(Some(0.5));
        let monitoring_metadata_2 = create_metadata_with_latency(None);

        // Verify that the metadata with a ping latency has a higher ordering
        assert_eq!(
            Ordering::Greater,
            compare_ping_latency(&Some(&monitoring_metadata_1), &Some(&monitoring_metadata_2))
        );
        assert_eq!(
            Ordering::Less,
            compare_ping_latency(&Some(&monitoring_metadata_2), &Some(&monitoring_metadata_1))
        );

        // Compare monitoring metadata that is missing entirely
        assert_eq!(
            Ordering::Greater,
            compare_ping_latency(&Some(&monitoring_metadata_1), &None)
        );
        assert_eq!(
            Ordering::Less,
            compare_ping_latency(&None, &Some(&monitoring_metadata_1))
        );
    }

    #[test]
    fn test_get_peer_priority() {
        // Create a prioritized peer state
        let prioritized_peers_state = PrioritizedPeersState::new(
            MempoolConfig::default(),
            NodeType::PublicFullnode,
            TimeService::mock(),
        );

        // Create a list of peers
        let validator_peer = create_validator_peer();
        let vfn_peer = create_vfn_peer();
        let public_peer = create_public_peer();

        // Set the prioritized peers
        let prioritized_peers = vec![validator_peer, vfn_peer, public_peer];
        prioritized_peers_state
            .prioritized_peers
            .write()
            .clone_from(&prioritized_peers);

        // Verify that the peer priorities are correct
        for (index, peer) in prioritized_peers.iter().enumerate() {
            let expected_priority = index;
            let actual_priority = prioritized_peers_state.get_peer_priority(peer);
            assert_eq!(actual_priority, expected_priority);
        }
    }

    fn prioritized_peer_state_well_formed(
        prioritized_peers_state: &PrioritizedPeersState,
        num_sender_buckets: u8,
    ) {
        // There is exists a peer with primary priority for each bucket
        for bucket in 0..num_sender_buckets {
            assert!(prioritized_peers_state.peer_to_sender_buckets.iter().any(
                |(_, sender_buckets)| {
                    sender_buckets.contains_key(&bucket)
                        && sender_buckets.get(&bucket).unwrap() == &BroadcastPeerPriority::Primary
                }
            ));
        }

        // There is exists a peer with failover priority for each bucket
        for bucket in 0..num_sender_buckets {
            assert!(prioritized_peers_state.peer_to_sender_buckets.iter().any(
                |(_, sender_buckets)| {
                    sender_buckets.contains_key(&bucket)
                        && sender_buckets.get(&bucket).unwrap() == &BroadcastPeerPriority::Failover
                }
            ));
        }
    }

    fn all_sender_buckets_assigned_to_vfn_network(
        prioritized_peers_state: &PrioritizedPeersState,
        num_sender_buckets: u8,
    ) {
        for bucket in 0..num_sender_buckets {
            assert!(prioritized_peers_state.peer_to_sender_buckets.iter().any(
                |(peer, sender_buckets)| {
                    peer.network_id() == NetworkId::Vfn
                        && sender_buckets.contains_key(&bucket)
                        && sender_buckets.get(&bucket).unwrap() == &BroadcastPeerPriority::Primary
                }
            ));
        }
    }

    #[test]
    fn test_all_sender_buckets_assigned_for_vfns() {
        let mempool_config = MempoolConfig::default();
        let mut prioritized_peers_state = PrioritizedPeersState::new(
            mempool_config.clone(),
            NodeType::ValidatorFullnode,
            TimeService::mock(),
        );

        let peer_metadata_1 = create_metadata_with_distance_and_latency(1, 0.5);
        let peer_1 = (create_public_peer(), Some(&peer_metadata_1));

        let peer_metadata_2 = create_metadata_with_distance_and_latency(1, 0.31);
        let peer_2 = (create_vfn_peer(), Some(&peer_metadata_2));

        // let peer_metadata_3 = create_metadata_with_distance_and_latency(1, 0.5);
        let peer_3 = (create_public_peer(), None);

        let peer_metadata_4 = create_metadata_with_distance_and_latency(1, 0.22);
        let peer_4 = (create_public_peer(), Some(&peer_metadata_4));

        let all_peers = vec![peer_1, peer_2, peer_3, peer_4];
        prioritized_peers_state.update_prioritized_peers(all_peers, 5000, 7000);
        assert!(!prioritized_peers_state.peer_to_sender_buckets.is_empty());
        prioritized_peer_state_well_formed(
            &prioritized_peers_state,
            mempool_config.num_sender_buckets,
        );
        all_sender_buckets_assigned_to_vfn_network(
            &prioritized_peers_state,
            mempool_config.num_sender_buckets,
        );

        let all_peers = vec![peer_1, peer_2, peer_4];
        prioritized_peers_state.update_prioritized_peers(all_peers, 3000, 7000);
        assert!(!prioritized_peers_state.peer_to_sender_buckets.is_empty());
        prioritized_peer_state_well_formed(
            &prioritized_peers_state,
            mempool_config.num_sender_buckets,
        );
        all_sender_buckets_assigned_to_vfn_network(
            &prioritized_peers_state,
            mempool_config.num_sender_buckets,
        );

        let all_peers = vec![peer_3, peer_1];
        prioritized_peers_state.update_prioritized_peers(all_peers, 0, 0);
        assert!(!prioritized_peers_state.peer_to_sender_buckets.is_empty());
        prioritized_peer_state_well_formed(
            &prioritized_peers_state,
            mempool_config.num_sender_buckets,
        );
    }

    #[test]
    fn test_all_sender_buckets_assigned_for_pfns() {
        let mempool_config = MempoolConfig::default();
        let mut prioritized_peers_state = PrioritizedPeersState::new(
            mempool_config.clone(),
            NodeType::PublicFullnode,
            TimeService::mock(),
        );

        let peer_metadata_1 = create_metadata_with_distance_and_latency(1, 0.5);
        let peer_1 = (create_public_peer(), Some(&peer_metadata_1));

        let peer_metadata_2 = create_metadata_with_distance_and_latency(1, 0.31);
        let peer_2 = (create_vfn_peer(), Some(&peer_metadata_2));

        // let peer_metadata_3 = create_metadata_with_distance_and_latency(1, 0.5);
        let peer_3 = (create_public_peer(), None);

        let peer_metadata_4 = create_metadata_with_distance_and_latency(1, 0.22);
        let peer_4 = (create_public_peer(), Some(&peer_metadata_4));

        let all_peers = vec![peer_1, peer_2, peer_3, peer_4];
        prioritized_peers_state.update_prioritized_peers(all_peers, 5000, 2000);
        assert!(!prioritized_peers_state.peer_to_sender_buckets.is_empty());
        prioritized_peer_state_well_formed(
            &prioritized_peers_state,
            mempool_config.num_sender_buckets,
        );

        let all_peers = vec![peer_1, peer_2, peer_4];
        prioritized_peers_state.update_prioritized_peers(all_peers, 3000, 2000);
        assert!(!prioritized_peers_state.peer_to_sender_buckets.is_empty());
        prioritized_peer_state_well_formed(
            &prioritized_peers_state,
            mempool_config.num_sender_buckets,
        );

        let all_peers = vec![peer_3, peer_1];
        prioritized_peers_state.update_prioritized_peers(all_peers, 0, 0);
        assert!(!prioritized_peers_state.peer_to_sender_buckets.is_empty());
        prioritized_peer_state_well_formed(
            &prioritized_peers_state,
            mempool_config.num_sender_buckets,
        );
    }

    #[test]
    fn test_ready_for_update_intelligent() {
        // Create a mempool configuration with intelligent peer prioritization enabled
        let shared_mempool_priority_update_interval_secs = 10;
        let enable_intelligent_peer_prioritization = true;
        let mempool_config = MempoolConfig {
            enable_intelligent_peer_prioritization,
            shared_mempool_priority_update_interval_secs,
            ..MempoolConfig::default()
        };

        // Create a prioritized peer state
        let time_service = TimeService::mock();
        let mut prioritized_peers_state = PrioritizedPeersState::new(
            mempool_config.clone(),
            NodeType::PublicFullnode,
            time_service.clone(),
        );

        // Verify that the prioritized peers should be updated (no prior update time)
        let peers_changed = false;
        assert!(prioritized_peers_state.ready_for_update(peers_changed));

        // Set the last peer priority update time
        prioritized_peers_state.last_peer_priority_update = Some(Instant::now());

        // Verify that the prioritized peers should still be updated (not all ping latencies were observed)
        assert!(prioritized_peers_state.ready_for_update(peers_changed));

        // Set the ping latencies observed flag
        prioritized_peers_state.observed_all_ping_latencies = true;

        // Verify that the prioritized peers should not be updated (not enough time has passed)
        assert!(!prioritized_peers_state.ready_for_update(peers_changed));

        // Emulate a change in peers and verify the prioritized peers should be updated
        assert!(prioritized_peers_state.ready_for_update(true));

        // Elapse some time (but not enough for the prioritized peers to be updated)
        let time_service = time_service.into_mock();
        time_service.advance_secs(shared_mempool_priority_update_interval_secs / 2);

        // Verify that the prioritized peers should not be updated (not enough time has passed)
        assert!(!prioritized_peers_state.ready_for_update(peers_changed));

        // Elapse enough time for the prioritized peers to be updated
        time_service.advance_secs(shared_mempool_priority_update_interval_secs + 1);

        // Verify that the prioritized peers should be updated (enough time has passed)
        assert!(prioritized_peers_state.ready_for_update(peers_changed));
    }

    #[test]
    fn test_ready_for_update_simple() {
        // Create a mempool configuration with intelligent peer prioritization disabled
        let enable_intelligent_peer_prioritization = false;
        let mempool_config = MempoolConfig {
            enable_intelligent_peer_prioritization,
            ..MempoolConfig::default()
        };

        // Create a prioritized peers state
        let time_service = TimeService::mock();
        let prioritized_peers_state = PrioritizedPeersState::new(
            mempool_config.clone(),
            NodeType::PublicFullnode,
            time_service.clone(),
        );

        // Verify that the prioritized peers is updated when the peers change
        for _ in 0..10 {
            let peers_changed = true;
            assert!(prioritized_peers_state.ready_for_update(peers_changed));
        }

        // Verify that the prioritized peers is not updated when the peers remain the same
        for _ in 0..10 {
            let peers_changed = false;
            assert!(!prioritized_peers_state.ready_for_update(peers_changed));
        }

        // Verify that the prioritized peers is updated when the peers change
        for _ in 0..10 {
            let peers_changed = true;
            assert!(prioritized_peers_state.ready_for_update(peers_changed));
        }
    }

    #[test]
    fn test_sort_peers_by_priority_intelligent() {
        // Create a mempool configuration with intelligent peer prioritization enabled
        let enable_intelligent_peer_prioritization = true;
        let mempool_config = MempoolConfig {
            enable_intelligent_peer_prioritization,
            ..MempoolConfig::default()
        };

        // Create a prioritized peer state
        let prioritized_peers_state = PrioritizedPeersState::new(
            mempool_config,
            NodeType::PublicFullnode,
            TimeService::mock(),
        );

        // Create a list of peers (without metadata)
        let validator_peer = (create_validator_peer(), None);
        let vfn_peer = (create_vfn_peer(), None);
        let public_peer = (create_public_peer(), None);

        // Verify that peers are prioritized by network ID first
        let all_peers = vec![vfn_peer, public_peer, validator_peer];
        let prioritized_peers = prioritized_peers_state.sort_peers_by_priority(&all_peers);
        let expected_peers = vec![validator_peer.0, vfn_peer.0, public_peer.0];
        assert_eq!(prioritized_peers, expected_peers);

        // Create a list of peers with the same network ID, but different validator distances
        let peer_metadata_1 = create_metadata_with_distance(Some(1));
        let public_peer_1 = (create_public_peer(), Some(&peer_metadata_1));

        let peer_metadata_2 = create_metadata_with_distance(None);
        let public_peer_2 = (
            create_public_peer(),
            Some(&peer_metadata_2), // No validator distance
        );

        let peer_metadata_3 = create_metadata_with_distance(Some(0));
        let public_peer_3 = (create_public_peer(), Some(&peer_metadata_3));

        let peer_metadata_4 = create_metadata_with_distance(Some(2));
        let public_peer_4 = (create_public_peer(), Some(&peer_metadata_4));

        // Verify that peers on the same network ID are prioritized by validator distance
        let all_peers = vec![public_peer_1, public_peer_2, public_peer_3, public_peer_4];
        let prioritized_peers = prioritized_peers_state.sort_peers_by_priority(&all_peers);
        let expected_peers = vec![
            public_peer_3.0,
            public_peer_1.0,
            public_peer_4.0,
            public_peer_2.0,
        ];
        assert_eq!(prioritized_peers, expected_peers);

        // Create a list of peers with the same network ID and validator distance, but different ping latencies
        let peer_metadata_1 = create_metadata_with_distance_and_latency(1, 0.5);
        let public_peer_1 = (create_public_peer(), Some(&peer_metadata_1));

        let peer_metadata_2 = create_metadata_with_distance_and_latency(1, 2.0);
        let public_peer_2 = (create_public_peer(), Some(&peer_metadata_2));

        let peer_metadata_3 = create_metadata_with_distance_and_latency(1, 0.4);
        let public_peer_3 = (create_public_peer(), Some(&peer_metadata_3));

        let peer_metadata_4 = create_metadata_with_distance(Some(1));
        let public_peer_4 = (
            create_public_peer(),
            Some(&peer_metadata_4), // No ping latency
        );

        // Verify that peers on the same network ID and validator distance are prioritized by ping latency
        let all_peers = vec![public_peer_1, public_peer_2, public_peer_3, public_peer_4];
        let prioritized_peers = prioritized_peers_state.sort_peers_by_priority(&all_peers);
        let expected_peers = vec![
            public_peer_3.0,
            public_peer_1.0,
            public_peer_2.0,
            public_peer_4.0,
        ];
        assert_eq!(prioritized_peers, expected_peers);
    }

    #[test]
    fn test_sort_peers_by_priority_simple() {
        // Create a mempool configuration with intelligent peer prioritization disabled
        let enable_intelligent_peer_prioritization = false;
        let mempool_config = MempoolConfig {
            enable_intelligent_peer_prioritization,
            ..MempoolConfig::default()
        };

        // Create a prioritized peer state
        let prioritized_peers_state = PrioritizedPeersState::new(
            mempool_config,
            NodeType::PublicFullnode,
            TimeService::mock(),
        );

        // Create a list of peers (without metadata)
        let validator_peer = (create_validator_peer(), None);
        let vfn_peer = (create_vfn_peer(), None);
        let public_peer = (create_public_peer(), None);

        // Verify that peers are prioritized by network ID first
        let all_peers = vec![vfn_peer, public_peer, validator_peer];
        let prioritized_peers = prioritized_peers_state.sort_peers_by_priority(&all_peers);
        let expected_peers = vec![validator_peer.0, vfn_peer.0, public_peer.0];
        assert_eq!(prioritized_peers, expected_peers);

        // Create a list of peers with the same network ID
        let mut all_peers = vec![];
        for _ in 0..100 {
            all_peers.push((create_vfn_peer(), None));
        }

        // Sort the peers by priority multiple times and verify that the order is consistent
        let prioritized_peers = prioritized_peers_state.sort_peers_by_priority(&all_peers);
        for _ in 0..10 {
            let new_prioritized_peers = prioritized_peers_state.sort_peers_by_priority(&all_peers);
            assert_eq!(prioritized_peers, new_prioritized_peers);
        }
    }

    #[test]
    fn test_update_prioritized_peers_intelligent() {
        // Create a mempool configuration with intelligent peer prioritization enabled
        let enable_intelligent_peer_prioritization = true;
        let mempool_config = MempoolConfig {
            enable_intelligent_peer_prioritization,
            ..MempoolConfig::default()
        };

        // Create a prioritized peer state
        let time_service = TimeService::mock();
        let mut prioritized_peers_state = PrioritizedPeersState::new(
            mempool_config,
            NodeType::PublicFullnode,
            time_service.clone(),
        );

        // Verify that the last peer priority update time is not set
        assert!(prioritized_peers_state.last_peer_priority_update.is_none());

        // Create a list of peers with and without ping latencies
        let peer_metadata_1 = create_metadata_with_distance_and_latency(1, 0.5);
        let public_peer_1 = (create_public_peer(), Some(&peer_metadata_1));

        let peer_metadata_2 = create_metadata_with_distance_and_latency(1, 2.0);
        let public_peer_2 = (create_public_peer(), Some(&peer_metadata_2));

        let peer_metadata_3 = create_metadata_with_distance_and_latency(1, 0.4);
        let public_peer_3 = (create_public_peer(), Some(&peer_metadata_3));

        let peer_metadata_4 = create_metadata_with_distance(Some(1));
        let public_peer_4 = (
            create_public_peer(),
            Some(&peer_metadata_4), // No ping latency
        );

        // Update the prioritized peers
        let all_peers = vec![public_peer_1, public_peer_2, public_peer_3, public_peer_4];
        prioritized_peers_state.update_prioritized_peers(all_peers, 5000, 7000);

        // Verify that the prioritized peers were updated correctly
        let expected_peers = vec![
            public_peer_3.0,
            public_peer_1.0,
            public_peer_2.0,
            public_peer_4.0,
        ];
        let prioritized_peers = prioritized_peers_state.prioritized_peers.read().clone();
        assert_eq!(prioritized_peers, expected_peers);

        // Verify that the last peer priority update time was set correctly
        assert_eq!(
            prioritized_peers_state.last_peer_priority_update,
            Some(time_service.now())
        );

        // Verify that the observed ping latencies flag was not set
        assert!(!prioritized_peers_state.observed_all_ping_latencies);

        // Elapse some time
        let time_service = time_service.into_mock();
        time_service.advance_secs(100);

        // Update the prioritized peers for only peers with ping latencies
        let all_peers = vec![public_peer_1, public_peer_2, public_peer_3];
        prioritized_peers_state.update_prioritized_peers(all_peers, 5000, 1000);

        // Verify that the prioritized peers were updated correctly
        let expected_peers = vec![public_peer_3.0, public_peer_1.0, public_peer_2.0];
        let prioritized_peers = prioritized_peers_state.prioritized_peers.read().clone();
        assert_eq!(prioritized_peers, expected_peers);

        // Verify that the last peer priority update time was set correctly
        assert_eq!(
            prioritized_peers_state.last_peer_priority_update,
            Some(time_service.now())
        );

        // Verify that the observed ping latencies flag was set
        assert!(prioritized_peers_state.observed_all_ping_latencies);
    }

    #[test]
    fn test_update_prioritized_peers_simple() {
        // Create a mempool configuration with intelligent peer prioritization disabled
        let enable_intelligent_peer_prioritization = false;
        let mempool_config = MempoolConfig {
            enable_intelligent_peer_prioritization,
            ..MempoolConfig::default()
        };

        // Create a prioritized peer state
        let time_service = TimeService::mock();
        let mut prioritized_peers_state = PrioritizedPeersState::new(
            mempool_config,
            NodeType::PublicFullnode,
            time_service.clone(),
        );

        // Create a list of peers with different network IDs
        let validator_peer = (create_validator_peer(), None);
        let vfn_peer = (create_vfn_peer(), None);
        let public_peer = (create_public_peer(), None);

        // Update the prioritized peers
        let all_peers = vec![validator_peer, vfn_peer, public_peer];
        prioritized_peers_state.update_prioritized_peers(all_peers, 5000, 2000);

        // Verify that the prioritized peers were updated correctly
        let expected_peers = vec![validator_peer.0, vfn_peer.0, public_peer.0];
        let prioritized_peers = prioritized_peers_state.prioritized_peers.read().clone();
        assert_eq!(prioritized_peers, expected_peers);

        // Create a list of peers with the same network ID but different metadata
        let mut all_metadata = Vec::new();
        for i in 0..100 {
            let metadata = create_metadata_with_distance_and_latency(i, i as f64);
            all_metadata.push(metadata);
        }
        let all_peers: Vec<_> = all_metadata
            .iter()
            .map(|metadata| (create_public_peer(), Some(metadata)))
            .collect();

        // Update the prioritized peers multiple times and verify that the order is consistent
        let prioritized_peers = prioritized_peers_state.sort_peers_by_priority(&all_peers);
        for _ in 0..10 {
            prioritized_peers_state.update_prioritized_peers(all_peers.clone(), 5000, 2000);
            let new_prioritized_peers = prioritized_peers_state.prioritized_peers.read().clone();
            assert_eq!(prioritized_peers, new_prioritized_peers);
        }

        // Verify that the prioritized peers are not sorted by validator distance
        let distance_sorted_peers = all_peers
            .iter()
            .sorted_by(|peer_a, peer_b| compare_validator_distance(&peer_a.1, &peer_b.1).reverse())
            .map(|(peer, _)| *peer)
            .collect::<Vec<_>>();
        assert_ne!(distance_sorted_peers, prioritized_peers);

        // Verify that the prioritized peers are not sorted by ping latency
        let latency_sorted_peers = all_peers
            .iter()
            .sorted_by(|peer_a, peer_b| compare_ping_latency(&peer_a.1, &peer_b.1).reverse())
            .map(|(peer, _)| *peer)
            .collect::<Vec<_>>();
        assert_ne!(latency_sorted_peers, prioritized_peers);
    }

    /// Creates a peer monitoring metadata with the given distance
    fn create_metadata_with_distance(
        distance_from_validators: Option<u64>,
    ) -> PeerMonitoringMetadata {
        // Create a network info response with the given distance
        let network_info_response =
            distance_from_validators.map(|distance_from_validators| NetworkInformationResponse {
                connected_peers: BTreeMap::new(),
                distance_from_validators,
            });

        // Create the peer monitoring metadata
        PeerMonitoringMetadata::new(None, None, network_info_response, None, None)
    }

    /// Creates a peer monitoring metadata with the given distance and latency
    fn create_metadata_with_distance_and_latency(
        distance_from_validators: u64,
        average_ping_latency_secs: f64,
    ) -> PeerMonitoringMetadata {
        let mut monitoring_metadata = create_metadata_with_distance(Some(distance_from_validators));
        monitoring_metadata.average_ping_latency_secs = Some(average_ping_latency_secs);
        monitoring_metadata
    }

    /// Creates a peer monitoring metadata with the given ping latency
    fn create_metadata_with_latency(
        average_ping_latency_secs: Option<f64>,
    ) -> PeerMonitoringMetadata {
        // Create the peer monitoring metadata
        PeerMonitoringMetadata::new(average_ping_latency_secs, None, None, None, None)
    }

    /// Creates a validator peer with a random peer ID
    fn create_validator_peer() -> PeerNetworkId {
        PeerNetworkId::new(NetworkId::Validator, PeerId::random())
    }

    /// Creates a VFN peer with a random peer ID
    fn create_vfn_peer() -> PeerNetworkId {
        PeerNetworkId::new(NetworkId::Vfn, PeerId::random())
    }

    /// Creates a public peer with a random peer ID
    fn create_public_peer() -> PeerNetworkId {
        PeerNetworkId::new(NetworkId::Public, PeerId::random())
    }
}
