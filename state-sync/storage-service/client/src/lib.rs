// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use aptos_config::network_id::PeerNetworkId;
use aptos_network::{
    application::{interface::NetworkClientInterface, storage::PeerMetadataStorage},
    protocols::network::{NetworkApplicationConfig, RpcError},
    ProtocolId,
};
use aptos_storage_service_types::{
    requests::StorageServiceRequest, responses::StorageServiceResponse, StorageServiceError,
    StorageServiceMessage,
};
use std::{sync::Arc, time::Duration};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Network RPC error: {0}")]
    RpcError(#[from] RpcError),

    #[error("Error from remote storage service: {0}")]
    StorageServiceError(#[from] StorageServiceError),
}

/// The interface for sending Storage Service requests and querying network peer
/// information.
#[derive(Clone, Debug)]
pub struct StorageServiceClient<NetworkClient> {
    network_client: NetworkClient,
}

impl<NetworkClient: NetworkClientInterface<StorageServiceMessage>>
    StorageServiceClient<NetworkClient>
{
    pub fn new(network_client: NetworkClient) -> Self {
        Self { network_client }
    }

    pub async fn send_request(
        &self,
        recipient: PeerNetworkId,
        timeout: Duration,
        request: StorageServiceRequest,
    ) -> Result<StorageServiceResponse, Error> {
        let response = self
            .network_client
            .send_to_peer_rpc(StorageServiceMessage::Request(request), timeout, recipient)
            .await
            .map_err(|error| Error::NetworkError(error.to_string()))?;
        match response {
            StorageServiceMessage::Response(Ok(response)) => Ok(response),
            StorageServiceMessage::Response(Err(err)) => Err(Error::StorageServiceError(err)),
            StorageServiceMessage::Request(request) => Err(Error::NetworkError(format!(
                "Got storage service request instead of response! Request: {:?}",
                request
            ))),
        }
    }

    pub fn get_peer_metadata_storage(&self) -> Arc<PeerMetadataStorage> {
        self.network_client.get_peer_metadata_storage()
    }
}

/// Returns a network application config for the storage client
pub fn storage_client_network_config() -> NetworkApplicationConfig {
    NetworkApplicationConfig::client([ProtocolId::StorageServiceRpc])
}
