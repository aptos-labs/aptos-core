// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::response::{NetworkInformationResponse, NodeInformationResponse};
use request::PeerMonitoringServiceRequest;
use response::PeerMonitoringServiceResponse;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod request;
pub mod response;

pub type Result<T, E = PeerMonitoringServiceError> = ::std::result::Result<T, E>;

/// Useful global constants
pub const MAX_DISTANCE_FROM_VALIDATORS: u64 = 100; // Nodes that aren't connected to the network

/// An error that can be returned to the client on a failure to
/// process a request.
#[derive(Clone, Debug, Deserialize, Error, PartialEq, Eq, Serialize)]
pub enum PeerMonitoringServiceError {
    #[error("Internal service error: {0}")]
    InternalError(String),
    #[error("Invalid service request: {0}")]
    InvalidRequest(String),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[allow(clippy::large_enum_variant)]
pub enum PeerMonitoringServiceMessage {
    /// A request to the peer monitoring service
    Request(PeerMonitoringServiceRequest),
    /// A response from the peer monitoring service
    Response(Result<PeerMonitoringServiceResponse>),
}

/// The peer monitoring metadata for a peer
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct PeerMonitoringMetadata {
    pub average_ping_latency_secs: Option<f64>, // The average latency ping for the peer
    pub latest_network_info_response: Option<NetworkInformationResponse>, // The latest network info response
    pub latest_node_info_response: Option<NodeInformationResponse>, // The latest node info response
}

/// We must manually define this because f64 doesn't implement Eq. Instead,
/// we rely on PartialEq (which is sufficient for our use-cases).
impl Eq for PeerMonitoringMetadata {}

impl PeerMonitoringMetadata {
    pub fn new(
        average_ping_latency_secs: Option<f64>,
        latest_network_info_response: Option<NetworkInformationResponse>,
        latest_node_info_response: Option<NodeInformationResponse>,
    ) -> Self {
        PeerMonitoringMetadata {
            average_ping_latency_secs,
            latest_network_info_response,
            latest_node_info_response,
        }
    }
}
