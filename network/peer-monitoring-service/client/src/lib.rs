// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use aptos_config::network_id::PeerNetworkId;
use aptos_network::{
    application::{interface::NetworkClientInterface, storage::PeerMetadataStorage},
    protocols::network::{NetworkApplicationConfig, RpcError},
    ProtocolId,
};
use aptos_peer_monitoring_service_types::{
    PeerMonitoringServiceError, PeerMonitoringServiceMessage, PeerMonitoringServiceRequest,
    PeerMonitoringServiceResponse,
};
use std::{sync::Arc, time::Duration};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Aptos network rpc error: {0}")]
    RpcError(#[from] RpcError),

    #[error("Error from remote monitoring service: {0}")]
    PeerMonitoringServiceError(#[from] PeerMonitoringServiceError),
}

/// The interface for sending peer monitoring service requests and querying
/// peer information.
#[derive(Clone, Debug)]
pub struct PeerMonitoringServiceClient<NetworkClient> {
    network_client: NetworkClient,
}

impl<NetworkClient: NetworkClientInterface<PeerMonitoringServiceMessage>>
    PeerMonitoringServiceClient<NetworkClient>
{
    pub fn new(network_client: NetworkClient) -> Self {
        Self { network_client }
    }

    pub async fn send_request(
        &self,
        recipient: PeerNetworkId,
        request: PeerMonitoringServiceRequest,
        timeout: Duration,
    ) -> Result<PeerMonitoringServiceResponse, Error> {
        let response = self
            .network_client
            .send_to_peer_rpc(
                PeerMonitoringServiceMessage::Request(request),
                timeout,
                recipient,
            )
            .await
            .map_err(|error| Error::NetworkError(error.to_string()))?;
        match response {
            PeerMonitoringServiceMessage::Response(Ok(response)) => Ok(response),
            PeerMonitoringServiceMessage::Response(Err(err)) => {
                Err(Error::PeerMonitoringServiceError(err))
            },
            PeerMonitoringServiceMessage::Request(_) => {
                Err(Error::RpcError(RpcError::InvalidRpcResponse))
            },
        }
    }

    pub fn get_peer_metadata_storage(&self) -> Arc<PeerMetadataStorage> {
        self.network_client.get_peer_metadata_storage()
    }
}

/// Returns a network application config for the peer monitoring client
pub fn peer_monitoring_client_network_config() -> NetworkApplicationConfig {
    NetworkApplicationConfig::client([ProtocolId::PeerMonitoringServiceRpc])
}
