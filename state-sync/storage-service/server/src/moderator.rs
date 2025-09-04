// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{error::Error, logging::LogEntry, metrics, utils, LogSchema};
use velor_config::{
    config::{VelorDataClientConfig, StorageServiceConfig},
    network_id::{NetworkId, PeerNetworkId},
};
use velor_logger::warn;
use velor_network::application::storage::PeersAndMetadata;
use velor_storage_service_types::{
    requests::StorageServiceRequest, responses::StorageServerSummary,
};
use velor_time_service::{TimeService, TimeServiceTrait};
use arc_swap::ArcSwap;
use dashmap::DashMap;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

/// A simple struct that tracks the state of an unhealthy peer
#[derive(Clone, Debug)]
pub struct UnhealthyPeerState {
    ignore_start_time: Option<Instant>, // The time when we first started ignoring the peer
    invalid_request_count: u64,         // The total number of invalid requests from the peer
    max_invalid_requests: u64, // The max number of invalid requests before ignoring the peer
    min_time_to_ignore_secs: u64, // The min time (secs) to ignore the peer (doubles each round)
    time_service: TimeService, // The time service
}

impl UnhealthyPeerState {
    pub fn new(
        max_invalid_requests: u64,
        min_time_to_ignore_secs: u64,
        time_service: TimeService,
    ) -> Self {
        Self {
            ignore_start_time: None,
            invalid_request_count: 0,
            max_invalid_requests,
            min_time_to_ignore_secs,
            time_service,
        }
    }

    /// Increments the invalid request count for the peer and marks
    /// the peer to be ignored if it has sent too many invalid requests.
    /// Note: we only ignore peers on the public network.
    pub fn increment_invalid_request_count(&mut self, peer_network_id: &PeerNetworkId) {
        // Increment the invalid request count
        self.invalid_request_count += 1;

        // If the peer is a PFN and has sent too many invalid requests, start ignoring it
        if self.ignore_start_time.is_none()
            && peer_network_id.network_id().is_public_network()
            && self.invalid_request_count >= self.max_invalid_requests
        {
            // TODO: at some point we'll want to terminate the connection entirely

            // Start ignoring the peer
            self.ignore_start_time = Some(self.time_service.now());

            // Log the fact that we're now ignoring the peer
            warn!(LogSchema::new(LogEntry::RequestModeratorIgnoredPeer)
                .peer_network_id(peer_network_id)
                .message("Ignoring peer due to too many invalid requests!"));
        }
    }

    /// Returns true iff the peer should be ignored
    pub fn is_ignored(&self) -> bool {
        self.ignore_start_time.is_some()
    }

    /// Refreshes the peer's state (if it has been ignored for long enough).
    /// Note: each time we unblock a peer, we double the min time to ignore the peer.
    /// This provides an exponential backoff for peers that are sending too many invalid requests.
    pub fn refresh_peer_state(&mut self, peer_network_id: &PeerNetworkId) {
        if let Some(ignore_start_time) = self.ignore_start_time {
            let ignored_duration = self.time_service.now().duration_since(ignore_start_time);
            if ignored_duration >= Duration::from_secs(self.min_time_to_ignore_secs) {
                // Reset the invalid request count
                self.invalid_request_count = 0;

                // Reset the ignore start time
                self.ignore_start_time = None;

                // Double the min time to ignore the peer
                self.min_time_to_ignore_secs *= 2;

                // Log the fact that we're no longer ignoring the peer
                warn!(LogSchema::new(LogEntry::RequestModeratorIgnoredPeer)
                    .peer_network_id(peer_network_id)
                    .message("No longer ignoring peer! Enough time has elapsed."));
            }
        }
    }
}

/// The request moderator is responsible for validating inbound storage
/// requests and ensuring that only valid (and satisfiable) requests are processed.
/// If a peer sends too many invalid requests, the moderator will mark the peer as
/// "unhealthy" and will ignore requests from that peer for some time.
pub struct RequestModerator {
    velor_data_client_config: VelorDataClientConfig,
    cached_storage_server_summary: Arc<ArcSwap<StorageServerSummary>>,
    peers_and_metadata: Arc<PeersAndMetadata>,
    storage_service_config: StorageServiceConfig,
    time_service: TimeService,
    unhealthy_peer_states: Arc<DashMap<PeerNetworkId, UnhealthyPeerState>>,
}

impl RequestModerator {
    pub fn new(
        velor_data_client_config: VelorDataClientConfig,
        cached_storage_server_summary: Arc<ArcSwap<StorageServerSummary>>,
        peers_and_metadata: Arc<PeersAndMetadata>,
        storage_service_config: StorageServiceConfig,
        time_service: TimeService,
    ) -> Self {
        Self {
            velor_data_client_config,
            cached_storage_server_summary,
            unhealthy_peer_states: Arc::new(DashMap::new()),
            peers_and_metadata,
            storage_service_config,
            time_service,
        }
    }

    /// Validates the given request and verifies that the peer is behaving
    /// correctly. If the request fails validation, an error is returned.
    pub fn validate_request(
        &self,
        peer_network_id: &PeerNetworkId,
        request: &StorageServiceRequest,
    ) -> Result<(), Error> {
        // Validate the request and time the operation
        let validate_request = || {
            // If the peer is being ignored, return an error
            if let Some(peer_state) = self.unhealthy_peer_states.get(peer_network_id) {
                if peer_state.is_ignored() {
                    return Err(Error::TooManyInvalidRequests(format!(
                        "Peer is temporarily ignored. Unable to handle request: {:?}",
                        request
                    )));
                }
            }

            // Get the latest storage server summary
            let storage_server_summary = self.cached_storage_server_summary.load();

            // Verify the request is serviceable using the current storage server summary
            if !storage_server_summary.can_service(
                &self.velor_data_client_config,
                self.time_service.clone(),
                request,
            ) {
                // Increment the invalid request count for the peer
                let mut unhealthy_peer_state = self
                    .unhealthy_peer_states
                    .entry(*peer_network_id)
                    .or_insert_with(|| {
                        // Create a new unhealthy peer state (this is the first invalid request)
                        let max_invalid_requests =
                            self.storage_service_config.max_invalid_requests_per_peer;
                        let min_time_to_ignore_peers_secs =
                            self.storage_service_config.min_time_to_ignore_peers_secs;
                        let time_service = self.time_service.clone();

                        UnhealthyPeerState::new(
                            max_invalid_requests,
                            min_time_to_ignore_peers_secs,
                            time_service,
                        )
                    });
                unhealthy_peer_state.increment_invalid_request_count(peer_network_id);

                // Return the validation error
                return Err(Error::InvalidRequest(format!(
                    "The given request cannot be satisfied. Request: {:?}, storage summary: {:?}",
                    request, storage_server_summary
                )));
            }

            Ok(()) // The request is valid
        };
        utils::execute_and_time_duration(
            &metrics::STORAGE_REQUEST_VALIDATION_LATENCY,
            Some((peer_network_id, request)),
            None,
            validate_request,
            None,
        )
    }

    /// Refresh the unhealthy peer states and garbage collect disconnected peers
    pub fn refresh_unhealthy_peer_states(&self) -> Result<(), Error> {
        // Get the currently connected peers
        let connected_peers_and_metadata = self
            .peers_and_metadata
            .get_connected_peers_and_metadata()
            .map_err(|error| {
                Error::UnexpectedErrorEncountered(format!(
                    "Unable to get connected peers and metadata: {}",
                    error
                ))
            })?;

        // Remove disconnected peers and refresh ignored peer states
        let mut num_ignored_peers = 0;
        self.unhealthy_peer_states
            .retain(|peer_network_id, unhealthy_peer_state| {
                if connected_peers_and_metadata.contains_key(peer_network_id) {
                    // Refresh the ignored peer state
                    unhealthy_peer_state.refresh_peer_state(peer_network_id);

                    // If the peer is ignored, increment the ignored peer count
                    if unhealthy_peer_state.is_ignored() {
                        num_ignored_peers += 1;
                    }

                    true // The peer is still connected, so we should keep it
                } else {
                    false // The peer is no longer connected, so we should remove it
                }
            });

        // Update the number of ignored peers
        metrics::set_gauge(
            &metrics::IGNORED_PEER_COUNT,
            NetworkId::Public.as_str(),
            num_ignored_peers,
        );

        Ok(())
    }

    #[cfg(test)]
    /// Returns a copy of the unhealthy peer states for testing
    pub(crate) fn get_unhealthy_peer_states(
        &self,
    ) -> Arc<DashMap<PeerNetworkId, UnhealthyPeerState>> {
        self.unhealthy_peer_states.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use velor_types::PeerId;

    #[test]
    fn test_unhealthy_peer_ignored() {
        // Create a new unhealthy peer state
        let max_invalid_requests = 5;
        let min_time_to_ignore_peers_secs = 1;
        let time_service = TimeService::mock();
        let mut unhealthy_peer_state = UnhealthyPeerState::new(
            max_invalid_requests,
            min_time_to_ignore_peers_secs,
            time_service.clone(),
        );

        // Verify the initial peer state
        assert_eq!(unhealthy_peer_state.invalid_request_count, 0);
        assert_eq!(unhealthy_peer_state.ignore_start_time, None);
        assert_eq!(
            unhealthy_peer_state.max_invalid_requests,
            max_invalid_requests
        );
        assert!(!unhealthy_peer_state.is_ignored());

        // Handle the maximum number of invalid requests
        let peer_network_id = PeerNetworkId::new(NetworkId::Public, PeerId::random());
        for _ in 0..max_invalid_requests {
            unhealthy_peer_state.increment_invalid_request_count(&peer_network_id);
        }

        // Verify the peer is now ignored
        assert_eq!(unhealthy_peer_state.invalid_request_count, 5);
        assert!(unhealthy_peer_state.is_ignored());
        assert_eq!(
            unhealthy_peer_state.ignore_start_time,
            Some(time_service.now())
        );

        // Elapse the minimum time to unblock peers
        let time_service = time_service.into_mock();
        time_service.advance(Duration::from_secs(min_time_to_ignore_peers_secs));

        // Refresh the peer state and verify it is no longer ignored
        unhealthy_peer_state.refresh_peer_state(&peer_network_id);
        assert!(!unhealthy_peer_state.is_ignored());

        // Verify the peer state is reset
        assert_eq!(unhealthy_peer_state.invalid_request_count, 0);
        assert_eq!(unhealthy_peer_state.ignore_start_time, None);
    }

    #[test]
    fn test_unhealthy_peer_exponential_backoff() {
        // Create a new unhealthy peer state
        let max_invalid_requests = 10;
        let min_time_to_ignore_peers_secs = 1;
        let time_service = TimeService::mock();
        let mut unhealthy_peer_state = UnhealthyPeerState::new(
            max_invalid_requests,
            min_time_to_ignore_peers_secs,
            time_service.clone(),
        );

        // Verify the initial ignore duration
        assert_eq!(
            unhealthy_peer_state.min_time_to_ignore_secs,
            min_time_to_ignore_peers_secs
        );

        // Perform several iterations of ignore and unblock loops
        let time_service = time_service.into_mock();
        for i in 0..10 {
            // Verify the initial peer state
            let expected_min_time_to_ignore_secs =
                min_time_to_ignore_peers_secs * 2_i32.pow(i) as u64;
            assert_eq!(
                unhealthy_peer_state.min_time_to_ignore_secs,
                expected_min_time_to_ignore_secs
            );

            // Handle the maximum number of invalid requests
            let peer_network_id = PeerNetworkId::new(NetworkId::Public, PeerId::random());
            for _ in 0..max_invalid_requests {
                unhealthy_peer_state.increment_invalid_request_count(&peer_network_id);
            }

            // Verify the peer is now ignored
            assert!(unhealthy_peer_state.is_ignored());
            assert_eq!(
                unhealthy_peer_state.ignore_start_time,
                Some(time_service.now())
            );

            // Elapse the minimum time to unblock peers
            time_service.advance(Duration::from_secs(expected_min_time_to_ignore_secs));

            // Refresh the peer state and verify it is no longer ignored
            unhealthy_peer_state.refresh_peer_state(&peer_network_id);
            assert!(!unhealthy_peer_state.is_ignored());

            // Verify the peer state is reset
            assert_eq!(unhealthy_peer_state.ignore_start_time, None);
        }
    }

    #[test]
    fn test_unhealthy_peer_networks() {
        // Create a new unhealthy peer state
        let max_invalid_requests = 10;
        let time_service = TimeService::mock();
        let mut unhealthy_peer_state =
            UnhealthyPeerState::new(max_invalid_requests, 1, time_service.clone());

        // Handle a lot of invalid requests for a validator
        let peer_network_id = PeerNetworkId::new(NetworkId::Validator, PeerId::random());
        for _ in 0..max_invalid_requests * 10 {
            unhealthy_peer_state.increment_invalid_request_count(&peer_network_id);
        }

        // Verify the peer is not ignored and that the number of invalid requests is correct
        assert!(!unhealthy_peer_state.is_ignored());
        assert_eq!(
            unhealthy_peer_state.invalid_request_count,
            max_invalid_requests * 10
        );

        // Create another unhealthy peer state
        let mut unhealthy_peer_state =
            UnhealthyPeerState::new(max_invalid_requests, 1, time_service.clone());

        // Handle a lot of invalid requests for a VFN
        let peer_network_id = PeerNetworkId::new(NetworkId::Vfn, PeerId::random());
        for _ in 0..max_invalid_requests * 20 {
            unhealthy_peer_state.increment_invalid_request_count(&peer_network_id);
        }

        // Verify the peer is not ignored and that the number of invalid requests is correct
        assert!(!unhealthy_peer_state.is_ignored());
        assert_eq!(
            unhealthy_peer_state.invalid_request_count,
            max_invalid_requests * 20
        );

        // Create another unhealthy peer state
        let mut unhealthy_peer_state =
            UnhealthyPeerState::new(max_invalid_requests, 1, time_service);

        // Handle a lot of invalid requests for a PFN
        let peer_network_id = PeerNetworkId::new(NetworkId::Public, PeerId::random());
        for _ in 0..max_invalid_requests * 5 {
            unhealthy_peer_state.increment_invalid_request_count(&peer_network_id);
        }

        // Verify the peer is ignored and that the number of invalid requests is correct
        assert!(unhealthy_peer_state.is_ignored());
        assert_eq!(
            unhealthy_peer_state.invalid_request_count,
            max_invalid_requests * 5
        );
    }
}
