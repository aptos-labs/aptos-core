// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use cfg_block::cfg_block;
use serde::{Deserialize, Serialize};

/// A peer monitoring service request
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum PeerMonitoringServiceRequest {
    GetNetworkInformation,    // Returns relevant network information for the peer
    GetNodeInformation,       // Returns relevant node information about the peer
    GetServerProtocolVersion, // Fetches the protocol version run by the server
    LatencyPing(LatencyPingRequest), // A simple message used by the client to ensure liveness and measure latency

    #[cfg(feature = "network-perf-test")] // Disabled by default
    PerformanceMonitoringRequest(PerformanceMonitoringRequest), // A request to monitor network performance
}

impl PeerMonitoringServiceRequest {
    /// Returns a summary label for the request
    pub fn get_label(&self) -> &'static str {
        match self {
            Self::GetNetworkInformation => "get_network_information",
            Self::GetNodeInformation => "get_node_information",
            Self::GetServerProtocolVersion => "get_server_protocol_version",
            Self::LatencyPing(_) => "latency_ping",

            #[cfg(feature = "network-perf-test")] // Disabled by default
            Self::PerformanceMonitoringRequest(_) => "performance_monitoring_request",
        }
    }
}

/// The latency ping request
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct LatencyPingRequest {
    pub ping_counter: u64, // A monotonically increasing counter to verify latency ping responses
}

cfg_block! {
    #[cfg(feature = "network-perf-test")] { // Disabled by default
        /// The performance monitoring request
        #[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
        pub struct PerformanceMonitoringRequest {
            pub request_counter: u64, // A monotonically increasing counter to verify responses
            pub data: Vec<u8>, // A vector of bytes to send in the request
        }
    }
}
