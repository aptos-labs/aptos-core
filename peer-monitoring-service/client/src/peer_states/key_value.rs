// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    peer_states::{
        latency_info::LatencyInfoState, network_info::NetworkInfoState, node_info::NodeInfoState,
        request_tracker::RequestTracker,
    },
    Error,
};
use velor_config::{config::NodeConfig, network_id::PeerNetworkId};
use velor_infallible::RwLock;
use velor_network::application::metadata::PeerMetadata;
use velor_peer_monitoring_service_types::{
    request::{LatencyPingRequest, PeerMonitoringServiceRequest},
    response::PeerMonitoringServiceResponse,
};
use velor_time_service::TimeService;
use enum_dispatch::enum_dispatch;
use std::{fmt::Display, sync::Arc};

/// A simple enum representing the different types of
/// states held for each peer.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PeerStateKey {
    LatencyInfo,
    NetworkInfo,
    NodeInfo,
}

impl PeerStateKey {
    /// A utility function for getting all peer state keys
    pub fn get_all_keys() -> Vec<PeerStateKey> {
        vec![
            PeerStateKey::LatencyInfo,
            PeerStateKey::NetworkInfo,
            PeerStateKey::NodeInfo,
        ]
    }

    /// Returns the label for the peer state key
    pub fn get_label(&self) -> &str {
        match self {
            PeerStateKey::LatencyInfo => "latency_info",
            PeerStateKey::NetworkInfo => "network_info",
            PeerStateKey::NodeInfo => "node_info",
        }
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
            PeerStateKey::NodeInfo => PeerMonitoringServiceRequest::GetNodeInformation.get_label(),
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
        &mut self,
        peer_network_id: &PeerNetworkId,
        error: Error,
    );

    /// Updates the peer state metrics for the given peer
    fn update_peer_state_metrics(&self, peer_network_id: &PeerNetworkId);
}

/// A simple enum representing the different types of
/// state values for each peer.
#[enum_dispatch(StateValueInterface)]
#[derive(Clone, Debug)]
pub enum PeerStateValue {
    LatencyInfoState,
    NetworkInfoState,
    NodeInfoState,
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
            PeerStateKey::NodeInfo => {
                let node_monitoring_config = node_config.peer_monitoring_service.node_monitoring;
                NodeInfoState::new(node_monitoring_config, time_service).into()
            },
        }
    }
}

// Display each peer state value as its type and internal state
impl Display for PeerStateValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PeerStateValue::LatencyInfoState(state) => write!(f, "LatencyInfoState: {}", state),
            PeerStateValue::NetworkInfoState(state) => write!(f, "NetworkInfoState: {}", state),
            PeerStateValue::NodeInfoState(state) => write!(f, "NodeInfoState: {}", state),
        }
    }
}
