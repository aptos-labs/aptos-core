// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    peer_states::{
        latency_info::LatencyInfoState, network_info::NetworkInfoState,
        request_tracker::RequestTracker,
    },
    Error,
};
use aptos_config::{config::NodeConfig, network_id::PeerNetworkId};
use aptos_infallible::RwLock;
use aptos_network::application::metadata::PeerMetadata;
use aptos_peer_monitoring_service_types::{
    LatencyPingRequest, PeerMonitoringServiceRequest, PeerMonitoringServiceResponse,
};
use aptos_time_service::TimeService;
use enum_dispatch::enum_dispatch;
use std::sync::Arc;

/// A simple enum representing the different types of
/// states held for each peer.
#[derive(Debug, Copy, Clone, Eq, Hash, PartialEq)]
pub enum PeerStateKey {
    LatencyInfo,
    NetworkInfo,
}

impl PeerStateKey {
    /// A utility function for getting all peer state keys
    pub fn get_all_keys() -> Vec<PeerStateKey> {
        vec![PeerStateKey::LatencyInfo, PeerStateKey::NetworkInfo]
    }

    // TODO: Can we avoid exposing this label construction here?
    /// Returns the metric label for the requests sent by the peer state key
    pub fn get_metrics_request_label(&self) -> &str {
        match self {
            PeerStateKey::LatencyInfo => {
                PeerMonitoringServiceRequest::LatencyPing(LatencyPingRequest { ping_counter: 0 })
                    .get_label()
            },
            PeerStateKey::NetworkInfo => {
                PeerMonitoringServiceRequest::GetNetworkInformation.get_label()
            },
        }
    }
}

/// The interface offered by all peer state value types
#[enum_dispatch]
pub trait StateValueInterface {
    /// Creates the monitoring service request
    fn create_monitoring_service_request(&mut self) -> PeerMonitoringServiceRequest;

    /// Returns the request timeout (ms)
    fn get_request_timeout_ms(&self) -> u64;

    /// Returns the request tracker
    fn get_request_tracker(&self) -> Arc<RwLock<RequestTracker>>;

    /// Handles the monitoring service response
    fn handle_monitoring_service_response(
        &mut self,
        peer_network_id: &PeerNetworkId,
        peer_metadata: PeerMetadata,
        monitoring_service_request: PeerMonitoringServiceRequest,
        monitoring_service_response: PeerMonitoringServiceResponse,
        response_time_secs: f64,
    );

    /// Handles a monitoring service error
    fn handle_monitoring_service_response_error(
        &self,
        peer_network_id: &PeerNetworkId,
        error: Error,
    );
}

/// A simple enum representing the different types of
/// states values for each peer.
#[enum_dispatch(StateValueInterface)]
#[derive(Clone, Debug)]
pub enum PeerStateValue {
    LatencyInfoState,
    NetworkInfoState,
}

impl PeerStateValue {
    pub fn new(
        node_config: NodeConfig,
        time_service: TimeService,
        peer_state_key: &PeerStateKey,
    ) -> Self {
        match peer_state_key {
            PeerStateKey::LatencyInfo => {
                let latency_monitoring_config =
                    node_config.peer_monitoring_service.latency_monitoring;
                LatencyInfoState::new(latency_monitoring_config, time_service).into()
            },
            PeerStateKey::NetworkInfo => NetworkInfoState::new(node_config, time_service).into(),
        }
    }
}
