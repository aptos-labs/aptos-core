// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use velor_config::network_id::PeerNetworkId;
use velor_network::{
    application::{interface::NetworkClientInterface, storage::PeersAndMetadata},
    protocols::network::RpcError,
};
use velor_storage_service_types::{
    requests::StorageServiceRequest, responses::StorageServiceResponse, StorageServiceError,
    StorageServiceMessage,
};
use std::{collections::HashSet, sync::Arc, time::Duration};
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

/// The interface for sending Storage Service requests and
/// querying network peer information.
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

    pub fn get_available_peers(&self) -> Result<HashSet<PeerNetworkId>, Error> {
        self.network_client
            .get_available_peers()
            .map(|peers| peers.into_iter().collect())
            .map_err(|error| Error::NetworkError(error.to_string()))
    }

    pub fn get_peers_and_metadata(&self) -> Arc<PeersAndMetadata> {
        self.network_client.get_peers_and_metadata()
    }
}
