// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use aptos_config::network_id::PeerNetworkId;
use network::application::types::PeerInfo;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, convert::TryFrom};
use thiserror::Error;

pub type Result<T, E = PeerMonitoringServiceError> = ::std::result::Result<T, E>;

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

/// A peer monitoring service request
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum PeerMonitoringServiceRequest {
    GetConnectedPeers,        // Returns all connected peers
    GetDepthFromValidators,   // Returns the depth of the node from the validators
    GetKnownPeers,            // Returns all of the known peers in the network
    GetServerProtocolVersion, // Fetches the protocol version run by the server
    GetValidatorsAndVFNs,     // Returns the current validators and VFNs
    Ping, // A simple message used by the client to ensure liveness and measure latency
}

impl PeerMonitoringServiceRequest {
    /// Returns a summary label for the request
    pub fn get_label(&self) -> &'static str {
        match self {
            Self::GetConnectedPeers => "get_connected_peers",
            Self::GetDepthFromValidators => "get_depth_from_validators",
            Self::GetKnownPeers => "get_known_peers",
            Self::GetServerProtocolVersion => "get_server_protocol_version",
            Self::GetValidatorsAndVFNs => "get_validators_and_vfns",
            Self::Ping => "ping",
        }
    }
}

/// A peer monitoring service response
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[allow(clippy::large_enum_variant)]
pub enum PeerMonitoringServiceResponse {
    ConnectedPeers(ConnectedPeersResponse), // Holds all currently connected peers
    DepthFromValidators(DepthFromValidatorsResponse), // Holds the min depth from the validators
    KnownPeers(KnownPeersResponse),         // Holds all currently known peers
    Ping(PingResponse), // A simple message to respond to liveness checks (i.e., pings)
    ServerProtocolVersion(ServerProtocolVersionResponse), // Returns the current server protocol version
    ValidatorsAndVFNs(ValidatorsAndVFNsResponse), // Holds the current validator set and VFNs
}

impl PeerMonitoringServiceResponse {
    /// Returns a summary label for the response
    pub fn get_label(&self) -> &'static str {
        match self {
            Self::ConnectedPeers(_) => "connected_peers",
            Self::DepthFromValidators(_) => "depth_from_validators",
            Self::KnownPeers(_) => "known_peers",
            Self::Ping(_) => "ping",
            Self::ServerProtocolVersion(_) => "server_protocol_version",
            Self::ValidatorsAndVFNs(_) => "validators_and_vfns",
        }
    }
}

/// A response for the connected peers request
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConnectedPeersResponse {
    pub connected_peers: HashMap<PeerNetworkId, PeerInfo>,
}

/// A response for the depth from validators request
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DepthFromValidatorsResponse {
    pub todo: bool,
}

/// A response for the known peers request
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct KnownPeersResponse {
    pub todo: bool,
}

/// A response for the ping request
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PingResponse {
    pub todo: bool,
}

/// A response for the server protocol version request
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ServerProtocolVersionResponse {
    pub version: u64,
}

/// A response for the current validators and VFNs
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ValidatorsAndVFNsResponse {
    pub todo: bool,
}

#[derive(Clone, Debug, Error)]
#[error("Unexpected response variant: {0}")]
pub struct UnexpectedResponseError(pub String);

impl TryFrom<PeerMonitoringServiceResponse> for ConnectedPeersResponse {
    type Error = UnexpectedResponseError;
    fn try_from(response: PeerMonitoringServiceResponse) -> Result<Self, Self::Error> {
        match response {
            PeerMonitoringServiceResponse::ConnectedPeers(inner) => Ok(inner),
            _ => Err(UnexpectedResponseError(format!(
                "expected connected_peers_response, found {}",
                response.get_label()
            ))),
        }
    }
}

impl TryFrom<PeerMonitoringServiceResponse> for DepthFromValidatorsResponse {
    type Error = UnexpectedResponseError;
    fn try_from(response: PeerMonitoringServiceResponse) -> Result<Self, Self::Error> {
        match response {
            PeerMonitoringServiceResponse::DepthFromValidators(inner) => Ok(inner),
            _ => Err(UnexpectedResponseError(format!(
                "expected depth_from_validators_response, found {}",
                response.get_label()
            ))),
        }
    }
}

impl TryFrom<PeerMonitoringServiceResponse> for KnownPeersResponse {
    type Error = UnexpectedResponseError;
    fn try_from(response: PeerMonitoringServiceResponse) -> Result<Self, Self::Error> {
        match response {
            PeerMonitoringServiceResponse::KnownPeers(inner) => Ok(inner),
            _ => Err(UnexpectedResponseError(format!(
                "expected known_peers_response, found {}",
                response.get_label()
            ))),
        }
    }
}

impl TryFrom<PeerMonitoringServiceResponse> for PingResponse {
    type Error = UnexpectedResponseError;
    fn try_from(response: PeerMonitoringServiceResponse) -> Result<Self, Self::Error> {
        match response {
            PeerMonitoringServiceResponse::Ping(inner) => Ok(inner),
            _ => Err(UnexpectedResponseError(format!(
                "expected ping_response, found {}",
                response.get_label()
            ))),
        }
    }
}

impl TryFrom<PeerMonitoringServiceResponse> for ServerProtocolVersionResponse {
    type Error = UnexpectedResponseError;
    fn try_from(response: PeerMonitoringServiceResponse) -> Result<Self, Self::Error> {
        match response {
            PeerMonitoringServiceResponse::ServerProtocolVersion(inner) => Ok(inner),
            _ => Err(UnexpectedResponseError(format!(
                "expected server_protocol_version_response, found {}",
                response.get_label()
            ))),
        }
    }
}

impl TryFrom<PeerMonitoringServiceResponse> for ValidatorsAndVFNsResponse {
    type Error = UnexpectedResponseError;
    fn try_from(response: PeerMonitoringServiceResponse) -> Result<Self, Self::Error> {
        match response {
            PeerMonitoringServiceResponse::ValidatorsAndVFNs(inner) => Ok(inner),
            _ => Err(UnexpectedResponseError(format!(
                "expected validators_and_vfns_response, found {}",
                response.get_label()
            ))),
        }
    }
}
