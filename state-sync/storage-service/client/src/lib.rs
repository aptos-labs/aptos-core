// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use async_trait::async_trait;
use diem_config::network_id::PeerNetworkId;
use diem_types::PeerId;
use network::{
    application::{
        interface::{MultiNetworkSender, NetworkInterface},
        storage::{LockingHashMap, PeerMetadataStorage},
    },
    error::NetworkError,
    peer_manager::{ConnectionRequestSender, PeerManagerRequestSender},
    protocols::network::{
        AppConfig, ApplicationNetworkSender, NetworkSender, NewNetworkSender, RpcError,
    },
    ProtocolId,
};
use std::{sync::Arc, time::Duration};
use storage_service_types::{
    StorageServiceError, StorageServiceMessage, StorageServiceRequest, StorageServiceResponse,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("DiemNet Rpc error: {0}")]
    RpcError(#[from] RpcError),

    #[error("Error from remote storage service: {0}")]
    StorageServiceError(#[from] StorageServiceError),
}

// TODO(philiphayes): need to expose access to somewhere to store per-peer data?
// is this the right place?
/// The interface for sending Storage Service requests and querying network peer
/// information.
#[derive(Clone, Debug)]
pub struct StorageServiceClient {
    network_sender: StorageServiceMultiSender,
    peer_metadata: Arc<PeerMetadataStorage>,
}

impl StorageServiceClient {
    pub fn new(
        network_sender: StorageServiceMultiSender,
        peer_metadata: Arc<PeerMetadataStorage>,
    ) -> Self {
        Self {
            network_sender,
            peer_metadata,
        }
    }

    pub async fn send_request(
        &self,
        recipient: PeerNetworkId,
        request: StorageServiceRequest,
        timeout: Duration,
    ) -> Result<StorageServiceResponse, Error> {
        let message = self
            .network_sender
            .send_rpc(recipient, StorageServiceMessage::Request(request), timeout)
            .await?;
        match message {
            StorageServiceMessage::Response(Ok(response)) => Ok(response),
            StorageServiceMessage::Response(Err(err)) => Err(Error::StorageServiceError(err)),
            StorageServiceMessage::Request(_) => Err(Error::RpcError(RpcError::InvalidRpcResponse)),
        }
    }
}

// TODO(philiphayes): not clear yet what value this trait is providing
#[async_trait]
impl NetworkInterface<StorageServiceMessage, StorageServiceMultiSender> for StorageServiceClient {
    // TODO(philiphayes): flesh out
    type AppDataKey = ();
    type AppData = ();

    fn peer_metadata_storage(&self) -> &PeerMetadataStorage {
        &self.peer_metadata
    }

    // TODO(philiphayes): kind of awkward? I don't actually want to expose this...
    // sending should just be a part of the interface itself no?
    fn sender(&self) -> StorageServiceMultiSender {
        todo!()
    }

    fn app_data(&self) -> &LockingHashMap<Self::AppDataKey, Self::AppData> {
        todo!()
    }
}

/// A network sender that dispatches across multiple networks.
pub type StorageServiceMultiSender =
    MultiNetworkSender<StorageServiceMessage, StorageServiceNetworkSender>;

pub fn network_endpoint_config() -> AppConfig {
    AppConfig::client([ProtocolId::StorageServiceRpc])
}

// TODO(philiphayes): this is a lot of boilerplate for what is effectively a
// NetworkSender type alias that impls ApplicationNetworkSender... maybe we just
// add ProtocolId to the APIs so we don't need this extra layer?
/// The Storage Service network sender for a single network.
#[derive(Clone, Debug)]
pub struct StorageServiceNetworkSender {
    inner: NetworkSender<StorageServiceMessage>,
}

impl NewNetworkSender for StorageServiceNetworkSender {
    fn new(
        peer_mgr_reqs_tx: PeerManagerRequestSender,
        connection_reqs_tx: ConnectionRequestSender,
    ) -> Self {
        Self {
            inner: NetworkSender::new(peer_mgr_reqs_tx, connection_reqs_tx),
        }
    }
}

#[async_trait]
impl ApplicationNetworkSender<StorageServiceMessage> for StorageServiceNetworkSender {
    fn send_to(
        &self,
        _recipient: PeerId,
        _message: StorageServiceMessage,
    ) -> Result<(), NetworkError> {
        unimplemented!()
    }

    // TODO(philiphayes): a lot of boilerplate just to add `ProtocolId::StorageServiceRpc`
    // by default.
    async fn send_rpc(
        &self,
        recipient: PeerId,
        message: StorageServiceMessage,
        timeout: Duration,
    ) -> Result<StorageServiceMessage, RpcError> {
        self.inner
            .send_rpc(recipient, ProtocolId::StorageServiceRpc, message, timeout)
            .await
    }
}
